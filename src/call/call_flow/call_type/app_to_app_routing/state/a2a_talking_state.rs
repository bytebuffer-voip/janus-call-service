use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use log::info;

pub struct A2ATalkingState;
impl A2ATalkingState {
    pub fn new() -> Self {
        A2ATalkingState
    }
}

#[async_trait::async_trait]
impl A2ACallStateHandler for A2ATalkingState {
    fn get_name(&self) -> String {
        "A2ATalkingState".to_string()
    }

    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction> {
        info!("A2ATalkingState enter");
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

    async fn on_timer(
        &mut self,
        call: &mut AppToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<A2AStateAction> {
        Ok(A2AStateAction::Stay)
    }
}
