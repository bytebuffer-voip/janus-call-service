use crate::call::call_flow::supervisor::CallSupervisor;
use crate::config::config::Config;
use std::sync::Arc;

pub struct AppState {
    pub config: Config,
    pub call_supervisor: Arc<CallSupervisor>,
}

impl AppState {
    pub fn new(config: Config) -> AppState {
        Self {
            config,
            call_supervisor: Arc::new(CallSupervisor::new()),
        }
    }
}
