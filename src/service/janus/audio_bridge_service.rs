use crate::app_state::AppState;
use crate::model::janus::{CreateJanusSessionResponse, JanusCreateRoomResp};
use crate::service::janus::session_service::send_request;
use crate::utils::code_utils;
use crate::utils::sdp_util::CodecInfo;
use log::info;
use rand::RngExt;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn attach(state: &Arc<AppState>, session_id: i64) -> anyhow::Result<i64> {
    let cfg = &state.config;
    let url = format!("{}/{}", &cfg.janus.http_uri, session_id);
    let json_body = json!({
        "janus": "attach",
        "plugin": "janus.plugin.audiobridge",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "apisecret" : cfg.janus.api_secret,
    });
    let body = send_request(&url, &json_body).await?;
    info!("attach response: {}", body.to_string());
    let resp: CreateJanusSessionResponse = serde_json::from_value(body)?;
    if resp.janus != "success" {
        return Err(anyhow::anyhow!(format!("Janus returned error: {:?}", resp)));
    }
    let data = resp
        .data
        .ok_or_else(|| anyhow::anyhow!("Empty janus session"))?;
    Ok(data.id)
}

pub async fn detach(state: &Arc<AppState>, session_id: i64, handle_id: i64) -> anyhow::Result<()> {
    let cfg = &state.config;
    let url = format!("{}/{}", &cfg.janus.http_uri, session_id);
    let json_body = json!({
        "janus": "detach",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "handle_id": handle_id,
        "apisecret" : cfg.janus.api_secret,
    });
    let body = send_request(&url, &json_body).await?;
    info!("detach response: {}", body.to_string());
    Ok(())
}

pub async fn create_room(
    state: &Arc<AppState>,
    session_id: i64,
    handle_id: i64,
) -> anyhow::Result<(i64, String, String)> {
    let room_id: i64 = rand::rng().random_range(1..=1_000_000_000);
    let pin = code_utils::generate_id(6);
    let secret = code_utils::generate_strong_password(10);
    let cfg = &state.config;
    let url = format!("{}/{}/{}", &cfg.janus.http_uri, session_id, handle_id);
    let desc = format!("Audio Bridge Room {}", room_id);
    let json_body = json!({
        "janus": "message",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "handle_id": handle_id,
        "apisecret" : cfg.janus.api_secret,
        "body": {
            "request": "create",
            "room": room_id,
            "description": desc,
            "is_private": false,
            "pin": pin.clone(),
            "secret": secret,
            "allow_rtp_participants" : true,
            "audiolevel_ext" : false,
            "audiolevel_event" : true,
        }
    });
    let body = send_request(&url, &json_body).await?;
    info!("Create Room response: {}", body.to_string());
    let resp: JanusCreateRoomResp = serde_json::from_value(body)?;
    if resp.janus != "success" {
        return Err(anyhow::anyhow!(format!("Janus returned error: {:?}", resp)));
    }
    if let Some(error_code) = resp.plugin_data.data.error_code {
        let error_msg = resp
            .plugin_data
            .data
            .error
            .unwrap_or(error_code.to_string());
        return Err(anyhow::anyhow!(format!(
            "Failed to create room, error: {}",
            error_msg
        )));
    }
    let room_id = resp
        .plugin_data
        .data
        .room
        .ok_or_else(|| anyhow::anyhow!("No room_id in create room response"));
    Ok((room_id?, pin, secret))
}

pub async fn delete_room(
    state: &Arc<AppState>,
    session_id: i64,
    handle_id: i64,
    room_id: i64,
    secret: String,
) -> anyhow::Result<()> {
    let cfg = &state.config;
    let url = format!("{}/{}/{}", &cfg.janus.http_uri, session_id, handle_id);
    let json_body = json!({
        "janus": "message",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "handle_id": handle_id,
        "apisecret" : cfg.janus.api_secret,
        "body": {
            "request": "destroy",
            "room": room_id,
            "secret": secret
        }
    });
    let body = send_request(&url, &json_body).await?;
    info!("Remove Room response: {}", body.to_string());
    Ok(())
}

pub async fn join(
    state: &Arc<AppState>,
    session_id: i64,
    handle_id: i64,
    display_name: String,
    room_id: i64,
    pin: String,
    secret: String,
) -> anyhow::Result<()> {
    let cfg = &state.config;
    let url = format!("{}/{}/{}", &cfg.janus.http_uri, session_id, handle_id);
    let json_body = json!({
        "janus": "message",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "handle_id": handle_id,
        "apisecret" : cfg.janus.api_secret,
        "body": {
            "request": "join",
            "room": room_id,
            "display": display_name,
            "pin" : pin,
            "denoise" : true,
            "secret" : secret
        }
    });
    let body = send_request(&url, &json_body).await?;
    info!("join response: {}", body.to_string());
    Ok(())
}

pub async fn join_with_rtp(
    state: &Arc<AppState>,
    session_id: i64,
    handle_id: i64,
    display_name: String,
    room_id: i64,
    pin: String,
    ip: String,
    port: u16,
    codec_info: Option<CodecInfo>,
    secret: String,
) -> anyhow::Result<()> {
    let cfg = &state.config;
    let url = format!("{}/{}/{}", &cfg.janus.http_uri, session_id, handle_id);

    let mut rtp = json!({
        "ip" : ip,
        "port" : port,
    });

    let mut codec = "pcma".to_string();
    if let Some(payload) = codec_info {
        if payload.need_pt_in_rtp {
            rtp["payload_type"] = json!(payload.payload_type);
        }
        codec = payload.janus_name.to_string();
    }

    let json_body = json!({
        "janus": "message",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "handle_id": handle_id,
        "apisecret" : cfg.janus.api_secret,
        "admin_secret" : cfg.janus.api_secret,
        "body": {
            "request": "join",
            "room": room_id,
            "display": display_name,
            "codec" : codec,
            "pin" : pin,
            "rtp" : rtp,
            "denoise" : true,
            "secret" : secret
        }
    });

    let body = send_request(&url, &json_body).await?;
    info!(
        "join_with_rtp uri: {}, body: {}, resp: {}",
        &url,
        json_body.to_string(),
        body.to_string()
    );
    Ok(())
}

pub async fn configure(
    state: &Arc<AppState>,
    session_id: i64,
    handle_id: i64,
    sdp_type: String,
    sdp: String,
) -> anyhow::Result<()> {
    let cfg = &state.config;
    let url = format!("{}/{}/{}", &cfg.janus.http_uri, session_id, handle_id);
    let jsep = json!({
        "type": sdp_type,
        "sdp": sdp,
    });
    let json_body = json!({
        "janus": "message",
        "transaction": uuid::Uuid::new_v4().to_string(),
        "session_id": session_id,
        "handle_id": handle_id,
        "apisecret" : cfg.janus.api_secret,
        "body": {
            "request": "configure",
            "muted" : false,
        },
        "jsep" : jsep,
    });
    let body = send_request(&url, &json_body).await?;
    info!("configure response: {}", body.to_string());
    Ok(())
}

pub async fn get_handle_info(
    state: &Arc<AppState>,
    session_id: i64,
    handle_id: i64,
) -> anyhow::Result<Value> {
    let cfg = &state.config;
    let url = format!("{}/{}/{}", &cfg.janus.admin_uri, session_id, handle_id);
    let json_body = json!({
        "janus": "handle_info",
        "session_id": session_id,
        "handle_id": handle_id,
        "admin_secret" : cfg.janus.admin_secret,
        "transaction": uuid::Uuid::new_v4().to_string(),
    });
    let body = send_request(&url, &json_body).await?;
    info!(
        "get_handle_info uri: {}, body: {}, resp: {}",
        &url,
        json_body.to_string(),
        body.to_string()
    );
    Ok(body)
}
