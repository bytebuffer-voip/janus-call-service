use crate::app_state::AppState;
use crate::controller::auth_controller::auth_routes;
use crate::controller::janus_event_controller::janus_routes;
use crate::controller::user_controller::user_routes;
use crate::websocket::janus_ws_handler::janus_ws_handler;
use crate::websocket::websocket_handler::{ConnectionState, ws_handler};
use axum::routing::any;
use axum::{Extension, Router};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>, connection_state: Arc<ConnectionState>) -> Router {
    let api = Router::new()
        .nest("/auth", auth_routes(state.clone()))
        .nest("/user", user_routes(state.clone()))
        .nest("/janus", janus_routes(state.clone()))
        .route("/call", any(ws_handler).layer(Extension(state.clone())))
        .route(
            "/janus-ws",
            any(janus_ws_handler).layer(Extension(state.clone())),
        )
        .layer(Extension(connection_state));
    api
}
