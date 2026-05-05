use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use log::info;
use serde_json::json;
use uuid::Uuid;

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
        let msg = json!({
            "cmd": "answered_ntf",
            "params": { "call_id": call.call_id }
        });
        call.conn_state
            .send_to_user(&call.params.callee_user.id, msg.to_string());
        call.conn_state
            .send_to_user(&call.params.client_info.user_id, msg.to_string());
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

    fn check_is_agent_client(&mut self, call: &mut AppToAppCall, client_id: Uuid) -> bool {
        if client_id == call.params.client_info.client_id {
            return true;
        }
        if let Some(callee_client_uuid) = call.callee_client_uuid {
            if client_id == callee_client_uuid {
                return true;
            }
        }
        false
    }

}
