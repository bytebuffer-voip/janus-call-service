use crate::call::call_flow::call_model::{CallEvent, TimerType};
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_call_state::{
    S2ACallStateHandler, S2AStateAction,
};
use crate::call::sip_to_app_call::SipToAppCall;
use crate::websocket::websocket_handler::ClientInfo;
use uuid::Uuid;

pub struct S2AEndState {
    pub reason: String,
    pub need_send_bye: bool,
    pub need_send_busy: bool,
}

impl S2AEndState {
    pub fn new(reason: String, need_send_bye: bool, need_send_busy: bool) -> S2AEndState {
        Self {
            reason,
            need_send_bye,
            need_send_busy,
        }
    }
}

#[async_trait::async_trait]
impl S2ACallStateHandler for S2AEndState {
    fn get_name(&self) -> &str {
        "S2AEndState"
    }

    async fn on_enter(&mut self, call: &mut SipToAppCall) -> anyhow::Result<S2AStateAction> {
        todo!()
    }

    async fn on_exit(&mut self, call: &mut SipToAppCall) -> anyhow::Result<()> {
        todo!()
    }

    async fn on_event(
        &mut self,
        call: &mut SipToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<S2AStateAction> {
        todo!()
    }

    async fn on_timer(
        &mut self,
        call: &mut SipToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<S2AStateAction> {
        todo!()
    }

    fn can_hangup(&mut self, _call: &mut SipToAppCall, _uuid: &str) -> bool {
        todo!()
    }

    fn check_is_agent_client(&mut self, call: &mut SipToAppCall, client_id: Uuid) -> bool {
        todo!()
    }

    async fn call_end(&mut self, _call: &mut SipToAppCall) -> S2AStateAction {
        todo!()
    }

    async fn kill_leg_if_exists(&mut self, _call: &mut SipToAppCall, _client_info: ClientInfo) {
        todo!()
    }
}
