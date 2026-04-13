use crate::app_state::AppState;
use crate::model::janus::CreateJanusSessionResponse;
use log::info;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

pub async fn send_request(url: &str, json_body: &Value) -> anyhow::Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client.post(url).json(json_body).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(format!(
            "Failed to send a message: {}",
            response.status()
        )));
    }
    let value: Value = response.json().await?;
    Ok(value)
}

pub async fn create_session(state: &Arc<AppState>) -> anyhow::Result<i64> {
    let cfg = &state.config;
    let url = &cfg.janus.http_uri;
    let json_body = json!({
        "janus": "create",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "apisecret" : cfg.janus.api_secret,
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client.post(url).json(&json_body).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(format!(
            "Failed to create janus session: {}",
            response.status()
        )));
    }
    let body: Value = response.json().await?;
    info!("Create janus session got response: {}", body.to_string());
    let resp: CreateJanusSessionResponse = serde_json::from_value(body)?;
    if resp.janus != "success" {
        return Err(anyhow::anyhow!(format!("Janus returned error: {:?}", resp)));
    }
    let data = resp
        .data
        .ok_or_else(|| anyhow::anyhow!("Empty janus session"))?;
    Ok(data.id)
}

pub async fn destroy_session(state: &Arc<AppState>, session_id: i64) -> anyhow::Result<()> {
    let cfg = &state.config;
    let url = format!("{}/{}", &cfg.janus.http_uri, session_id);
    let json_body = json!({
        "janus": "destroy",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "apisecret" : cfg.janus.api_secret,
        "session_id": session_id,
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client.post(url).json(&json_body).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(format!(
            "Failed to destroy janus session: {}",
            response.status()
        )));
    }
    let body: Value = response.json().await?;
    info!("Destroy janus session got response: {}", body.to_string());
    Ok(())
}

pub async fn keepalive(state: &Arc<AppState>, session_id: i64) -> anyhow::Result<()> {
    let cfg = &state.config;
    let url = format!("{}/{}", &cfg.janus.http_uri, session_id);
    let json_body = json!({
        "janus": "keepalive",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "apisecret" : cfg.janus.api_secret,
    });
    let body = send_request(&url, &json_body).await?;
    info!("keepalive response: {}", body.to_string());
    Ok(())
}
