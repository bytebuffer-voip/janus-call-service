use crate::app_state::AppState;
use crate::config::config::Config;
use crate::model::user_info::UserInfo;
use crate::utils::{cookie_util, jwt_helper};
use axum::Json;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;

pub async fn auth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut req: Request,
    next: axum::middleware::Next,
) -> Result<Response, Response> {
    let path = req.uri().path();
    let method = req.method();
    if method == Method::OPTIONS || path == "/login" {
        return Ok(next.run(req).await);
    }
    if let Some(user) = try_get_user(&headers).await {
        req.extensions_mut().insert(user);
        return Ok(next.run(req).await);
    }
    let error_response = Json(json!({
        "rc": 401,
        "rd": "Unauthorized, please authenticate",
    }));

    let response = (StatusCode::UNAUTHORIZED, error_response).into_response();
    Err(response)
}

async fn try_get_user(headers: &HeaderMap) -> Option<UserInfo> {
    let Some(token) = cookie_util::get_token_from_cookies(headers) else {
        return None;
    };
    let jwt_key = &Config::global_config().jwt_key;
    let claims = jwt_helper::verify_token(&token.to_string(), jwt_key)
        .ok()?
        .claims;
    let user_info = UserInfo {
        user_id: claims.user_id,
        username: claims.sub,
        display_name: claims.display_name,
        title: claims.title,
        avatar: claims.avatar,
    };
    Some(user_info)
}
