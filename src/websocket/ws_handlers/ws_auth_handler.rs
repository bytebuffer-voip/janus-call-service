use crate::app_state::AppState;
use crate::model::auth::{WebsocketAuthData, WebsocketAuthRequest};
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;
use serde_json::Value;
use std::sync::Arc;
use futures_util::SinkExt;
use log::info;
use tokio::sync::Mutex;
use crate::utils::jwt_helper;

pub async fn handle_auth(
    app_state: &Arc<AppState>,
    auth_tx: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<WebsocketAuthData>>>>,
    sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>,
    value: &Value,
) {
    let payload = match value.get("params") {
        Some(v) => v,
        None => return,
    };
    let Ok(auth) = serde_json::from_value::<WebsocketAuthRequest>(payload.clone()) else {
        return;
    };
    let jwt_key = &app_state.config.jwt_key;
    match jwt_helper::verify_token(&auth.token, jwt_key) {
        Ok(data) => {
            let claims = data.claims;
            let auth_data = WebsocketAuthData {
                user_id: claims.user_id,
                name: claims.display_name,
                device_id: auth.device_id,
            };
            if let Some(tx) = auth_tx.lock().await.take() {
                let _ = tx.send(auth_data);
            }
        }
        Err(e) => {
            info!("Authentication error: {}", e);
            let _ = sender.lock().await.send(Message::Close(None)).await;
        }
    }
}
