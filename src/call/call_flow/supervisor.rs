use crate::app_state::AppState;
use crate::call::call_flow::call_actor::CallActor;
use crate::call::call_flow::call_model::{Call, CallEvent};
use crate::websocket::ws_connection::ConnectionState;
use dashmap::DashMap;
use log::info;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;

#[derive(Debug)]
pub enum SupervisorCommand {
    StopCall(String),
}

pub struct CallHandle {
    pub tx: mpsc::Sender<CallEvent>,
    pub task: JoinHandle<()>,
}

pub struct CallSupervisor {
    calls: DashMap<String, CallHandle>,
}

impl CallSupervisor {
    pub fn new() -> Self {
        Self {
            calls: DashMap::new(),
        }
    }

    pub async fn start_call<F>(
        self: Arc<Self>,
        app_state: Arc<AppState>,
        conn_state: Arc<ConnectionState>,
        call_id: &str,
        make_cal_func: F,
    ) -> mpsc::Sender<CallEvent>
    where
        F: FnOnce(mpsc::Sender<SupervisorCommand>) -> Call + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(512);
        let (api_tx, mut api_rx) = mpsc::channel(32);
        let call = make_cal_func(api_tx);
        let actor = CallActor::new(call_id.to_string(), rx, tx.clone(), call);
        let task = tokio::spawn(actor.run());
        let handle = CallHandle {
            tx: tx.clone(),
            task,
        };
        self.calls.insert(call_id.to_string(), handle);
        let supervisor_clone = Arc::clone(&self);
        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                match cmd {
                    SupervisorCommand::StopCall(call_id) => {
                        info!("Stopping call {}", call_id);
                        supervisor_clone.stop_call(&call_id).await;
                    }
                }
            }
        });
        if let Err(e) = tx.send(CallEvent::Start).await {
            info!("Error sending CallEvent::Start: {:?}", e);
        }
        tx
    }

    async fn stop_call(&self, call_id: &str) {
        if let Some((_, handle)) = self.calls.remove(call_id) {
            if let Err(e) = handle.tx.send(CallEvent::Stop).await {
                info!("Err: {:?}", e);
            }
            let mut task = handle.task;
            if timeout(Duration::from_secs(3), &mut task).await.is_err() {
                info!("Call {} did not stop in time, aborting task", call_id);
                task.abort();
            }
        }
    }

    pub fn get_call_tx(&self, call_id: &str) -> Option<mpsc::Sender<CallEvent>> {
        self.calls.get(call_id).map(|handle| handle.tx.clone())
    }
}
