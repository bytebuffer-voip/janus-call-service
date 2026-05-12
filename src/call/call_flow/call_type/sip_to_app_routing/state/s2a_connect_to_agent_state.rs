use crate::call::call_flow::call_model::{CallEvent, TimerType};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_waiting_callee_sdp_state::HandleClientMap;
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_call_state::{
    S2ACallStateHandler, S2AStateAction,
};
use crate::call::sip_to_app_call::SipToAppCall;
use log::info;

const STATE_NAME: &str = "S2AConnectToAgentState";

pub struct S2AConnectToAgentState {
    resend_count: u32,
    handle_client: HandleClientMap,
}

impl S2AConnectToAgentState {
    pub fn new() -> Self {
        Self {
            resend_count: 0,
            handle_client: HandleClientMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl S2ACallStateHandler for S2AConnectToAgentState {
    fn get_name(&self) -> &str {
        STATE_NAME
    }

    async fn on_enter(&mut self, call: &mut SipToAppCall) -> anyhow::Result<S2AStateAction> {
        info!("Entering state: {}", self.get_name());
        Ok(S2AStateAction::Stay)
    }

    async fn on_exit(&mut self, call: &mut SipToAppCall) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_event(
        &mut self,
        call: &mut SipToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<S2AStateAction> {
        Ok(S2AStateAction::Stay)
    }

    async fn on_timer(
        &mut self,
        call: &mut SipToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<S2AStateAction> {
        Ok(S2AStateAction::Stay)
    }
}
