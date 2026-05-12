use crate::app_state::AppState;
use crate::websocket::websocket_handler::ConnectionState;
use log::info;
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
    }
}
