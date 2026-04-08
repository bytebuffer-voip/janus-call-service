use crate::app_state::AppState;
use crate::call::call_flow::call_model::Call;
use crate::repo::user_repo;
use crate::utils::call_id_gen::gen_call_id;
use crate::utils::json_utils::get_string_value;
use crate::websocket::websocket_handler::{ClientInfo, ConnectionState};
use axum::extract::ws::{Message, WebSocket};
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use log::info;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

type WsSender = Arc<Mutex<SplitSink<WebSocket, Message>>>;

pub async fn send_json(sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>, json: Value) {
    let msg = Message::Text(json.to_string().into());
    if sender.lock().await.send(msg).await.is_err() {
        info!("Error sending message: {}", json);
    }
}

async fn send_err_resp(sender: &WsSender, rc: i32, rd: &str, req_id: &str) {
    let resp = json!({
        "cmd": "call_start_resp",
        "params": { "rc": rc, "rd": rd, "req_id": req_id }
    });
    send_json(sender, resp).await;
}

pub async fn handle_call_start_req(
    app_state: &Arc<AppState>,
    conn_state: &Arc<ConnectionState>,
    sender: &WsSender,
    client_info: &ClientInfo,
    value: &Value,
) {
    info!("Received call_start_req: {}", value);
    let Some(payload) = value.get("params") else {
        return;
    };

    let caller_id = &client_info.user_id;
    let callee = get_string_value(payload, "callee");
    let req_id = get_string_value(payload, "req_id");

    if callee.is_empty() {
        send_err_resp(sender, 400, "Callee is required", &req_id).await;
        return;
    }

    if callee == client_info.user_id {
        send_err_resp(sender, 400, "Callee cannot be the same as caller", &req_id).await;
        return;
    }

    let Ok(Some(caller_user)) = user_repo::get_user(caller_id).await else {
        send_err_resp(sender, 400, "Caller user not found", &req_id).await;
        return;
    };

    let Ok(Some(callee_user)) = user_repo::get_user(callee).await else {
        send_err_resp(sender, 400, "Callee user not found", &req_id).await;
        return;
    };

    let call_id = gen_call_id();
    info!(
        "Starting call from {} to {}, call_id: {}",
        caller_id, callee, call_id
    );

    let supervisor = app_state.call_supervisor.clone();
    let app_for_call = app_state.clone();
    let conn_for_call = conn_state.clone();
    let id_for_call = call_id.clone();

    // let params = PeerToPeerCallParams {
    //     caller_client_info: client_info.clone(),
    //     caller: caller_id.to_string(),
    //     caller_user,
    //     callee_user,
    // };
    //
    // supervisor
    //     .start_call(
    //         app_state.clone(),
    //         conn_state.clone(),
    //         id_for_call.clone().as_str(),
    //         move |api_tx| {
    //             Call::PeerToPeer(PeerToPeerCall::new(
    //                 app_for_call,
    //                 conn_for_call,
    //                 id_for_call,
    //                 params,
    //                 api_tx,
    //             ))
    //         },
    //     )
    //     .await;

    let resp = json!({
        "cmd": "call_start_resp",
        "params": {
            "rc": 0,
            "rd": "Success",
            "call_id": call_id,
            "status": "started",
            "req_id": req_id
        }
    });
    send_json(sender, resp).await;
}
