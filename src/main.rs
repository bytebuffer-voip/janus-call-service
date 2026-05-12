#![allow(dead_code)]
#![allow(unused_variables)]

use crate::app_state::AppState;
use crate::config::config::Config;
use crate::config::mongodb_cfg::get_mongo_client;
use crate::network::sip_transport;
use crate::network::sip_transport::SipTransport;
use crate::router::create_router;
use crate::websocket::ws_connection::ConnectionState;
use dotenv::dotenv;
use log::info;
use mongodb::bson::doc;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

mod app_state;
mod call;
mod config;
mod controller;
mod middlerware;
mod model;
mod network;
mod repo;
mod router;
mod service;
mod utils;
mod websocket;

#[tokio::main]
async fn main() {
    dotenv().ok();
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    Config::load();
    let app_name = &Config::global_config().app_name;
    info!("App: [{}] starting up...", app_name);

    let client = get_mongo_client().await.unwrap();
    client
        .database(&Config::global_config().mongodb.db_name)
        .run_command(doc! { "ping": 1 })
        .await
        .unwrap();

    let cfg = Config::global_config();
    let address = format!("0.0.0.0:{}", cfg.sip_transport.port);
    let transport = Arc::new(SipTransport::bind(&address).await.unwrap());

    let app_state = Arc::new(AppState::new(cfg.clone(), transport));
    let conn_state = Arc::new(ConnectionState::default());
    let router = create_router(app_state.clone(), conn_state.clone())
        .into_make_service_with_connect_info::<SocketAddr>();

    //
    run_tasks(app_state, conn_state).await.unwrap();

    let port = cfg.port;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    axum::serve(listener, router).await.unwrap();
}

async fn run_tasks(
    shared_state: Arc<AppState>,
    connection_state: Arc<ConnectionState>,
) -> anyhow::Result<()> {
    let app_state = shared_state.clone();
    let conn_state = connection_state.clone();
    tokio::spawn(async move {
        let _ = sip_transport::recv_loop(&app_state, &conn_state).await;
    });
    Ok(())
}
