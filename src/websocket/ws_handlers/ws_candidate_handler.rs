use crate::app_state::AppState;
use crate::call::call_flow::call_model::{CallEvent, WebsocketEvent};
use crate::utils::json_utils::{get_int, get_string, get_string_value};
use crate::websocket::websocket_handler::ClientInfo;
use log::info;
use serde_json::Value;
use std::sync::Arc;

pub async fn handle_candidate_req(
    app_state: &Arc<AppState>,
    client_info: &ClientInfo,
    value: &Value,
) {
    let payload = match value.get("params") {
        Some(v) => v,
        None => return,
    };
    let call_id = get_string_value(payload, "call_id");
    let candidate = get_string_value(payload, "candidate");
    let sdp_mid = get_string(payload, "sdp_mid");
    let sdp_mline_index = get_int(payload, "sdp_mline_index");
    if call_id.is_empty() || candidate.is_empty() {
        return;
    }
    if candidate.contains("srflx") || candidate.contains("relay") {
        info!("call: {}, Received ICE candidate: {}", call_id, candidate);
    }
    if let Some(tx) = app_state.call_supervisor.get_call_tx(call_id) {
        let evt = CallEvent::Websocket(WebsocketEvent::OnICECandidate {
            client_info: client_info.clone(),
            candidate: candidate.to_string(),
            sdp_mid: sdp_mid.map(|s| s.to_string()),
            sdp_mline_index,
        });
        if let Err(e) = tx.send(evt).await {
            info!("Error sending ICE candidate to call supervisor: {:?}", e);
        };
    }
}
