use crate::app_state::AppState;
use crate::call::call_flow::call_model::{CallEvent, WebsocketEvent};
use crate::utils::json_utils::get_string_value;
use crate::websocket::websocket_handler::ClientInfo;
use log::info;
use serde_json::Value;
use std::sync::Arc;

pub async fn handle_end_call_req(
    app_state: &Arc<AppState>,
    client_info: &ClientInfo,
    value: &Value,
) {
    let payload = match value.get("params") {
        Some(v) => v,
        None => return,
    };
    let call_id = get_string_value(payload, "call_id");
    if call_id.is_empty() {
        return;
    }
    if let Some(tx) = app_state.call_supervisor.get_call_tx(call_id) {
        let evt = CallEvent::Websocket(WebsocketEvent::EndCall(client_info.clone()));
        if let Err(e) = tx.send(evt).await {
            info!("Error sending end call to call supervisor: {:?}", e);
        };
    }
}
