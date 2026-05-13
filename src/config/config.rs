use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::{env, fs};

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub db_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Janus {
    pub http_uri: String,
    pub admin_uri: String,
    pub api_secret: String,
    pub admin_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SipTransportConfig {
    pub port: u16,
    pub public_ip: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KamailioConfig {
    pub host: String,
    pub port: u16,
    pub http_uri: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

impl KamailioConfig {
    pub fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let ip: IpAddr = self.host.parse()?;
        Ok(SocketAddr::new(ip, self.port))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app_name: String,
    pub port: u16,
    pub jwt_key: String,
    pub expire_verify_token_ms: i64,
    pub mongodb: MongoConfig,
    pub janus: Janus,
    pub sip_transport: SipTransportConfig,
    pub kamailio: KamailioConfig,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    pub fn global_config() -> &'static Config {
        CONFIG.get().unwrap_or_else(|| {
            panic!("Config not initialized. Please call Config::init() before accessing the global config.");
        })
    }

    fn expand_env(content: &str) -> String {
        let mut result = content.to_string();
        for (key, value) in env::vars() {
            let pattern = format!("${{{}}}", key);
            result = result.replace(&pattern, &value);
        }
        result
    }

    pub fn load() {
        let text = fs::read_to_string("./config/config.yml").unwrap();
        let expanded = Self::expand_env(&text);
        let cfg: Config = serde_yaml::from_str(&expanded).unwrap();
        CONFIG.set(cfg).unwrap();
    }
}
