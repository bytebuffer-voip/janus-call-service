use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: String,
    pub network: String,
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
    pub language_code: Option<String>,
    pub time_zone: Option<String>,
    pub date_format: Option<u8>,
    pub time_format: Option<u8>,
    pub number_format: Option<u8>,
    pub photo_url: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub current_balance: f64,
    pub status: Option<UserStatus>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UserStatus {
    Pending,
    Active,
    Inactive,
    Locked,
    Suspended,
    Deleted,
}

impl UserStatus {
    pub fn from_str(status: &str) -> Self {
        match status {
            "Pending" => UserStatus::Pending,
            "Active" => UserStatus::Active,
            "Inactive" => UserStatus::Inactive,
            "Locked" => UserStatus::Locked,
            "Suspended" => UserStatus::Suspended,
            "Deleted" => UserStatus::Deleted,
            _ => UserStatus::Pending,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            UserStatus::Pending => "Pending",
            UserStatus::Active => "Active",
            UserStatus::Inactive => "Inactive",
            UserStatus::Locked => "Locked",
            UserStatus::Suspended => "Suspended",
            UserStatus::Deleted => "Deleted",
        }
    }
}
