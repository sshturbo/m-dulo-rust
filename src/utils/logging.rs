use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use env_logger::Env;
use log::{info, warn, error, LevelFilter};
use uuid::Uuid;

static LOG_ENABLED: AtomicBool = AtomicBool::new(true);
static INIT: Once = Once::new();

/// Inicializa o logger, se ainda não foi inicializado.
pub fn init_logging() {
    INIT.call_once(|| {
        if LOG_ENABLED.load(Ordering::Relaxed) {
            env_logger::Builder::from_env(Env::default().default_filter_or("info"))
                .format_timestamp_secs()
                .format(|buf, record| {
                    use std::io::Write;
                    writeln!(
                        buf,
                        "[{}] {}: {}",
                        record.level(),
                        record.target(),
                        record.args()
                    )
                })
                .filter(None, LevelFilter::Info)
                .write_style(env_logger::WriteStyle::Always)
                .init();
            
            info!("Sistema de logs iniciado");
        }
    });
}

/// Ativa os logs (chame antes de init_logging).
pub fn enable_logs() {
    LOG_ENABLED.store(true, Ordering::Relaxed);
}

/// Desativa os logs (chame antes de init_logging).
pub fn disable_logs() {
    LOG_ENABLED.store(false, Ordering::Relaxed);
}

pub fn log_proxy_nova_conexao(addr: &str) {
    println!("[PROXY] Nova conexão recebida de {}", addr);
    info!("[PROXY] Nova conexão recebida de {}", addr);
}

pub fn log_proxy_tipo_conexao(tipo: &str) {
    println!("[PROXY] Tipo de conexão detectada: {}", tipo);
    info!("[PROXY] Tipo de conexão detectada: {}", tipo);
}

pub fn log_proxy_uuid_invalido(uuid: &Uuid, addr: &str) {
    println!("[PROXY] UUID inválido {} de {}", uuid, addr);
    warn!("[PROXY] UUID inválido {} de {}", uuid, addr);
}

pub fn log_proxy_uuid_valido(uuid: &Uuid, addr: &str) {
    println!("[PROXY] UUID válido {} de {}", uuid, addr);
    info!("[PROXY] UUID válido {} de {}", uuid, addr);
}

pub fn log_proxy_conexao_estabelecida(uuid: &Uuid, tipo: &str) {
    println!("[PROXY] Conexão estabelecida - UUID: {}, Tipo: {}", uuid, tipo);
    info!("[PROXY] Conexão estabelecida - UUID: {}, Tipo: {}", uuid, tipo);
}

pub fn log_proxy_conexao_encerrada(uuid: &Uuid, motivo: &str) {
    println!("[PROXY] Conexão encerrada - UUID: {}, Motivo: {}", uuid, motivo);
    info!("[PROXY] Conexão encerrada - UUID: {}, Motivo: {}", uuid, motivo);
}

pub fn log_proxy_erro(erro: &str) {
    eprintln!("[PROXY] Erro: {}", erro);
    error!("[PROXY] Erro: {}", erro);
}

pub fn log_proxy_xray_conectado(uuid: &Uuid) {
    println!("[PROXY] Conectado ao Xray - UUID: {}", uuid);
    info!("[PROXY] Conectado ao Xray - UUID: {}", uuid);
} 