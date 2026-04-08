use crate::model::response::BaseResponse;
use crate::model::user_info::UserInfo;
use crate::utils::email_utils;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl LoginRequest {
    pub fn validate(&self) -> Option<BaseResponse> {
        if !email_utils::is_valid_email(&self.email) {
            return Some(BaseResponse::fails("Invalid email format".to_string()));
        }
        if self.password.len() < 6 {
            return Some(BaseResponse::fails(
                "Password must be at least 6 characters long".to_string(),
            ));
        }
        None
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthResponse {
    pub rc: i64,
    pub rd: String,
    pub token: Option<String>,
    pub user: Option<UserInfo>,
}

impl AuthResponse {
    pub fn fails(rd: String) -> Self {
        AuthResponse {
            rc: -1,
            rd,
            token: None,
            user: None,
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebsocketAuthRequest {
    pub token: String,
    #[serde(default)]
    pub device_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebsocketAuthData {
    pub user_id: String,
    pub name: String,
    pub device_id: String,
}