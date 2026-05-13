use crate::call::app_to_app_call::AppToAppCall;
use crate::call::sip_to_app_call::SipToAppCall;
use crate::websocket::ws_connection::ClientInfo;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimerType {
    WaitSDPTimeout,
    ResendIncomingCall,
    CheckRoomTimer,
    JanusKeepalive,
}

#[derive(Debug, Clone)]
pub enum WebsocketEvent {
    OnSDP {
        client_info: ClientInfo,
        sdp: String,
    },
    OnICECandidate {
        client_info: ClientInfo,
        candidate: String,
        sdp_mline_index: Option<i64>,
        sdp_mid: Option<String>,
    },
    OnAnswer {
        client_info: ClientInfo,
        sdp: String,
        code: i64,
    },
    OnICECandidateCompleted {
        client_info: ClientInfo,
    },
    EndCall(ClientInfo),
    InCallResp {
        client_info: ClientInfo,
    },
}

#[derive(Debug, Clone)]
pub enum CallEvent {
    Start,
    Websocket(WebsocketEvent),
    JanusEvent(Value),
    Timer(TimerType),
    StartTimer(TimerType, Duration),
    StopTimer(TimerType),
    Stop,
}

#[derive(Debug, Clone)]
pub enum CallTimerAction {
    Start(TimerType, Duration),
    Cancel(TimerType),
    CancelAll,
    StopCall,
    None,
}

#[derive(Debug)]
pub enum Call {
    AppToApp(AppToAppCall),
    SIPToApp(SipToAppCall),
}

impl Call {
    pub async fn on_event(&mut self, event: CallEvent) {
        match self {
            Call::AppToApp(c) => c.on_event(event).await,
            Call::SIPToApp(c) => c.on_event(event).await,
        }
    }

    pub async fn cleanup(&mut self) {
        match self {
            Call::AppToApp(c) => c.cleanup().await,
            Call::SIPToApp(c) => c.cleanup().await,
        }
    }

    pub async fn on_timer(&mut self, timer: TimerType) -> CallTimerAction {
        match self {
            Call::AppToApp(c) => c.on_timer(timer).await,
            Call::SIPToApp(c) => c.on_timer(timer).await,
        }
    }
}
