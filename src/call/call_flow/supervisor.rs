use crate::app_state::AppState;
use crate::call::call_flow::call_actor::CallActor;
use crate::call::call_flow::call_model::{Call, CallEvent};
use crate::websocket::ws_connection::ConnectionState;
use dashmap::DashMap;
use log::{debug, info};
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
    pub janus_handles: Vec<String>,
    // for sip calls
    pub sip_pending_trans: Vec<String>,
    pub dialog_ids: Vec<String>,
}

pub struct CallSupervisor {
    calls: DashMap<String, CallHandle>,
    janus_handle_map: DashMap<String, String>,
    // for sip call
    sip_pending_trans: DashMap<String, String>,
    dialog_ids: DashMap<String, String>,
}

impl CallSupervisor {
    pub fn new() -> Self {
        Self {
            calls: DashMap::new(),
            janus_handle_map: DashMap::new(),
            sip_pending_trans: DashMap::new(),
            dialog_ids: DashMap::new(),
        }
    }

    pub async fn start_call<F>(
        self: Arc<Self>,
        app_state: Arc<AppState>,
        conn_state: Arc<ConnectionState>,
        call_id: &str,
        janus_handle_key: Option<String>,
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
            janus_handles: vec![],
            sip_pending_trans: vec![],
            dialog_ids: vec![],
        };
        self.calls.insert(call_id.to_string(), handle);
        if let Some(janus_handle_key) = janus_handle_key {
            self.add_janus_handle(call_id, &janus_handle_key);
        }
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

            handle.janus_handles.iter().for_each(|id| {
                debug!("Removing janus handle id {}", id);
                self.janus_handle_map.remove(id);
            });

            handle.sip_pending_trans.iter().for_each(|tx| {
                debug!("Removing sip pending transaction id {}", tx);
                self.sip_pending_trans.remove(tx);
            });

            handle.dialog_ids.iter().for_each(|id| {
                debug!("Removing dialog id {}", id);
                self.dialog_ids.remove(id);
            });

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

    pub fn get_call_tx_by_janus_handle_id(
        &self,
        janus_handle_id: &str,
    ) -> Option<mpsc::Sender<CallEvent>> {
        self.janus_handle_map
            .get(janus_handle_id)
            .and_then(|call_id| self.calls.get(call_id.value()).map(|call| call.tx.clone()))
    }

    pub fn add_janus_handle(&self, call_id: &str, janus_handle_id: &str) {
        if let Some(mut call_handle) = self.calls.get_mut(call_id) {
            let exists = call_handle
                .janus_handles
                .iter()
                .any(|x| x == janus_handle_id);
            if !exists {
                call_handle.janus_handles.push(janus_handle_id.to_string());
            }
            self.janus_handle_map
                .insert(janus_handle_id.to_string(), call_id.to_string());
        }
    }

    pub fn remove_janus_handle_id(&self, call_id: &str, janus_handle_id: &str) {
        if let Some(mut call_handle) = self.calls.get_mut(call_id) {
            call_handle.janus_handles.retain(|x| x != janus_handle_id);
            self.janus_handle_map.remove(janus_handle_id);
        }
    }

    pub fn add_sip_pending_tran(&self, call_id: &str, pending_trans_id: &str) {
        if let Some(mut call_handle) = self.calls.get_mut(call_id) {
            let exists = call_handle
                .sip_pending_trans
                .iter()
                .any(|x| x == pending_trans_id);
            if !exists {
                call_handle
                    .sip_pending_trans
                    .push(pending_trans_id.to_string());
            }
            self.sip_pending_trans
                .insert(pending_trans_id.to_string(), call_id.to_string());
        }
    }

    pub fn get_call_tx_by_sip_pending_tran(
        &self,
        pending_trans_id: &str,
    ) -> Option<mpsc::Sender<CallEvent>> {
        self.sip_pending_trans
            .get(pending_trans_id)
            .and_then(|call_id| self.calls.get(call_id.value()).map(|call| call.tx.clone()))
    }

    // dialog
    pub fn add_dialog(&self, call_id: &str, dialog_id: &str) {
        if let Some(mut call_handle) = self.calls.get_mut(call_id) {
            let exists = call_handle.dialog_ids.iter().any(|x| x == dialog_id);
            if !exists {
                call_handle.dialog_ids.push(dialog_id.to_string());
            }
            self.dialog_ids
                .insert(dialog_id.to_string(), call_id.to_string());
        }
    }

    pub fn get_call_tx_by_dialog_id(&self, dialog_id: &str) -> Option<mpsc::Sender<CallEvent>> {
        self.dialog_ids
            .get(dialog_id)
            .and_then(|call_id| self.calls.get(call_id.value()).map(|call| call.tx.clone()))
    }
}
