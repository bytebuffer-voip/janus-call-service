use crate::call::app_to_app_call::AppToAppCall;
use crate::call::call_flow::call_model::{CallEvent, TimerType, WebsocketEvent};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_call_state::{
    A2ACallStateHandler, A2AStateAction,
};
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_end_state::A2AEndState;
use crate::call::call_flow::call_type::app_to_app_routing::state::a2a_talking_state::A2ATalkingState;
use crate::service::janus::{audio_bridge_service, session_service};
use crate::utils::{jsep_utils, json_utils};
use log::info;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub struct HandleClientMap {
    handle_to_client: HashMap<i64, Uuid>,
    client_to_handle: HashMap<Uuid, i64>,
}

impl HandleClientMap {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn get_client(&self, handle_id: i64) -> Option<&Uuid> {
        self.handle_to_client.get(&handle_id)
    }

    #[inline]
    pub fn get_handle(&self, client_id: &Uuid) -> Option<i64> {
        self.client_to_handle.get(client_id).copied()
    }

    #[inline]
    pub fn insert(&mut self, handle_id: i64, client_id: Uuid) {
        self.handle_to_client.insert(handle_id, client_id);
        self.client_to_handle.insert(client_id, handle_id);
    }

    #[inline]
    pub fn iter_handles_except<'a>(&'a self, exclude: &'a Uuid) -> impl Iterator<Item = i64> + 'a {
        self.client_to_handle
            .iter()
            .filter(move |(id, _)| *id != exclude)
            .map(|(_, &handle)| handle)
    }
}

pub struct A2AWaitingCalleeSDPState {
    handle_client: HandleClientMap,
    sdp: String,
    client_id: Uuid,
}

impl A2AWaitingCalleeSDPState {
    pub fn new(sdp: String, client_id: Uuid) -> A2AWaitingCalleeSDPState {
        A2AWaitingCalleeSDPState {
            handle_client: HandleClientMap::new(),
            sdp,
            client_id,
        }
    }

    async fn get_or_create_handle(
        &mut self,
        call: &mut AppToAppCall,
        client_id: Uuid,
    ) -> Option<i64> {
        if let Some(handle_id) = self.handle_client.get_handle(&client_id) {
            return Some(handle_id);
        }

        let handle_id =
            audio_bridge_service::attach(&call.app_state, call.params.caller_session_id)
                .await
                .ok()?;

        let janus_key = format!("janus_{}_{}", call.params.caller_session_id, handle_id);
        call.app_state
            .call_supervisor
            .add_janus_handle(&call.call_id, &janus_key);

        call.web_rtc_man.add_client_handle(client_id, handle_id);
        call.callee_handle_ids.push(handle_id);

        self.handle_client.insert(handle_id, client_id);
        Some(handle_id)
    }

    async fn handle_join_and_configure(
        &mut self,
        call: &mut AppToAppCall,
        handle_id: i64,
        user_id: String,
        sdp: String,
    ) -> anyhow::Result<()> {
        audio_bridge_service::join(
            &call.app_state,
            call.params.caller_session_id,
            handle_id,
            user_id,
            call.params.room_id,
            call.params.pin.to_string(),
            call.params.secret.to_string(),
        )
        .await?;
        audio_bridge_service::configure(
            &call.app_state,
            call.params.caller_session_id,
            handle_id,
            "offer".to_string(),
            sdp,
        )
        .await?;
        Ok(())
    }

    async fn on_waiting_sdp_webrtc(
        &mut self,
        call: &mut AppToAppCall,
        evt: WebsocketEvent,
    ) -> anyhow::Result<bool> {
        match evt {
            WebsocketEvent::OnSDP {
                client_info, sdp, ..
            } => {
                let Some(handle_id) = self.get_or_create_handle(call, client_info.client_id).await
                else {
                    return Ok(false);
                };
                self.handle_join_and_configure(
                    call,
                    handle_id,
                    client_info.user_id.clone(),
                    sdp.clone(),
                )
                .await?;

                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn notify_other_devices_answered(&self, call: &mut AppToAppCall, exclude_client_id: &Uuid) {
        let payload = json!({
            "cmd": "answered_on_other_device",
            "params": { "call_id": call.call_id }
        })
        .to_string();
        info!(
            "call_id: {} Sending answered_on_other_device excluding client_id: {}",
            call.call_id, exclude_client_id
        );
        call.conn_state.send_to_user_except_client_id(
            &call.params.callee_user.id,
            exclude_client_id,
            payload,
        );
    }

    async fn detach_other_handles(&self, call: &mut AppToAppCall, exclude_client_id: &Uuid) {
        // for handle_id in self.handle_client.iter_handles_except(exclude_client_id) {
        //     call.callee_handle_ids.remove(&handle_id);
        //     if let Err(e) = audio_bridge_service::detach(
        //         &call.app_state,
        //         call.params.caller_session_id,
        //         handle_id,
        //     )
        //     .await
        //     {
        //         info!("Failed to detach audio bridge service: {}", e);
        //     }
        // }
    }

    async fn handle_sdp_answer_received(
        &mut self,
        call: &mut AppToAppCall,
        client_id: &Uuid,
        handle_id: i64,
        sdp: String,
    ) -> anyhow::Result<bool> {
        let r = call
            .web_rtc_man
            .on_server_sdp(&call.app_state, &call.conn_state, handle_id, &sdp)
            .await?;
        call.callee_client_uuid = Some(client_id.clone());
        // Notify other devices and detach their handles
        self.notify_other_devices_answered(call, client_id);
        self.detach_other_handles(call, client_id).await;
        Ok(true)
    }

    async fn on_janus_event(
        &mut self,
        call: &mut AppToAppCall,
        evt: Value,
    ) -> anyhow::Result<bool> {
        info!("A2AWaitingCalleeSDPState.on_janus_event: {}", evt);
        if evt.get("type").and_then(Value::as_i64).unwrap_or(-1) == -1 {
            return Ok(false);
        }
        let session_id = json_utils::get_int_value(&evt, "session_id");
        if session_id != call.params.caller_session_id {
            return Ok(false);
        }
        let handle_id = json_utils::get_int_value(&evt, "handle_id");
        let Some(client_id) = self.handle_client.get_client(handle_id).cloned() else {
            return Ok(false);
        };
        let jsep_type = jsep_utils::get_value_from_jsep(&evt, "type").unwrap_or_default();
        if jsep_type != "answer" {
            return Ok(false);
        }
        let Some(jsep_sdp) = jsep_utils::get_value_from_jsep(&evt, "sdp") else {
            return Ok(false);
        };
        self.handle_sdp_answer_received(call, &client_id, handle_id, jsep_sdp)
            .await
    }
}

#[async_trait::async_trait]
impl A2ACallStateHandler for A2AWaitingCalleeSDPState {
    fn get_name(&self) -> String {
        "A2AWaitingCalleeSDPState".to_string()
    }

    async fn on_enter(&mut self, call: &mut AppToAppCall) -> anyhow::Result<A2AStateAction> {
        info!("A2AWaitingCalleeSDPState.on_enter");
        let Some(handle_id) = self.get_or_create_handle(call, self.client_id).await else {
            let end_state = A2AEndState {
                reason: "Failed to create Janus handle".to_string(),
            };
            return Ok(A2AStateAction::Transition(Box::new(end_state)));
        };

        if let Err(e) = self
            .handle_join_and_configure(
                call,
                handle_id,
                call.params.callee_user.id.clone(),
                self.sdp.clone(),
            )
            .await
        {
            let end_state = A2AEndState {
                reason: format!("Failed to configure audio bridge: {}", e),
            };
            return Ok(A2AStateAction::Transition(Box::new(end_state)));
        }

        call.start_timer(TimerType::WaitSDPTimeout, 45).await;
        Ok(A2AStateAction::Stay)
    }

    async fn on_exit(&mut self, call: &mut AppToAppCall) -> anyhow::Result<()> {
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
                if let Err(e) = self.on_waiting_sdp_webrtc(call, evt).await {
                    info!("A2AWaitingCalleeSDPState.on_waiting_sdp_webrtc: {:?}", e);
                }
            }
            CallEvent::JanusEvent(evt) => {
                return match self.on_janus_event(call, evt).await {
                    Ok(true) => Ok(A2AStateAction::Transition(Box::new(A2ATalkingState::new()))),
                    Ok(false) => Ok(A2AStateAction::Stay),
                    Err(e) => {
                        info!("A2AWaitingCalleeSDPState.on_janus_event: {:?}", e);
                        let end_state = A2AEndState {
                            reason: format!("Error processing Janus event: {}", e),
                        };
                        Ok(A2AStateAction::Transition(Box::new(end_state)))
                    }
                };
            }
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
            info!("A2AWaitingCalleeSDPState.on_timer WaitSDPTimeout");
            let end_state = A2AEndState {
                reason: "Wait timeout".to_string(),
            };
            return Ok(A2AStateAction::Transition(Box::new(end_state)));
        }
        Ok(A2AStateAction::Stay)
    }
}
