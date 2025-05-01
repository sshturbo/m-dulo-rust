use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_token: String,
    pub database_url: String,
    pub logs_enabled: Option<bool>,
    pub cloudflare_api_key: String,
    pub cloudflare_domain: String,
    pub xray_port: u16,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    pub fn load_from_file(path: &str) {
        let contents = fs::read_to_string(path)
            .expect("Erro ao ler arquivo config.json");
        
        let config: Config = serde_json::from_str(&contents)
            .expect("Erro ao parsear config.json");
            
        CONFIG.set(config)
            .expect("Erro ao definir configuração global");
    }
    
    pub fn get() -> &'static Config {
        CONFIG.get()
            .expect("Configuração não foi carregada")
    }
} 