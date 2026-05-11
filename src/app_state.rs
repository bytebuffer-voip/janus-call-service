use crate::call::call_flow::supervisor::CallSupervisor;
use crate::config::config::Config;
use crate::network::sip_transport::SipTransport;
use std::sync::Arc;

pub struct AppState {
    pub config: Config,
    pub call_supervisor: Arc<CallSupervisor>,
    pub sip_transport: Arc<SipTransport>,
}

impl AppState {
    pub fn new(config: Config, sip_transport: Arc<SipTransport>) -> AppState {
        Self {
            config,
            call_supervisor: Arc::new(CallSupervisor::new()),
            sip_transport,
        }
    }
}
