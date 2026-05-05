use crate::app_state::AppState;
use crate::call::call_flow::call_model::{CallEvent, CallTimerAction, TimerType, WebsocketEvent};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_end_state::A2AEndState;
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_waiting_caller_sdp_state::A2AWaitingCallerSdpState;
use crate::call::call_flow::supervisor::SupervisorCommand;
use crate::model::janus_webrtc::JanusWebRTCSessionManager;
use crate::model::user::User;
use crate::service::janus::session_service;
use crate::utils::json_utils;
use crate::websocket::websocket_handler::{ClientInfo, ConnectionState};
use futures_util::future::BoxFuture;
use log::info;
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct A2ACallInitParams {
    pub client_info: ClientInfo,
    pub caller: String,
    pub callee: String,
    pub caller_user: User,
    pub callee_user: User,
    pub caller_session_id: i64,
    pub caller_handle_id: i64,
    pub room_id: i64,
    pub pin: String,
    pub secret: String,
}

impl A2ACallInitParams {
    pub fn new(
        client_info: ClientInfo,
        caller: String,
        callee: String,
        caller_user: User,
        callee_user: User,
        caller_session_id: i64,
        caller_handle_id: i64,
        room_id: i64,
        pin: String,
        secret: String,
    ) -> Self {
        Self {
            client_info,
            caller,
            callee,
            caller_user,
            callee_user,
            caller_session_id,
            caller_handle_id,
            room_id,
            pin,
            secret,
        }
    }
}

pub struct AppToAppCall {
    pub app_state: Arc<AppState>,
    pub conn_state: Arc<ConnectionState>,
    pub call_id: String,
    pub params: A2ACallInitParams,
    pub api_tx: Sender<SupervisorCommand>,
    pub callee_handle_ids: Vec<i64>,
    pub callee_client_uuid: Option<Uuid>,
    state: Option<Box<dyn A2ACallStateHandler>>,
    pub web_rtc_man: JanusWebRTCSessionManager,
}

impl fmt::Debug for AppToAppCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppToAppCall").finish()
    }
}

impl AppToAppCall {
    pub fn new(
        app_state: Arc<AppState>,
        conn_state: Arc<ConnectionState>,
        call_id: String,
        params: A2ACallInitParams,
        api_tx: Sender<SupervisorCommand>,
    ) -> Self {
        let mut web_rtc_man =
            JanusWebRTCSessionManager::new(call_id.clone(), params.caller_session_id);
        web_rtc_man.add_client_handle(
            params.client_info.client_id.clone(),
            params.caller_handle_id,
        );

        Self {
            app_state,
            conn_state,
            call_id,
            params,
            api_tx,
            callee_handle_ids: Vec::new(),
            callee_client_uuid: None,
            state: None,
            web_rtc_man,
        }
    }

    async fn apply_action(&mut self, action: A2AStateAction) -> anyhow::Result<()> {
        match action {
            A2AStateAction::Stay => Ok(()),
            A2AStateAction::Transition(next) => self.transition_to(next).await,
            A2AStateAction::Hangup { reason } => {
                self.transition_to(Box::new(A2AEndState { reason })).await
            }
        }
    }

    fn transition_to(
        &mut self,
        mut next: Box<dyn A2ACallStateHandler>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            loop {
                let prev_name = self
                    .state
                    .as_ref()
                    .map(|s| s.get_name().to_string())
                    .unwrap_or("<none>".to_string());
                if let Some(mut prev) = self.state.take() {
                    prev.on_exit(self).await?;
                }
                let next_name = next.get_name().to_string();
                self.state = Some(next);
                info!("Call {} {} → {}", self.call_id, prev_name, next_name);
                if let Some(mut curr) = self.state.take() {
                    let action = curr.on_enter(self).await;
                    self.state = Some(curr);
                    match action? {
                        A2AStateAction::Stay => return Ok(()),
                        A2AStateAction::Transition(n) => {
                            next = n;
                            continue;
                        }
                        A2AStateAction::Hangup { reason } => {
                            next = Box::new(A2AEndState { reason });
                            continue;
                        }
                    }
                } else {
                    return Ok(());
                }
            }
        })
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

    pub async fn on_event(&mut self, event: CallEvent) {
        let state_event = event.clone();
        match event {
            CallEvent::Start => {
                self.start_timer(TimerType::JanusKeepalive, 30).await;
                self.transition_to(Box::new(A2AWaitingCallerSdpState::new()))
                    .await
                    .unwrap_or_else(|e| {
                        info!(
                            "Error transitioning to A2AWaitSDPState for call {}, error: {}",
                            self.call_id, e
                        );
                    });
            }
            CallEvent::Websocket(e) => {
                let _ = self.process_websocket_event(e).await;
            }
            CallEvent::JanusEvent(e) => {
                let _ = self.process_janus_event(e).await;
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

    async fn process_janus_event(&mut self, evt: Value) -> anyhow::Result<()> {
        if evt.get("type").and_then(Value::as_i64).unwrap_or(-1) == -1 {
            return Ok(());
        }
        let session_id = json_utils::get_int_value(&evt, "session_id");
        let handle_id = json_utils::get_int_value(&evt, "handle_id");
        if let Some(event) = evt.get("event") {
            if let Some(local_candidate) = event.get("local-candidate").and_then(|v| v.as_str()) {
                self.web_rtc_man
                    .on_server_candidate(
                        &self.app_state,
                        &self.conn_state,
                        handle_id,
                        local_candidate,
                    )
                    .await?;
                return Ok(());
            }
        }
        Ok(())
    }

    async fn process_websocket_event(&mut self, evt: WebsocketEvent) -> anyhow::Result<()> {
        match evt {
            WebsocketEvent::EndCall(info) => {
                if self.check_is_agent_client(info.client_id) {
                    let end_state = A2AEndState {
                        reason: "User hangup".into(),
                    };
                    self.apply_action(A2AStateAction::Transition(Box::new(end_state)))
                        .await?;
                }
            }
            WebsocketEvent::OnICECandidate {
                candidate,
                client_info,
                sdp_mid,
                sdp_mline_index,
            } => {
                self.web_rtc_man
                    .on_client_candidate(
                        &self.app_state,
                        &self.conn_state,
                        &client_info.client_id,
                        &candidate,
                        sdp_mline_index,
                        sdp_mid,
                    )
                    .await?;
            }
            WebsocketEvent::OnICECandidateCompleted { client_info } => {
                self.web_rtc_man
                    .on_client_candidate_completed(
                        &self.app_state,
                        &self.conn_state,
                        &client_info.client_id,
                    )
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    fn check_is_agent_client(&mut self, client_id: Uuid) -> bool {
        if let Some(mut state) = self.state.take() {
            let res = state.check_is_agent_client(self, client_id);
            self.state = Some(state);
            res
        } else {
            false
        }
    }

    pub async fn on_timer(&mut self, timer: TimerType) -> CallTimerAction {
        if timer == TimerType::JanusKeepalive {
            let _ =
                session_service::keepalive(&self.app_state, self.params.caller_session_id).await;
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

    pub async fn cleanup(&mut self) {
        if let Some(mut state) = self.state.take() {
            let state_action = state.call_end(self).await;
            self.state = Some(state);
            let _ = self.apply_action(state_action).await;
        }
    }
}
