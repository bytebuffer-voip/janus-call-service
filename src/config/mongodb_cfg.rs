use crate::config::config::Config;
use log::info;
use mongodb::Client;
use mongodb::bson::doc;
use mongodb::options::{ClientOptions, ServerApi, ServerApiVersion};
use tokio::sync::OnceCell;

static MONGO_CLIENT: OnceCell<Client> = OnceCell::const_new();

pub async fn get_mongo_client() -> anyhow::Result<&'static Client> {
    MONGO_CLIENT.get_or_try_init(init_mongo_client).await
}

async fn init_mongo_client() -> anyhow::Result<Client> {
    let cfg = Config::global_config();
    let mongodb = cfg.mongodb.clone();
    let client_uri = format!(
        "mongodb://{}:{}@{}:{}/{}?maxPoolSize=20&w=majority",
        mongodb.username, mongodb.password, mongodb.host, mongodb.port, mongodb.db_name
    );
    info!("Connecting to MongoDB ...");
    let mut client_options = ClientOptions::parse(client_uri).await?;
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    let client = Client::with_options(client_options)?;
    client
        .database("admin")
        .run_command(doc! { "ping": 1 })
        .await?;
    info!("Pinged your deployment. You successfully connected to MongoDB!");
    Ok(client)
}
