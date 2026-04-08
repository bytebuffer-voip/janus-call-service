use crate::model::user_info::UserInfo;
use chrono::Local;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: String,
    pub user_id: String,
    pub display_name: String,
    pub avatar: String,
    pub title: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn create_token_for_user(user: &UserInfo, jwt_key: &str) -> anyhow::Result<String> {
    let now = Local::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::from_secs(15552000)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: user.username.clone(),
        user_id: user.user_id.clone(),
        display_name: user.display_name.clone(),
        avatar: user.avatar.clone(),
        title: user.title.clone(),
        exp,
        iat,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_key.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_token(token: &String, secret_key: &String) -> anyhow::Result<TokenData<TokenClaims>> {
    let decoding_key = DecodingKey::from_secret(secret_key.as_ref());
    let validation = Validation::new(Algorithm::HS256);
    decode::<TokenClaims>(token, &decoding_key, &validation)
        .map(|data| data)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}
