use crate::app_state::AppState;
use crate::call::call_flow::call_model::{CallEvent, CallTimerAction, TimerType};
use crate::call::call_flow::supervisor::SupervisorCommand;
use crate::model::janus_webrtc::JanusWebRTCSessionManager;
use crate::websocket::websocket_handler::ConnectionState;
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_call_state::S2ACallStateHandler;

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

    pub async fn on_event(&mut self, event: CallEvent) {}

    pub async fn cleanup(&mut self) {}

    pub async fn on_timer(&mut self, timer: TimerType) -> CallTimerAction {
        CallTimerAction::None
    }
    
}
