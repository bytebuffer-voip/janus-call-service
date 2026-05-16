use crate::app_state::AppState;
use crate::call::call_flow::call_model::Call;
use crate::call::sip_to_app_call::{SipToAppCall, SipToAppParams};
use crate::config::config::Config;
use crate::service::janus::{audio_bridge_service, session_service};
use crate::utils::call_id_gen::gen_call_id;
use crate::websocket::websocket_handler::ConnectionState;
use log::info;
use rsip::prelude::HeadersExt;
use rsip::{Header, Headers, Method, Request, Response, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;

pub struct SipTransport {
    pub socket: Arc<tokio::net::UdpSocket>,
}

impl SipTransport {
    pub async fn bind(addr: &str) -> std::io::Result<Self> {
        let socket = tokio::net::UdpSocket::bind(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub async fn send(&self, data: &str, addr: SocketAddr) -> anyhow::Result<()> {
        self.socket.send_to(data.as_bytes(), addr).await?;
        Ok(())
    }
}

pub async fn recv_loop(
    state: &Arc<AppState>,
    conn_state: &Arc<ConnectionState>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 65535];
    info!("SIP recv_loop started, listening for responses...");
    loop {
        let (len, src) = state.sip_transport.socket.recv_from(&mut buf).await?;
        let msg = String::from_utf8_lossy(&buf[..len]);
        info!("Received SIP message from {}: {}", src, msg);
        if let Ok(req) = Request::try_from(msg.as_ref()) {
            let method = req.method();
            let body = req.body.clone();
            let text_body = String::from_utf8_lossy(body.as_ref()); // sdp offer

            match method {
                Method::Invite => {
                    let ringing_resp = make_ringing_response(&state.config, &req);
                    info!(
                        "Sending Ringing Response:\n{}",
                        ringing_resp.to_string().as_str()
                    );

                    if let Err(e) = send_sip_response(state, &ringing_resp).await {
                        info!("Error sending Ringing response: {:?}", e);
                        continue;
                    }

                    let session_id = match session_service::create_session(state).await {
                        Ok(sid) => sid,
                        Err(err) => {
                            info!("Failed to create Janus session: {}", err);
                            continue;
                        }
                    };

                    let handle_id = match audio_bridge_service::attach(&state, session_id).await {
                        Ok(hid) => hid,
                        Err(err) => {
                            info!("Failed to attach AudioBridge: {}", err);
                            let _ = session_service::destroy_session(&state, session_id).await;
                            continue;
                        }
                    };

                    let (room_id, pin, secret) = match audio_bridge_service::create_room(
                        &state, session_id, handle_id,
                    )
                    .await
                    {
                        Ok((rid, p, s)) => (rid, p, s),
                        Err(err) => {
                            info!("Error creating audio bridge room: {}", err);
                            let _ =
                                audio_bridge_service::detach(&state, session_id, handle_id).await;
                            let _ = session_service::destroy_session(&state, session_id).await;
                            continue;
                        }
                    };

                    let params =
                        SipToAppParams::new(req, session_id, handle_id, room_id, pin, secret);

                    let janus_key = format!("janus_{}_{}", session_id, handle_id);

                    let call_id = gen_call_id();
                    let cs = state.call_supervisor.clone();
                    let state_clone = state.clone();
                    let state_clone_2 = state_clone.clone();
                    let conn_clone = conn_state.clone();
                    let conn_clone_2 = conn_state.clone();
                    let call_id_clone = call_id.clone();

                    cs.clone()
                        .start_call(
                            state_clone_2,
                            conn_clone_2,
                            &call_id_clone,
                            Some(janus_key),
                            move |api| {
                                Call::SIPToApp(SipToAppCall::new(
                                    state_clone,
                                    conn_clone,
                                    call_id,
                                    params,
                                    api,
                                ))
                            },
                        )
                        .await;
                }
                v => {
                    info!(
                        "Received SIP request with method {} and body: {}",
                        v, text_body
                    );
                }
            }
        }
    }
}

pub async fn send_sip_response(state: &Arc<AppState>, response: &Response) -> anyhow::Result<()> {
    let kamailio_addr = state.config.kamailio.socket_addr()?;
    state
        .sip_transport
        .send(&response.to_string(), kamailio_addr)
        .await?;
    Ok(())
}

pub fn make_ringing_response(config: &Config, invite: &Request) -> Response {
    build_response(config, invite, StatusCode::Ringing)
}

fn build_response(config: &Config, invite: &Request, code: StatusCode) -> Response {
    let mut headers = Headers::default();

    // Copy Via headers (QUAN TRỌNG: giữ nguyên thứ tự)
    invite.headers.iter().for_each(|e| {
        if let Header::Via(via) = e {
            headers.push(Header::Via(via.clone().into()));
        }
    });

    // Copy Record-Route headers
    invite.headers.iter().for_each(|e| {
        if let Header::RecordRoute(r) = e {
            headers.push(Header::RecordRoute(r.clone().into()));
        }
    });

    // From header (copy từ request)
    headers.push(Header::From(invite.from_header().unwrap().clone().into()));

    // To header (copy từ request)
    headers.push(Header::To(invite.to_header().unwrap().clone().into()));

    // Call-ID header
    headers.push(Header::CallId(
        invite.call_id_header().unwrap().clone().into(),
    ));

    // CSeq header
    headers.push(Header::CSeq(invite.cseq_header().unwrap().clone().into()));

    // Contact header
    let proxy = format!(
        "{}:{}",
        config.sip_transport.public_ip, config.sip_transport.port
    );
    let contact = rsip::typed::Contact {
        display_name: None,
        uri: rsip::Uri {
            scheme: Some(rsip::Scheme::Sip),
            auth: None,
            host_with_port: rsip::Domain::from(proxy).into(),
            params: Default::default(),
            headers: Default::default(),
        },
        params: Default::default(),
    };
    headers.push(Header::Contact(contact.into()));

    Response {
        version: invite.version().clone(),
        status_code: code,
        headers,
        body: vec![],
    }
}
