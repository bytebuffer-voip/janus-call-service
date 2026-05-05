use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use crate::call::call_flow::call_type::message_helper;
use crate::service::janus::{audio_bridge_service, session_service};
use log::info;

pub struct A2AEndState {
    pub reason: String,
}

#[async_trait::async_trait]
impl A2ACallStateHandler for A2AEndState {
    fn get_name(&self) -> String {
        "A2AEndState".to_string()
    }

    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction> {
        info!(
            "A2PEndState enter for call {}, with reason: {}",
            call.call_id, self.reason
        );

        call.stop_timer(TimerType::JanusKeepalive).await;

        message_helper::notify_call_end(
            &call.conn_state,
            &call.call_id,
            &call.params.client_info.user_id,
            &self.reason,
        );

        message_helper::notify_call_end(
            &call.conn_state,
            &call.call_id,
            &call.params.callee_user.id,
            &self.reason,
        );

        let _ = audio_bridge_service::delete_room(
            &call.app_state,
            call.params.caller_session_id,
            call.params.caller_handle_id,
            call.params.room_id,
            call.params.secret.to_string(),
        )
        .await;

        let _ = audio_bridge_service::detach(
            &call.app_state,
            call.params.caller_session_id,
            call.params.caller_handle_id,
        )
        .await;

        if !call.callee_handle_ids.is_empty() {
            for callee_handle_id in &call.callee_handle_ids {
                let _ = audio_bridge_service::detach(
                    &call.app_state,
                    call.params.caller_session_id,
                    *callee_handle_id,
                )
                .await;
            }
        }

        let _ =
            session_service::destroy_session(&call.app_state, call.params.caller_session_id).await;

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

    async fn call_end(&mut self, call: &mut AppToAppCall) -> A2AStateAction {
        A2AStateAction::Stay
    }
}
