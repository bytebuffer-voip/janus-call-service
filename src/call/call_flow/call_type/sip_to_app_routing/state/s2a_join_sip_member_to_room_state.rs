use crate::call::call_flow::call_model::TimerType;
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_call_state::{
    S2ACallStateHandler, S2AStateAction,
};
use crate::call::call_flow::call_type::sip_to_app_routing::state::s2a_end_state::S2AEndState;
use crate::call::sip_to_app_call::SipToAppCall;
use crate::service::janus::audio_bridge_service;
use crate::utils::sdp_util::select_codec;
use crate::utils::{sdp_util, sip_utils};
use log::info;
use rsip::SipMessage;
use uuid::Uuid;

const STATE_NAME: &str = "S2AJoinSipMemberToRoomState";
pub struct S2AJoinSipMemberToRoomState {
    pub sdp_answer: Option<String>,
}

impl S2AJoinSipMemberToRoomState {
    pub fn new() -> Self {
        Self { sdp_answer: None }
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
        event: crate::call::call_flow::call_model::CallEvent,
    ) -> anyhow::Result<S2AStateAction> {
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
