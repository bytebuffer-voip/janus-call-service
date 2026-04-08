use serde::Deserialize;
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
pub struct Config {
    pub app_name: String,
    pub port: u16,
    pub jwt_key: String,
    pub expire_verify_token_ms: i64,
    pub mongodb: MongoConfig,
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
