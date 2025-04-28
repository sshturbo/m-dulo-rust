use serde::Deserialize;
use std::fs;
use once_cell::sync::OnceCell;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub api_token: String,
    pub database_url: String,
    pub logs_enabled: Option<bool>,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

impl Config {
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config não inicializada")
    }
    pub fn load_from_file(path: &str) {
        let content = fs::read_to_string(path).expect("Falha ao ler config.json");
        let config: Config = serde_json::from_str(&content).expect("Falha ao parsear config.json");
        CONFIG.set(config).expect("Config já foi inicializada");
    }
} 