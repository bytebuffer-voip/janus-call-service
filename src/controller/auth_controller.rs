use crate::app_state::AppState;
use crate::model::auth::LoginRequest;
use crate::service::login_service;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use axum::{Extension, Json, Router};
use cookie::time::Duration;
use cookie::{Cookie, SameSite};
use log::info;
use std::sync::Arc;

pub fn auth_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/login", post(login_handler))
        .with_state(app_state.clone())
        .layer(Extension(app_state))
}

async fn login_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Response {
    info!("{:?}", request);
    if let Some(resp) = request.validate() {
        info!("Validation failed: {:?}", resp);
        return (StatusCode::BAD_REQUEST, Json(resp)).into_response();
    }

    let resp = login_service::login(&state, request)
        .await
        .unwrap_or_else(|e| {
            info!("Login service failed: {:?}", e);
            crate::model::auth::AuthResponse::fails(e.to_string())
        });

    if resp.rc != 0 {
        return (StatusCode::UNAUTHORIZED, Json(resp)).into_response();
    }

    let token_value = resp.token.clone().unwrap_or_default();
    let cookie = Cookie::build(("t", token_value))
        .path("/")
        .secure(false)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(Duration::days(365))
        .build();

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());

    (StatusCode::OK, headers, Json(resp)).into_response()
}
