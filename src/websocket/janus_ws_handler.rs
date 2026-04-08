use crate::app_state::AppState;
use crate::websocket::websocket_handler::ConnectionState;
use axum::Extension;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::StreamExt;
use http::HeaderMap;
use log::info;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

pub async fn janus_ws_handler(
    headers: HeaderMap,
    Extension(app_state): Extension<Arc<AppState>>,
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<ConnectionState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let t = ws.on_upgrade(move |socket| handle_socket(socket, state, addr, app_state));
    t
}

async fn handle_socket(
    socket: WebSocket,
    conn_state: Arc<ConnectionState>,
    ip: SocketAddr,
    app_state: Arc<AppState>,
) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let ip_str = ip.to_string();
    loop {
        let msg_opt = tokio::select! {
            msg = receiver.next() => msg,
        };
        let Some(Ok(msg)) = msg_opt else { break };
        match msg {
            Message::Text(msg) => {
                info!("JanusEvent: {}", msg.to_string());
            }
            Message::Binary(_) => {}
            Message::Ping(_) => {}
            Message::Pong(_) => {}
            Message::Close(_) => {}
        }
    }
}
