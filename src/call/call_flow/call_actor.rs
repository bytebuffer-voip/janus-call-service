use crate::call::call_flow::call_model::{Call, CallEvent, TimerType};
use log::info;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct CallActor {
    pub call_id: String,
    pub rx: mpsc::Receiver<CallEvent>,
    pub supervisor_tx: mpsc::Sender<CallEvent>,
    pub timers: HashMap<TimerType, tokio::task::JoinHandle<()>>,
    pub call: Call,
}

impl CallActor {
    pub fn new(
        call_id: String,
        rx: mpsc::Receiver<CallEvent>,
        supervisor_tx: mpsc::Sender<CallEvent>,
        call: Call,
    ) -> Self {
        Self {
            call_id,
            rx,
            supervisor_tx,
            timers: HashMap::new(),
            call,
        }
    }

    pub async fn run(mut self) {
        while let Some(event) = self.rx.recv().await {
            match &event {
                CallEvent::Stop => {
                    self.cancel_all_timers();
                    self.call.cleanup().await;
                    return;
                }
                CallEvent::StartTimer(timer_type, duration) => {
                    self.start_timer(*duration, *timer_type);
                    continue;
                }
                CallEvent::StopTimer(timer_type) => {
                    self.cancel_timer(timer_type);
                    continue;
                }
                CallEvent::Timer(timer_type) => {
                    self.call.on_timer(*timer_type).await;
                    continue;
                }
                _ => {}
            }
            self.call.on_event(event).await;
        }
        info!("Stopping call actor {}", self.call_id);
        self.cancel_all_timers();
        self.call.cleanup().await;
    }

    fn start_timer(&mut self, dur: Duration, timer: TimerType) {
        if let Some(h) = self.timers.remove(&timer) {
            h.abort();
        }
        let tx = self.supervisor_tx.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(dur).await;
            if let Err(e) = tx.send(CallEvent::Timer(timer)).await {
                info!("Error sending CallEvent::Timer {:?}: {:?}", timer, e);
            }
        });
        self.timers.insert(timer, handle);
    }

    fn cancel_timer(&mut self, timer: &TimerType) {
        if let Some(h) = self.timers.remove(timer) {
            h.abort();
        }
    }

    fn cancel_all_timers(&mut self) {
        for (_, handle) in self.timers.drain() {
            handle.abort();
        }
    }
}

impl Drop for CallActor {
    fn drop(&mut self) {
        info!("call actor dropped: {}", self.call_id);
        for (_, handle) in self.timers.drain() {
            handle.abort();
        }
    }
}
