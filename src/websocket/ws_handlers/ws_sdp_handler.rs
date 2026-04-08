use crate::app_state::AppState;
use crate::call::call_flow::call_model::{CallEvent, WebsocketEvent};
use crate::utils::json_utils::get_string_value;
use crate::websocket::websocket_handler::ClientInfo;
use log::info;
use serde_json::Value;
use std::sync::Arc;

pub async fn handle_sdp_req(app_state: &Arc<AppState>, client_info: &ClientInfo, value: &Value) {
    if let Some(payload) = value.get("params") {
        let call_id = get_string_value(payload, "call_id");
        let sdp = get_string_value(payload, "sdp");
        if call_id.is_empty() || sdp.is_empty() {
            return;
        }
        let device_id = get_string_value(payload, "device_id");
        info!(
            "call: {}, Received SDP: {}, device_id: {}",
            call_id, sdp, device_id
        );
        if let Some(tx) = app_state.call_supervisor.get_call_tx(call_id) {
            let evt = CallEvent::Websocket(WebsocketEvent::OnSDP {
                client_info: client_info.clone(),
                sdp: sdp.to_string(),
            });
            if let Err(e) = tx.send(evt).await {
                info!("Error sending SDP to call supervisor: {:?}", e);
            };
        }
    }
}
