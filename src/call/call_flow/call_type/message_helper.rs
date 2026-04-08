use crate::model::candidate::Candidate;
use crate::websocket::websocket_handler::ConnectionState;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub fn notify_sdp(conn_state: &Arc<ConnectionState>, send_to: &Uuid, call_id: &str, sdp: &str) {
    let ntf = json!({
        "cmd": "sdp_ntf",
        "params": {
            "call_id": call_id,
            "sdp_type": "answer",
            "sdp": sdp,
        }
    });
    conn_state.send_to_client_by_id(send_to, ntf.to_string());
}

pub fn notify_candidate(
    conn_state: &Arc<ConnectionState>,
    send_to: &Uuid,
    call_id: &str,
    candidate: &Candidate,
) {
    let ntf = json!({
        "cmd": "candidate_ntf",
        "params": {
            "call_id": call_id,
            "candidate": candidate.candidate,
            "sdpMid": candidate.sdp_mid,
            "sdpMLineIndex": candidate.sdp_m_line_index,
        }
    });
    conn_state.send_to_client_by_id(send_to, ntf.to_string());
}

pub fn notify_call_end(
    conn_state: &Arc<ConnectionState>,
    call_id: &str,
    user_id: &str,
    reason: &str,
) {
    let msg = json!({
        "cmd": "call_end",
        "params": {
            "call_id": call_id,
            "reason": reason
        }
    });
    conn_state.send_to_user(user_id, msg.to_string());
}
