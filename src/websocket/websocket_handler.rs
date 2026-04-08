use crate::app_state::AppState;
use crate::config::config::Config;
use crate::model::auth::WebsocketAuthData;
use crate::utils::{cookie_util, jwt_helper};
pub(crate) use crate::websocket::ws_connection::{ClientInfo, ConnectionState};
use crate::websocket::ws_handlers::ws_answer_handler::handle_call_answer_req;
use crate::websocket::ws_handlers::ws_auth_handler::handle_auth;
use crate::websocket::ws_handlers::ws_candidate_handler::handle_candidate_req;
use crate::websocket::ws_handlers::ws_end_call_handler::handle_end_call_req;
use crate::websocket::ws_handlers::ws_incall_req_handler::handle_in_call_resp;
use crate::websocket::ws_handlers::ws_sdp_handler::handle_sdp_req;
use crate::websocket::ws_handlers::ws_start_call::handle_call_start_req;
use axum::extract::ws::Utf8Bytes;
use axum::{
    Extension,
    extract::{
        ConnectInfo,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use http::HeaderMap;
use log::debug;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc, oneshot};
use uuid::Uuid;

pub async fn ws_handler(
    headers: HeaderMap,
    Extension(app_state): Extension<Arc<AppState>>,
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<ConnectionState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let access_token = cookie_util::get_token_from_cookies(&headers);
    let t =
        ws.on_upgrade(move |socket| handle_socket(access_token, socket, state, addr, app_state));
    t
}

async fn handle_socket(
    access_token: Option<String>,
    socket: WebSocket,
    conn_state: Arc<ConnectionState>,
    ip: SocketAddr,
    app_state: Arc<AppState>,
) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let client_id = Uuid::new_v4();
    let ip_str = ip.to_string();
    let (auth_tx, auth_rx) = oneshot::channel::<WebsocketAuthData>();
    let auth_tx = Arc::new(Mutex::new(Some(auth_tx)));

    let (auth_done_tx, mut auth_done_rx) = mpsc::unbounded_channel::<ClientInfo>();
    let mut client_info: Option<ClientInfo> = None;

    let auth_state = conn_state.clone();
    let tx_clone = tx.clone();

    if let Some(token) = access_token {
        let jwt_key = &Config::global_config().jwt_key;
        if let Ok(data) = jwt_helper::verify_token(&token, jwt_key) {
            let claims = data.claims;
            debug!(
                "websocket authenticate user: {} successful",
                &claims.user_id
            );
            let info = ClientInfo {
                user_id: claims.user_id.clone(),
                name: claims.display_name.clone(),
                client_id,
                device_id: String::new(),
                ip: ip_str.clone(),
                sender: tx_clone,
            };
            auth_state.add_client(client_id, info.clone());
            client_info = Some(info);
        }
    }

    if client_info.is_none() {
        let tx_clone = tx.clone();
        let auth_sender = sender.clone();
        tokio::spawn(async move {
            if let Ok(Ok(auth_data)) = tokio::time::timeout(Duration::from_secs(30), auth_rx).await
            {
                let info = ClientInfo {
                    user_id: auth_data.user_id.clone(),
                    name: auth_data.name.clone(),
                    client_id,
                    device_id: auth_data.device_id.clone(),
                    ip: ip_str,
                    sender: tx_clone,
                };
                auth_state.add_client(client_id, info.clone());
                let _ = auth_done_tx.send(info);
            } else {
                let _ = auth_sender.lock().await.send(Message::Close(None)).await;
            }
        });
    }

    // Task send ping every 10 seconds
    let sender_clone = sender.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let mut sender_guard = sender_clone.lock().await;
            if sender_guard
                .send(Message::Ping(Bytes::from(vec![])))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Task send data from server to guest
    let sender_clone = sender.clone();
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if sender_clone
                .lock()
                .await
                .send(Message::Text(Utf8Bytes::from(data)))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    loop {
        let msg_opt = tokio::select! {
            msg = receiver.next() => msg,
            Some(info) = auth_done_rx.recv(), if client_info.is_none() => {
                client_info = Some(info);
                continue;
            }
        };
        let Some(Ok(msg)) = msg_opt else { break };
        match msg {
            Message::Text(text) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(cmd) = value.get("cmd").and_then(|v| v.as_str()) {
                        if cmd != "auth" && client_info.is_none() {
                            let _ = sender
                                .lock()
                                .await
                                .send(Message::Text(Utf8Bytes::from(
                                    "{\"error\":\"unauthorized\"}",
                                )))
                                .await;
                            continue;
                        }
                        match cmd {
                            "auth" => handle_auth(&app_state, &auth_tx, &sender, &value).await,
                            "call_answer_req" => {
                                if let Some(client_info) = &client_info {
                                    handle_call_answer_req(&app_state, client_info, &value).await;
                                }
                            }
                            "sdp_req" => {
                                if let Some(client_info) = &client_info {
                                    handle_sdp_req(&app_state, client_info, &value).await;
                                }
                            }
                            "candidate_req" => {
                                if let Some(client_info) = &client_info {
                                    handle_candidate_req(&app_state, client_info, &value).await;
                                }
                            }
                            "call_start_req" => {
                                if let Some(client_info) = &client_info {
                                    handle_call_start_req(
                                        &app_state,
                                        &conn_state,
                                        &sender,
                                        client_info,
                                        &value,
                                    )
                                    .await
                                }
                            }
                            "end_call_req" => {
                                if let Some(client_info) = &client_info {
                                    handle_end_call_req(&app_state, client_info, &value).await;
                                }
                            }
                            "in_call_resp" => {
                                if let Some(client_info) = &client_info {
                                    handle_in_call_resp(&app_state, client_info, &value).await;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Message::Ping(payload) => {
                let _ = sender.lock().await.send(Message::Pong(payload)).await;
            }
            Message::Pong(_) => {}
            Message::Close(_) => break,
            _ => {}
        }
    }

    debug!("Websocket connection closed --- client_id: {}", client_id);
    conn_state.remove_client(&client_id);
    debug!("Disconnected from {}", client_id);
}
