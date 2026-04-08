// use crate::app_state::AppState;
// use crate::call::call_flow::call_model::{CallEvent, TimerType, WebsocketEvent};
// use crate::call::call_flow::call_type::peer_to_peer_routing::state::p2p_call_state::{
//     P2PCallStateHandler, P2PStateAction,
// };
// use crate::call::call_flow::call_type::peer_to_peer_routing::state::p2p_end_state::P2PEndState;
// use crate::call::call_flow::call_type::peer_to_peer_routing::state::p2p_waiting_caller_sdp_state::P2PWaitingCallerSdpState;
// use crate::call::call_flow::supervisor::SupervisorCommand;
// use crate::model::user::User;
// use crate::websocket::websocket_handler::ClientInfo;
// use crate::websocket::ws_connection::ConnectionState;
// use futures_util::future::BoxFuture;
// use log::info;
// use std::fmt;
// use std::fmt::{Debug, Formatter};
// use std::sync::Arc;
// use tokio::sync::mpsc::Sender;
//
// pub struct PeerToPeerCallParams {
//     pub caller_client_info: ClientInfo,
//     pub caller: String,
//     pub caller_user: User,
//     pub callee_user: User,
// }
//
// pub struct PeerToPeerCall {
//     pub app_state: Arc<AppState>,
//     pub conn_state: Arc<ConnectionState>,
//     pub call_id: String,
//     pub params: PeerToPeerCallParams,
//     pub api_tx: Sender<SupervisorCommand>,
//     state: Option<Box<dyn P2PCallStateHandler>>,
// }
//
// impl Debug for PeerToPeerCall {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         f.debug_struct("PeerToPeerCall").finish()
//     }
// }
//
// impl PeerToPeerCall {
//     pub fn new(
//         app_state: Arc<AppState>,
//         conn_state: Arc<ConnectionState>,
//         call_id: String,
//         params: PeerToPeerCallParams,
//         api_tx: Sender<SupervisorCommand>,
//     ) -> Self {
//         Self {
//             app_state,
//             conn_state,
//             call_id,
//             params,
//             api_tx,
//             state: None,
//         }
//     }
//
//     async fn apply_action(&mut self, action: P2PStateAction) -> anyhow::Result<()> {
//         match action {
//             P2PStateAction::Stay => Ok(()),
//             P2PStateAction::Transition(next) => self.transition_to(next).await,
//             P2PStateAction::Hangup { reason } => {
//                 self.transition_to(Box::new(P2PEndState {
//                     reason: reason.into(),
//                 }))
//                 .await
//             }
//         }
//     }
//
//     fn transition_to(
//         &mut self,
//         mut next: Box<dyn P2PCallStateHandler>,
//     ) -> BoxFuture<'_, anyhow::Result<()>> {
//         Box::pin(async move {
//             loop {
//                 let prev_name = self
//                     .state
//                     .as_ref()
//                     .map(|s| s.get_name())
//                     .unwrap_or("<none>");
//                 if let Some(mut prev) = self.state.take() {
//                     prev.on_exit(self).await?;
//                 }
//                 let next_name = next.get_name();
//                 self.state = Some(next);
//                 info!("Call {} {} → {}", self.call_id, prev_name, next_name);
//                 if let Some(mut curr) = self.state.take() {
//                     let action = curr.on_enter(self).await;
//                     self.state = Some(curr);
//                     match action? {
//                         P2PStateAction::Stay => return Ok(()),
//                         P2PStateAction::Transition(n) => {
//                             next = n;
//                             continue;
//                         }
//                         P2PStateAction::Hangup { reason } => {
//                             next = Box::new(P2PEndState {
//                                 reason: reason.into(),
//                             });
//                             continue;
//                         }
//                     }
//                 } else {
//                     return Ok(());
//                 }
//             }
//         })
//     }
//
//     pub async fn on_event(&mut self, event: CallEvent) {
//         match event {
//             CallEvent::Start => {
//                 if let Err(e) = self
//                     .transition_to(Box::new(P2PWaitingCallerSdpState::new()))
//                     .await
//                 {
//                     info!("Error transitioning to P2PWaitingCallerSdpState: {:?}", e);
//                     let _ = self
//                         .transition_to(Box::new(P2PEndState {
//                             reason: "Failed to start call".into(),
//                         }))
//                         .await;
//                 }
//             }
//             CallEvent::Websocket(ref e) => {
//                 let _ = self.process_websocket_event(e).await;
//                 if let Some(mut state) = self.state.take() {
//                     let next_action = state.on_event(self, event).await;
//                     self.state = Some(state);
//                     if let Ok(next_action) = next_action {
//                         let _ = self.apply_action(next_action).await;
//                     }
//                 }
//             }
//             _ => {}
//         }
//     }
//
//     async fn process_websocket_event(&mut self, evt: &WebsocketEvent) -> anyhow::Result<()> {
//         match evt {
//             WebsocketEvent::EndCall(info) => {
//                 let end_state = P2PEndState::new("Hangup".to_string());
//                 self.apply_action(P2PStateAction::Transition(Box::new(end_state)))
//                     .await?;
//             }
//             _ => {}
//         }
//         Ok(())
//     }
//
//     pub async fn start_timer(&self, ty: TimerType, secs: u64) {
//         if let Some(tx) = self.app_state.call_supervisor.get_call_tx(&self.call_id) {
//             let _ = tx
//                 .send(CallEvent::StartTimer(
//                     ty,
//                     std::time::Duration::from_secs(secs),
//                 ))
//                 .await;
//         }
//     }
//
//     pub async fn stop_timer(&self, ty: TimerType) {
//         if let Some(tx) = self.app_state.call_supervisor.get_call_tx(&self.call_id) {
//             let _ = tx.send(CallEvent::StopTimer(ty)).await;
//         }
//     }
//
//     pub async fn cleanup(&mut self) {
//         if let Some(mut state) = self.state.take() {
//             let next_action = state.call_end(self).await;
//             self.state = Some(state);
//             if let Err(e) = self.apply_action(next_action).await {
//                 info!("Error transitioning to P2PCallState: {:?}", e);
//             }
//         }
//     }
//
//     pub async fn on_timer(&mut self, timer: TimerType) {
//         if let Some(mut state) = self.state.take() {
//             let res = state.on_timer(self, timer).await;
//             self.state = Some(state);
//             if let Ok(state_action) = res {
//                 let _ = self.apply_action(state_action).await;
//             }
//         }
//     }
// }
