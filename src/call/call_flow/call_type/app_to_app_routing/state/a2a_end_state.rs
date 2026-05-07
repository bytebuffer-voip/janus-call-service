use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use uuid::Uuid;

pub struct A2AEndState {
    pub reason: String,
}

#[async_trait::async_trait]
impl A2ACallStateHandler for A2AEndState {
    fn get_name(&self) -> String {
        "A2AEndState".to_string()
    }

    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction> {
        todo!()
    }

    async fn on_exit(&mut self, call: &mut AppToAppCall) -> anyhow::Result<()> {
        todo!()
    }

    async fn on_event(
        &mut self,
        call: &mut AppToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<A2AStateAction> {
        todo!()
    }

    async fn on_timer(
        &mut self,
        call: &mut AppToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<A2AStateAction> {
        todo!()
    }

    fn can_hangup(&mut self, call: &mut AppToAppCall, uuid: &str) -> bool {
        todo!()
    }

    fn check_is_agent_client(&mut self, call: &mut AppToAppCall, client_id: Uuid) -> bool {
        todo!()
    }

    async fn call_end(&mut self, call: &mut AppToAppCall) -> A2AStateAction {
        todo!()
    }
}
