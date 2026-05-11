use crate::app_state::AppState;
use crate::call::call_flow::call_model::{CallEvent, CallTimerAction, TimerType};
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_call_state::{
    S2ACallStateHandler, S2AStateAction,
};
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_end_state::S2AEndState;
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_join_sip_member_to_room_state::S2AJoinSipMemberToRoomState;
use crate::call::call_flow::supervisor::SupervisorCommand;
use crate::model::janus_webrtc::JanusWebRTCSessionManager;
use crate::service::janus::session_service;
use crate::websocket::websocket_handler::ConnectionState;
use futures_util::future::BoxFuture;
use log::info;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct SipToAppParams {
    pub invite_request: rsip::message::Request,
    pub session_id: i64,
    pub handle_id: i64,
    pub room_id: i64,
    pub pin: String,
    pub secret: String,
}

impl SipToAppParams {
    pub fn new(
        invite_request: rsip::message::Request,
        session_id: i64,
        handle_id: i64,
        room_id: i64,
        pin: String,
        secret: String,
    ) -> Self {
        Self {
            invite_request,
            session_id,
            handle_id,
            room_id,
            pin,
            secret,
        }
    }
}

pub struct SipToAppCall {
    pub app_state: Arc<AppState>,
    pub conn_state: Arc<ConnectionState>,
    pub call_id: String,
    pub params: SipToAppParams,
    pub api_tx: Sender<SupervisorCommand>,
    pub web_rtc_man: JanusWebRTCSessionManager,
    state: Option<Box<dyn S2ACallStateHandler>>,
}

impl fmt::Debug for SipToAppCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SipToAppCall")
            .field("call_id", &self.call_id)
            .finish()
    }
}

impl SipToAppCall {
    pub fn new(
        app_state: Arc<AppState>,
        conn_state: Arc<ConnectionState>,
        call_id: String,
        params: SipToAppParams,
        api_tx: Sender<SupervisorCommand>,
    ) -> Self {
        let web_rtc_man = JanusWebRTCSessionManager::new(call_id.clone(), params.session_id);
        Self {
            app_state,
            conn_state,
            call_id,
            params,
            api_tx,
            web_rtc_man,
            state: None,
        }
    }

    async fn apply_action(&mut self, action: S2AStateAction) -> anyhow::Result<()> {
        match action {
            S2AStateAction::Stay => Ok(()),
            S2AStateAction::Transition(next) => self.transition_to(next).await,
        }
    }

    fn transition_to(
        &mut self,
        mut next: Box<dyn S2ACallStateHandler>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            loop {
                let transition_log = format!(
                    "SipToApp {} {} → {}",
                    self.call_id,
                    self.state.as_ref().map_or("<none>", |s| s.get_name()),
                    next.get_name(),
                );

                if let Some(mut prev) = self.state.take() {
                    prev.on_exit(self).await?;
                }

                info!("{}", transition_log);
                self.state = Some(next);

                if let Some(mut curr) = self.state.take() {
                    let action = curr.on_enter(self).await;
                    self.state = Some(curr);
                    match action? {
                        S2AStateAction::Stay => return Ok(()),
                        S2AStateAction::Transition(n) => {
                            next = n;
                            continue;
                        }
                    }
                } else {
                    return Ok(());
                }
            }
        })
    }

    pub async fn on_event(&mut self, event: CallEvent) {
        let state_event = event.clone();
        match event {
            CallEvent::Start => {
                info!("Call {} start", self.call_id);
                self.start_timer(TimerType::JanusKeepalive, 30).await;
                let next_state = S2AJoinSipMemberToRoomState::new();
                if let Err(e) = self.transition_to(Box::new(next_state)).await {
                    info!("{}", e);
                    let _ = self
                        .transition_to(Box::new(S2AEndState::new(
                            format!("Start call fail: {}", e),
                            false,
                            true,
                        )))
                        .await;
                    return;
                }
            }
            _ => {}
        }
        if let Some(mut state) = self.state.take() {
            let r = state.on_event(self, state_event).await;
            self.state = Some(state);
            if let Ok(state_action) = r {
                let _ = self.apply_action(state_action).await;
            }
        }
    }

    pub async fn cleanup(&mut self) {
        info!("Cleanup {}", self.call_id);
    }

    pub async fn start_timer(&self, ty: TimerType, secs: u64) {
        if let Some(tx) = self.app_state.call_supervisor.get_call_tx(&self.call_id) {
            if let Err(e) = tx
                .send(CallEvent::StartTimer(ty, Duration::from_secs(secs)))
                .await
            {
                info!(
                    "Failed to start timer {:?} for call {}, error: {}",
                    ty, self.call_id, e
                );
            }
        }
    }

    pub async fn stop_timer(&self, ty: TimerType) {
        if let Some(tx) = self.app_state.call_supervisor.get_call_tx(&self.call_id) {
            let _ = tx.send(CallEvent::StopTimer(ty)).await;
        }
    }

    pub async fn on_timer(&mut self, timer: TimerType) -> CallTimerAction {
        if timer == TimerType::JanusKeepalive {
            let _ = session_service::keepalive(&self.app_state, self.params.session_id).await;
            return CallTimerAction::Start(TimerType::JanusKeepalive, Duration::from_secs(30));
        }
        if let Some(mut state) = self.state.take() {
            let res = state.on_timer(self, timer).await;
            self.state = Some(state);
            if let Ok(state_action) = res {
                let _ = self.apply_action(state_action).await;
            }
        }
        CallTimerAction::None
    }
}
