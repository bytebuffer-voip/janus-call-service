use crate::call::call_flow::call_model::{CallEvent, TimerType};
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_end_state::S2AEndState;
use crate::call::sip_to_app_call::SipToAppCall;
use crate::websocket::websocket_handler::ClientInfo;
use uuid::Uuid;

pub enum S2AStateAction {
    Stay,
    Transition(Box<dyn S2ACallStateHandler>),
}

#[async_trait::async_trait]
pub trait S2ACallStateHandler: Send + Sync {
    fn get_name(&self) -> &str;

    async fn on_enter(&mut self, call: &mut SipToAppCall) -> anyhow::Result<S2AStateAction>;
    async fn on_exit(&mut self, call: &mut SipToAppCall) -> anyhow::Result<()>;

    async fn on_event(
        &mut self,
        call: &mut SipToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<S2AStateAction>;

    async fn on_timer(
        &mut self,
        call: &mut SipToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<S2AStateAction> {
        let reason = match timer {
            TimerType::WaitSDPTimeout => "Timeout SDP",
            _ => "",
        };
        let end_state = S2AEndState::new(reason.to_string(), true, true);
        Ok(S2AStateAction::Transition(Box::new(end_state)))
    }

    fn can_hangup(&mut self, _call: &mut SipToAppCall, _uuid: &str) -> bool {
        false
    }

    fn check_is_agent_client(&mut self, call: &mut SipToAppCall, client_id: Uuid) -> bool {
        // TODO: Implement when agent_client_id is available
        false
    }

    async fn call_end(&mut self, _call: &mut SipToAppCall) -> S2AStateAction {
        let reason = "Call ended".to_string();
        let end_state = S2AEndState::new(reason.clone(), true, false);
        S2AStateAction::Transition(Box::new(end_state))
    }

    async fn kill_leg_if_exists(&mut self, _call: &mut SipToAppCall, _client_info: ClientInfo) {
        // Default implementation does nothing
    }
}
