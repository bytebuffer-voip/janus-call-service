use crate::call::call_flow::call_model::{CallEvent, SipEvent, TimerType, WebsocketEvent};
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_call_state::{
    S2ACallStateHandler, S2AStateAction,
};
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_end_state::S2AEndState;
use crate::call::sip_to_app_call::SipToAppCall;
use crate::network::sip_transport::send_sip_response;
use crate::service::janus::audio_bridge_service;
use crate::utils::sdp_util::select_codec;
use crate::utils::{sdp_util, sip_utils};
use log::info;
use rsip::{Method, SipMessage};
use serde_json::Value;
use uuid::Uuid;
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_connect_to_agent_state::S2AConnectToAgentState;

const STATE_NAME: &str = "S2AJoinSipMemberToRoomState";
pub struct S2AJoinSipMemberToRoomState {
    pub sdp_answer: Option<String>,
}

impl S2AJoinSipMemberToRoomState {
    pub fn new() -> Self {
        Self { sdp_answer: None }
    }

    async fn on_janus_event(
        &mut self,
        call: &mut SipToAppCall,
        evt: &Value,
    ) -> anyhow::Result<bool> {
        let event_type = evt.get("type").and_then(|e| e.as_i64()).unwrap_or(-1);
        if event_type == -1 {
            return Ok(false);
        }
        let session_id = evt
            .get("session_id")
            .and_then(|s| s.as_i64())
            .unwrap_or_default();
        let handle_id = evt
            .get("handle_id")
            .and_then(|h| h.as_i64())
            .unwrap_or_default();

        let event_data = evt.get("event").and_then(|e| e.get("data"));

        let event_str = event_data
            .and_then(|d| d.get("event"))
            .and_then(|e| e.as_str())
            .unwrap_or_default();

        if event_str == "joined" {
            let handle_info = match audio_bridge_service::get_handle_info(
                &call.app_state,
                call.params.session_id,
                call.params.handle_id,
            )
            .await
            {
                Ok(info) => info,
                Err(e) => {
                    info!("{}: Failed to get handle info: {}", STATE_NAME, e);
                    return Ok(false);
                }
            };
            if let Some(plain_rtp) = handle_info
                .get("info")
                .and_then(|info| info.get("plugin_specific"))
                .and_then(|s| s.get("plain-rtp"))
            {
                info!("{}: Joined with plain RTP: {}", STATE_NAME, plain_rtp);
                let local_port = plain_rtp
                    .get("local-port")
                    .and_then(|p| p.as_i64())
                    .unwrap_or(-1);
                let local_ip = plain_rtp
                    .get("local-ip")
                    .and_then(|p| p.as_str())
                    .unwrap_or_default();
                info!(
                    "{}: Local RTP - IP: {}, Port: {}",
                    STATE_NAME, local_ip, local_port
                );
                if local_port != -1 {
                    let offer_sdp = String::from_utf8_lossy(&call.params.invite_request.body);
                    let Some(codec) = select_codec(&offer_sdp) else {
                        info!("{}: No supported codec in offer SDP", STATE_NAME);
                        return Ok(false);
                    };
                    self.sdp_answer = Some(sdp_util::build_sdp_answer(
                        &local_ip,
                        local_port as u16,
                        &codec,
                    ));
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[async_trait::async_trait]
impl S2ACallStateHandler for S2AJoinSipMemberToRoomState {
    fn get_name(&self) -> &str {
        STATE_NAME
    }

    async fn on_enter(&mut self, call: &mut SipToAppCall) -> anyhow::Result<S2AStateAction> {
        let tran_id = sip_utils::get_pending_transaction_id(&SipMessage::Request(
            call.params.invite_request.clone(),
        ));

        // TODO: tran_id
        if let Some(tran_id) = tran_id {
            call.app_state
                .call_supervisor
                .add_sip_pending_tran(&call.call_id, &tran_id);
        }

        // SDP
        if !call.params.invite_request.body.is_empty() {
            let sdp = String::from_utf8_lossy(&call.params.invite_request.body);
            info!("{}: SDP offer from carrier:\n{}", STATE_NAME, sdp);
            let Some((ip, port)) = sdp_util::parse_sdp_ip_port(&sdp) else {
                return Ok(S2AStateAction::Transition(Box::new(S2AEndState::new(
                    "Failed to parse SDP offer".to_string(),
                    false,
                    true,
                ))));
            };
            info!("{}: SDP offer ip: {}, port: {}", STATE_NAME, ip, port);

            let name = format!("SIP participant {}", Uuid::new_v4());
            let codec = select_codec(&sdp);

            if let Err(e) = audio_bridge_service::join_with_rtp(
                &call.app_state,
                call.params.session_id,
                call.params.handle_id,
                name,
                call.params.room_id,
                call.params.pin.to_string(),
                ip,
                port,
                codec,
                call.params.secret.to_string(),
            )
            .await
            {
                info!("{}: Failed to join room: {}", STATE_NAME, e);
                return Ok(S2AStateAction::Transition(Box::new(S2AEndState::new(
                    "Failed to join audio bridge room".to_string(),
                    false,
                    true,
                ))));
            }
        }

        call.start_timer(TimerType::WaitSDPTimeout, 60).await;
        Ok(S2AStateAction::Stay)
    }

    async fn on_exit(&mut self, call: &mut SipToAppCall) -> anyhow::Result<()> {
        call.stop_timer(TimerType::WaitSDPTimeout).await;
        Ok(())
    }

    async fn on_event(
        &mut self,
        call: &mut SipToAppCall,
        event: CallEvent,
    ) -> anyhow::Result<S2AStateAction> {
        match event {
            CallEvent::Websocket(WebsocketEvent::EndCall(_)) => {
                info!(
                    "{}: Received EndCall event, transitioning to end state",
                    STATE_NAME
                );
                let end_state = S2AEndState::new("Remote party hung up".to_string(), false, true);
                return Ok(S2AStateAction::Transition(Box::new(end_state)));
            }
            CallEvent::SIP(SipEvent::SIPRequest { req, .. }) if req.method == Method::Ack => {
                info!("{}: Received SIP ack", STATE_NAME);
            }
            CallEvent::JanusEvent(evt) => {
                info!("{}: JanusEvent: {:?}", STATE_NAME, evt.to_string());
                if self.on_janus_event(call, &evt).await? {
                    let complete_sdp = self.sdp_answer.clone().unwrap_or_default();
                    let to_tag = rsip::common::uri::param::tag::Tag::default();
                    call.to_tag = Some(to_tag.clone());
                    call.sip_answer_sdp = Some(complete_sdp.clone());

                    let sdp_early = sdp_util::sdp_set_direction(&complete_sdp, "sendonly");
                    info!("{}: Sending 183 OK with SDP:\n{}", STATE_NAME, sdp_early);

                    let ok_resp = sip_utils::build_response_183_with_sdp(
                        &call.app_state.config,
                        &call.params.invite_request,
                        sdp_early,
                        to_tag,
                    );

                    if call.ok_resp.is_none() {
                        call.ok_resp = Some(ok_resp.clone());
                    }

                    if let Err(e) = send_sip_response(&call.app_state, &ok_resp).await {
                        info!("{}: Failed to send SIP OK response {}", STATE_NAME, e);
                        let end_state =
                            S2AEndState::new("Remote party hung up".to_string(), false, true);
                        return Ok(S2AStateAction::Transition(Box::new(end_state)));
                    }

                    let sip_msg = SipMessage::Response(ok_resp);
                    if let Ok(dialog_id) = sip_utils::get_dialog_id(&sip_msg, false) {
                        info!("{}: dialog_id: {}", STATE_NAME, dialog_id);
                        call.app_state
                            .call_supervisor
                            .add_dialog(&call.call_id, &dialog_id);
                    }

                    let next_state = S2AConnectToAgentState::new();
                    return Ok(S2AStateAction::Transition(Box::new(next_state)));
                }
            }
            _ => {}
        }
        Ok(S2AStateAction::Stay)
    }

    async fn on_timer(
        &mut self,
        call: &mut SipToAppCall,
        timer: TimerType,
    ) -> anyhow::Result<S2AStateAction> {
        if timer == TimerType::WaitSDPTimeout {
            info!("{}: SDP timeout for call {}", STATE_NAME, call.call_id);
            let end_state = S2AEndState::new("Audio Bridge SDP timeout".to_string(), false, true);
            return Ok(S2AStateAction::Transition(Box::new(end_state)));
        }
        Ok(S2AStateAction::Stay)
    }
}
