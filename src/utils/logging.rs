use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use env_logger::Env;

static LOG_ENABLED: AtomicBool = AtomicBool::new(true);
static INIT: Once = Once::new();

/// Inicializa o logger, se ainda não foi inicializado.
pub fn init_logging() {
    INIT.call_once(|| {
        if LOG_ENABLED.load(Ordering::Relaxed) {
            env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
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