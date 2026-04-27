use crate::app_state::AppState;
use crate::middlerware::auth::auth;
use crate::model::response::EntityBaseResponse;
use crate::model::user::User;
use crate::model::user_info::UserInfo;
use crate::repo::user_repo;
use axum::routing::get;
use axum::{middleware, Extension, Json, Router};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use axum::extract::Query;

pub fn user_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/me", get(get_me))
        .route("/others", get(get_others))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state.clone())
        .layer(Extension(state))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeResponse {
    user_id: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    phone_number: Option<String>,
    language_code: Option<String>,
    time_zone: Option<String>,
    photo_url: Option<String>,
    created_at: Option<i64>,
}

impl MeResponse {
    pub fn from_user(user: &User) -> Self {
        Self {
            user_id: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            phone_number: user.phone_number.clone(),
            language_code: user.language_code.clone(),
            time_zone: user.time_zone.clone(),
            photo_url: user.photo_url.clone(),
            created_at: user.created_at,
        }
    }
}

async fn get_me(Extension(user_info): Extension<UserInfo>) -> Json<EntityBaseResponse<MeResponse>> {
    match user_repo::get_user(&user_info.user_id).await {
        Ok(Some(user)) => {
            let me = MeResponse::from_user(&user);
            Json(EntityBaseResponse::success("OK".to_string(), Some(me)))
        }
        Ok(None) => Json(EntityBaseResponse::fails("User not found".to_string())),
        Err(e) => {
            info!("get_me error: {:?}", e);
            Json(EntityBaseResponse::fails(e.to_string()))
        }
    }
}

#[derive(Deserialize)]
struct OptionalSearchQuery {
    key: Option<String>,
}

async fn get_others(
    Extension(user_info): Extension<UserInfo>,
    query: Query<OptionalSearchQuery>,
) -> Json<EntityBaseResponse<Vec<MeResponse>>> {
    match user_repo::get_users_except(&user_info.user_id, query.key.as_deref()).await {
        Ok(users) => {
            let list: Vec<MeResponse> = users.iter().map(MeResponse::from_user).collect();
            Json(EntityBaseResponse::success("OK".to_string(), Some(list)))
        }
        Err(e) => {
            info!("get_others error: {:?}", e);
            Json(EntityBaseResponse::fails(e.to_string()))
        }
    }
}
