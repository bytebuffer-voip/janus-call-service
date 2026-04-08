use crate::app_state::AppState;
use crate::config::constants::COLLECTION_USERS;
use crate::config::mongodb_cfg::get_mongo_client;
use crate::model::auth::{AuthResponse, LoginRequest};
use crate::model::user::User;
use crate::model::user_info::UserInfo;
use crate::utils::jwt_helper::create_token_for_user;
use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use mongodb::bson::doc;
use std::sync::Arc;

pub async fn login(state: &Arc<AppState>, request: LoginRequest) -> anyhow::Result<AuthResponse> {
    let cfg = &state.config;
    let username = request.email.to_lowercase();

    let client = get_mongo_client().await?;
    let collection = client
        .database(&cfg.mongodb.db_name)
        .collection::<User>(COLLECTION_USERS);

    let filter = doc! { "username": &username, "network": "web" };
    let user = collection
        .find_one(filter)
        .await?
        .ok_or_else(|| anyhow!("User not found"))?;

    if !verify_password(&user.password, &request.password)? {
        return Err(anyhow!("Password incorrect"));
    }

    let user_info = UserInfo {
        user_id: user.id.clone(),
        username: username.clone(),
        display_name: username.clone(),
        title: String::new(),
        avatar: "".to_string(),
    };
    let token = create_token_for_user(&user_info, &cfg.jwt_key)?;

    Ok(AuthResponse {
        rc: 0,
        rd: "OK".to_string(),
        token: Some(token),
        user: Some(user_info),
    })
}

fn verify_password(hash_str: &str, password: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(hash_str).map_err(|_| anyhow!("Invalid hash"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
