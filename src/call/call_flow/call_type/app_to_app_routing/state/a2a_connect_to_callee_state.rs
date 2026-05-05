use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::TimerType::ResendIncomingCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType, WebsocketEvent};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_end_state::A2AEndState;
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_waiting_callee_sdp_state::A2AWaitingCalleeSDPState;
use crate::controller::user_controller::MeResponse;
use log::info;
use serde_json::{Value, json};

pub struct A2AConnectToCalleeState;

impl A2AConnectToCalleeState {
    pub fn new() -> Self {
        A2AConnectToCalleeState {}
    }
}

async fn send_incoming_call_with_resend(call: &AppToAppCall, start_resend: bool, resend_secs: u64) {
    let callee_user = &call.params.callee_user;
    let mut params = serde_json::Map::new();
    params.insert(
        "call_id".to_string(),
        Value::String(call.call_id.to_string()),
    );
    params.insert(
        "call_from".to_string(),
        Value::String(call.params.caller.to_string()),
    );

    let user_caller = MeResponse::from_user(&call.params.caller_user);
    if let Ok(user_caller) = serde_json::to_value(user_caller) {
        params.insert("call_from_user".to_string(), user_caller);
    }

    let val = json!({
        "cmd": "incoming_call",
        "params": params
    });

    info!("Sending incoming call to agent: {}", val.to_string());
    call.conn_state
        .send_to_user(&callee_user.id, val.to_string());

    if start_resend {
        call.start_timer(ResendIncomingCall, resend_secs).await;
    }
}

#[async_trait::async_trait]
impl A2ACallStateHandler for A2AConnectToCalleeState {
    fn get_name(&self) -> String {
        "A2AConnectToCalleeState".to_string()
    }

    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction> {
        info!("Entering state: {}", self.get_name());
        send_incoming_call_with_resend(call, true, 5).await;
        call.start_timer(TimerType::WaitSDPTimeout, 45).await;
        Ok(A2AStateAction::Stay)
    }

    async fn on_exit(&mut self, call: &mut AppToAppCall) -> anyhow::Result<()> {
        call.stop_timer(ResendIncomingCall).await;
        call.stop_timer(TimerType::WaitSDPTimeout).await;
        Ok(())
    }

    async fn on_event(
        &mut self,
        call: &mut AppToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<A2AStateAction> {
        match event {
            CallEvent::Websocket(evt) => match evt {
                WebsocketEvent::OnAnswer {
                    code,
                    client_info,
                    sdp,
                } => {
                    call.stop_timer(ResendIncomingCall).await;
                    info!("A2AConnectToCalleeState.OnAnswer {}, sdp: {}", code, sdp);
                    match code {
                        180 => {
                            let ringing = json!({
                                "cmd": "call_ringing",
                                "params": {
                                    "call_id": call.call_id.to_string(),
                                    "status": "ringing"
                                }
                            });
                            call.conn_state.send_to_user(
                                &call.params.client_info.user_id,
                                ringing.to_string(),
                            );
                        }
                        486 => {
                            let end_state = A2AEndState {
                                reason: "Callee Busy".to_string(),
                            };
                            return Ok(A2AStateAction::Transition(Box::new(end_state)));
                        }
                        200 => {
                            let next_state = A2AWaitingCalleeSDPState::new(
                                sdp.clone(),
                                client_info.client_id.clone(),
                            );
                            return Ok(A2AStateAction::Transition(Box::new(next_state)));
                        }
                        v => {
                            info!("Received unexpected answer code: {}", v);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(A2AStateAction::Stay)
    }

    async fn on_timer(
        &mut self,
        call: &mut AppToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<A2AStateAction> {
        if ResendIncomingCall == timer {
            send_incoming_call_with_resend(call, true, 5).await;
            return Ok(A2AStateAction::Stay);
        }
        if TimerType::WaitSDPTimeout == timer {
            info!("A2AConnectToCalleeState.on_timer");
            let end_state = A2AEndState {
                reason: "Connect to callee timeout".to_string(),
            };
            return Ok(A2AStateAction::Transition(Box::new(end_state)));
        }
        Ok(A2AStateAction::Stay)
    }
}
