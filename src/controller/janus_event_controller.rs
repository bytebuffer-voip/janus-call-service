use crate::app_state::AppState;
use crate::model::response::EntityBaseResponse;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Extension, Json, Router};
use http::StatusCode;
use log::info;
use serde_json::Value;
use std::sync::Arc;

pub fn janus_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/event", post(janus_event))
        .with_state(app_state.clone())
        .layer(Extension(app_state))
}

async fn janus_event(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<Value>,
) -> Response {
    info!("JanusEvent: {}", request.to_string());
    let resp = EntityBaseResponse::success("OK".to_string(), Some(""));
    (StatusCode::OK, Json(resp)).into_response()
}
