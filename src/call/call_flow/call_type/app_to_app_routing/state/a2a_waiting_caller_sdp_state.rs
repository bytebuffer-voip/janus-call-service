use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType, WebsocketEvent};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_end_state::A2AEndState;
use crate::service::janus::audio_bridge_service;
use crate::utils::{jsep_utils, json_utils};
use log::info;
use serde_json::Value;

pub struct A2AWaitingCallerSdpState;

impl A2AWaitingCallerSdpState {
    pub fn new() -> Self {
        A2AWaitingCallerSdpState {}
    }

    async fn handle_webrtc_sdp(
        &mut self,
        call: &mut AppToAppCall,
        sdp: String,
    ) -> anyhow::Result<()> {
        audio_bridge_service::join(
            &call.app_state,
            call.params.caller_session_id,
            call.params.caller_handle_id,
            call.params.client_info.user_id.clone(),
            call.params.room_id,
            call.params.pin.to_string(),
            call.params.secret.to_string(),
        )
        .await?;
        audio_bridge_service::configure(
            &call.app_state,
            call.params.caller_session_id,
            call.params.caller_handle_id,
            "offer".to_string(),
            sdp.clone(),
        )
        .await?;
        Ok(())
    }

    async fn process_websocket_event(
        &mut self,
        call: &mut AppToAppCall,
        evt: WebsocketEvent,
    ) -> anyhow::Result<()> {
        match evt {
            WebsocketEvent::OnSDP { sdp, client_info } => {
                self.handle_webrtc_sdp(call, sdp).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn process_janus_event(
        &mut self,
        call: &mut AppToAppCall,
        evt: Value,
    ) -> anyhow::Result<bool> {
        if evt.get("type").and_then(Value::as_i64).unwrap_or(-1) == -1 {
            return Ok(false);
        }

        let session_id = json_utils::get_int_value(&evt, "session_id");
        let handle_id = json_utils::get_int_value(&evt, "handle_id");
        if session_id != call.params.caller_session_id || handle_id != call.params.caller_handle_id
        {
            return Ok(false);
        }

        let jsep_type = jsep_utils::get_value_from_jsep(&evt, "type").unwrap_or_default();
        if jsep_type != "answer" {
            return Ok(false);
        }

        let Some(sdp) = jsep_utils::get_value_from_jsep(&evt, "sdp") else {
            return Ok(false);
        };

        let r = call
            .web_rtc_man
            .on_server_sdp(&call.app_state, &call.conn_state, handle_id, &sdp)
            .await?;

        Ok(r)
    }
}

#[async_trait::async_trait]
impl A2ACallStateHandler for A2AWaitingCallerSdpState {
    fn get_name(&self) -> String {
        "A2AWaitingCallerSdpState".to_string()
    }

    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction> {
        info!("A2AWaitingCallerSdpState.on_enter");
        call.start_timer(TimerType::WaitSDPTimeout, 45).await;
        Ok(A2AStateAction::Stay)
    }

    async fn on_exit(&mut self, call: &mut AppToAppCall) -> anyhow::Result<()> {
        info!("A2AWaitingCallerSdpState.on_exit");
        call.stop_timer(TimerType::WaitSDPTimeout).await;
        Ok(())
    }

    async fn on_event(
        &mut self,
        call: &mut AppToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<A2AStateAction> {
        match event {
            CallEvent::Websocket(evt) => {
                let _ = self.process_websocket_event(call, evt).await;
            }
            CallEvent::JanusEvent(evt) => match self.process_janus_event(call, evt).await {
                Ok(true) => {
                    // TODO
                    info!("A2AWaitingCallerSdpState.on_event true");
                }
                Ok(false) => {
                    // STAY
                    info!("A2AWaitingCallerSdpState.on_event false");
                }
                Err(err) => {
                    info!("A2AWaitingCallerSdpState.on_event: {:?}", err);
                    let end_state = A2AEndState {
                        reason: err.to_string(),
                    };
                    return Ok(A2AStateAction::Transition(Box::new(end_state)));
                }
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
        if timer == TimerType::WaitSDPTimeout {
            info!("A2AWaitingCallerSdpState.on_timer: WaitSDPTimeout");
            return Ok(A2AStateAction::Transition(Box::new(A2AEndState {
                reason: "Timeout waiting for caller SDP".to_string(),
            })));
        }
        Ok(A2AStateAction::Stay)
    }
}
