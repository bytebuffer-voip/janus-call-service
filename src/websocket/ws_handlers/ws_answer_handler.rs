use crate::app_state::AppState;
use crate::call::call_flow::call_model::{CallEvent, WebsocketEvent};
use crate::utils::json_utils::get_string_value;
use crate::websocket::websocket_handler::ClientInfo;
use log::info;
use serde_json::Value;
use std::sync::Arc;

pub async fn handle_call_answer_req(
    app_state: &Arc<AppState>,
    client_info: &ClientInfo,
    value: &Value,
) {
    let payload = match value.get("params") {
        Some(v) => v,
        None => return,
    };

    let Some(code) = payload.get("code").and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }) else {
        return;
    };

    let call_id = get_string_value(payload, "call_id");
    if call_id.is_empty() {
        return;
    }

    let sdp = payload
        .get("sdp")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if let Some(tx) = app_state.call_supervisor.get_call_tx(&call_id) {
        let evt = CallEvent::Websocket(WebsocketEvent::OnAnswer {
            client_info: client_info.clone(),
            sdp: sdp.to_string(),
            code,
        });
        if let Err(e) = tx.send(evt).await {
            info!("Error sending answered to call supervisor: {:?}", e);
        };
    }
}
