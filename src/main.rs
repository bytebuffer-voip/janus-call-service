#![allow(dead_code)]
#![allow(unused_variables)]

use crate::app_state::AppState;
use crate::config::config::Config;
use crate::config::mongodb_cfg::get_mongo_client;
use crate::router::create_router;
use crate::websocket::ws_connection::ConnectionState;
use dotenv::dotenv;
use log::info;
use mongodb::bson::doc;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

mod app_state;
mod config;
mod controller;
mod middlerware;
mod model;
mod repo;
mod router;
mod service;
mod utils;
mod websocket;
mod call;

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

    let app_state = Arc::new(AppState::new(Config::global_config().clone()));
    let conn_state = Arc::new(ConnectionState::default());
    let router =
        create_router(app_state, conn_state).into_make_service_with_connect_info::<SocketAddr>();

    let port = Config::global_config().port;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    axum::serve(listener, router).await.unwrap();
}
