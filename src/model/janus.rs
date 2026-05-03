use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateJanusSessionRespData {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateJanusSessionResponse {
    pub janus: String,
    pub transaction: Option<String>,
    pub data: Option<CreateJanusSessionRespData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JanusCreateRoomData {
    #[serde(rename = "audiobridge")]
    pub audio_bridge: Option<String>,
    pub room: Option<i64>,
    pub permanent: Option<bool>,
    pub error_code: Option<i32>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JanusPluginData {
    pub plugin: String,
    pub data: JanusCreateRoomData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JanusCreateRoomResp {
    pub janus: String,
    pub session_id: i64,
    pub transaction: String,
    pub sender: i64,
    #[serde(rename = "plugindata")]
    pub plugin_data: JanusPluginData,
}