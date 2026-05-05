use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::CallEvent;
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use log::info;
use uuid::Uuid;

pub struct A2AWaitingCalleeSDPState {
    sdp: String,
    client_id: Uuid,
}

impl A2AWaitingCalleeSDPState {
    pub fn new(sdp: String, client_id: Uuid) -> A2AWaitingCalleeSDPState {
        A2AWaitingCalleeSDPState { sdp, client_id }
    }
}

#[async_trait::async_trait]
impl A2ACallStateHandler for A2AWaitingCalleeSDPState {
    fn get_name(&self) -> String {
        "A2AWaitingCalleeSDPState".to_string()
    }

    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction> {
        info!("A2AWaitingCalleeSDPState.on_enter");
        Ok(A2AStateAction::Stay)
    }

    async fn on_exit(&mut self, call: &mut AppToAppCall) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_event(
        &mut self,
        call: &mut AppToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<A2AStateAction> {
        Ok(A2AStateAction::Stay)
    }
}
