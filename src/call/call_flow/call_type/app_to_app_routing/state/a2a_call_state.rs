use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType};
use uuid::Uuid;

pub enum A2AStateAction {
    Stay,
    Transition(Box<dyn A2ACallStateHandler>),
    Hangup { reason: String },
}

impl A2AStateAction {
    pub fn get_name(&self) -> String {
        match self {
            A2AStateAction::Stay => "Stay".to_string(),
            A2AStateAction::Transition(a) => format!("Transition to {}", a.get_name()),
            A2AStateAction::Hangup { reason } => format!("Hangup {}", reason),
        }
    }
}

#[async_trait::async_trait]
pub trait A2ACallStateHandler: Send + Sync {
    fn get_name(&self) -> String;
    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction>;
    async fn on_exit(&mut self, call: &mut AppToAppCall) -> anyhow::Result<()>;

    async fn on_event(
        &mut self,
        call: &mut AppToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<A2AStateAction>;

    async fn on_timer(
        &mut self,
        call: &mut AppToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<A2AStateAction> {
        Ok(A2AStateAction::Stay)
    }

    fn can_hangup(&mut self, call: &mut AppToAppCall, uuid: &str) -> bool {
        false
    }

    fn check_is_agent_client(&mut self, call: &mut AppToAppCall, client_id: Uuid) -> bool {
        false
    }

    async fn call_end(&mut self, call: &mut AppToAppCall) -> A2AStateAction {
        A2AStateAction::Stay
    }
}
