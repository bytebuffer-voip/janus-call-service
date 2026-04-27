use crate::app_state::AppState;
use crate::websocket::websocket_handler::ConnectionState;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct JanusWebRTCSessionManager {
    pub call_id: String,
    pub session_id: i64,
    pub client_handle: HashMap<Uuid, i64>,
    pub handle_client: HashMap<i64, Uuid>,
}

impl JanusWebRTCSessionManager {
    pub fn new(call_id: String, session_id: i64) -> Self {
        Self {
            call_id,
            session_id,
            client_handle: HashMap::new(),
            handle_client: HashMap::new(),
        }
    }

    pub fn add_client_handle(&mut self, client_id: Uuid, handle_id: i64) {
        self.client_handle.insert(client_id.clone(), handle_id);
        self.handle_client.insert(handle_id, client_id);
    }

    pub async fn on_server_sdp(
        &mut self,
        app_state: &Arc<AppState>,
        conn_state: &Arc<ConnectionState>,
        handle_id: i64,
        sdp: &str,
    ) -> anyhow::Result<bool> {
        if let Some(client_id) = self.handle_client.get(&handle_id) {
            let msg = json!({
                "cmd": "sdp_ntf",
                "params": {
                    "call_id": self.call_id,
                    "sdp_type": "answer",
                    "sdp": sdp
                }
            });
            conn_state.send_to_client_by_id(client_id, msg.to_string());
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn on_server_candidate(
        &mut self,
        app_state: &Arc<AppState>,
        conn_state: &Arc<ConnectionState>,
        handle_id: i64,
        candidate: &str,
    ) -> anyhow::Result<()> {
        if let Some(client_id) = self.handle_client.get(&handle_id) {
            let candidate_str = if candidate.starts_with("candidate:") {
                candidate.to_string()
            } else {
                format!("candidate:{}", candidate)
            };
            let ntf = json!({
                "cmd": "candidate_ntf",
                "params": {
                    "call_id": self.call_id.clone(),
                    "candidate": candidate_str,
                }
            });
            conn_state.send_to_client_by_id(client_id, ntf.to_string());
        }
        Ok(())
    }

    pub async fn on_client_candidate(
        &mut self,
        app_state: &Arc<AppState>,
        conn_state: &Arc<ConnectionState>,
        client_id: &Uuid,
        candidate: &str,
        sdp_mline_index: Option<i64>,
        sdp_mid: Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(handle_id) = self.client_handle.get(client_id) {
            let sdp_mline_index = sdp_mline_index.unwrap_or(0) as u32;
            let sdp_mid = sdp_mid.unwrap_or_else(|| "audio".to_string());
            // audio_bridge_service::on_candidate(
            //     app_state,
            //     self.session_id,
            //     *handle_id,
            //     sdp_mid,
            //     sdp_mline_index,
            //     candidate.to_string(),
            // )
            //     .await?;
        }
        Ok(())
    }

    pub async fn on_client_candidate_completed(
        &mut self,
        app_state: &Arc<AppState>,
        conn_state: &Arc<ConnectionState>,
        client_id: &Uuid,
    ) -> anyhow::Result<()> {
        // if let Some(handle_id) = self.client_handle.get(client_id) {
        //     audio_bridge_service::on_end_candidate(app_state, self.session_id, *handle_id).await?;
        // }
        Ok(())
    }
}
