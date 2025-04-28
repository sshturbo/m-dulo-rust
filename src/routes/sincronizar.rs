use crate::models::user::User;
use sqlx::PgPool;
use std::collections::HashSet;
use tokio::sync::{Mutex, broadcast};
use std::sync::Arc;
use std::process::Command;
use thiserror::Error;
use crate::utils::user_utils::adicionar_usuario_sistema;
use std::fs;
use serde_json::Value;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use crate::utils::restart_xray::reiniciar_xray;
use futures::future::join_all;
use tokio::time::{timeout, sleep};
use std::time::Duration;
use log::{info, error, warn};
use std::future::Future;
use crate::routes::criar::Database;

const BATCH_SIZE: usize = 20;
const MAX_RETRIES: u32 = 3;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Error, Debug)]
pub enum SyncError {}

// Cache de configurações
#[derive(Clone)]
struct ConfigCache {
    xray_config: Option<Value>,
    v2ray_config: Option<Value>,
    last_update: std::time::SystemTime,
}

impl ConfigCache {
    fn new() -> Self {
        Self {
            xray_config: None,
            v2ray_config: None,
            last_update: std::time::SystemTime::now(),
        }
    }

    fn need_refresh(&self) -> bool {
        self.last_update.elapsed().unwrap_or_default() > Duration::from_secs(300)
    }
}

#[derive(Clone)]
struct ProcessingMetrics {
    total_users: usize,
    processed_users: usize,
    errors: Vec<String>,
}

// Status do processamento
#[derive(Clone)]
struct SyncStatus {
    metrics: ProcessingMetrics,
    tx: broadcast::Sender<ProcessingMetrics>,
}

impl SyncStatus {
    fn new(total_users: usize) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            metrics: ProcessingMetrics {
                total_users,
                processed_users: 0,
                errors: Vec::new(),
            },
            tx,
        }
    }

    fn update(&mut self, processed: usize, error: Option<String>) {
        self.metrics.processed_users += processed;
        if let Some(err) = error {
            self.metrics.errors.push(err);
        }
        
        // Usando o campo total_users para calcular o progresso
        let progress_percentage = (self.metrics.processed_users as f64 / self.metrics.total_users as f64 * 100.0) as u32;
        info!("Progresso da sincronização: {}% ({}/{})", 
            progress_percentage,
            self.metrics.processed_users,
            self.metrics.total_users
        );
        
        let _ = self.tx.send(self.metrics.clone());
    }
}

// Função principal que recebe a lista e inicia o processamento em background
pub async fn sincronizar_usuarios(db: Database, pool: &PgPool, usuarios: Vec<User>) -> Result<String, SyncError> {
    let pool = pool.clone();
    let usuarios_len = usuarios.len();
    
    let sync_status = Arc::new(Mutex::new(SyncStatus::new(usuarios_len)));
    let config_cache = Arc::new(Mutex::new(ConfigCache::new()));
    
    info!("Iniciando sincronização de {} usuários", usuarios_len);
    
    // Inicia o processamento em background
    let status_clone = sync_status.clone();
    tokio::spawn(async move {
        if let Err(e) = processar_usuarios_em_lotes(db, &pool, usuarios, status_clone, config_cache).await {
            error!("Erro no processamento em background: {}", e);
        }
    });

    Ok(format!("Iniciado processamento de {} usuários em background", usuarios_len))
}

// Função auxiliar para retry de operações
async fn with_retry<F, T>(operation: F) -> Result<T, String>
where
    F: Fn() -> std::pin::Pin<Box<dyn Future<Output = Result<T, String>> + Send>> + Send + Sync,
    T: Send,
{
    let mut retries = 0;
    loop {
        match timeout(OPERATION_TIMEOUT, operation()).await {
            Ok(result) => match result {
                Ok(value) => return Ok(value),
                Err(e) => {
                    if retries >= MAX_RETRIES {
                        return Err(e);
                    }
                    warn!("Operação falhou (tentativa {}): {}", retries + 1, e);
                    retries += 1;
                    sleep(RETRY_DELAY).await;
                }
            },
            Err(_) => {
                if retries >= MAX_RETRIES {
                    error!("Timeout após {} tentativas", MAX_RETRIES);
                    return Err("Timeout nas operações".to_string());
                }
                warn!("Timeout (tentativa {})", retries + 1);
                retries += 1;
                sleep(RETRY_DELAY).await;
            }
        }
    }
}

// Função que processa os usuários em lotes
async fn processar_usuarios_em_lotes(
    db: Database, 
    pool: &PgPool, 
    usuarios: Vec<User>,
    sync_status: Arc<Mutex<SyncStatus>>,
    config_cache: Arc<Mutex<ConfigCache>>
) -> Result<(), SyncError> {
    let db = db.clone();

    for chunk in usuarios.chunks(BATCH_SIZE) {
        let mut tasks = Vec::new();
        for user in chunk {
            let pool = pool.clone();
            let db = db.clone();
            let user = user.clone();
            tasks.push(tokio::spawn(async move {
                let login = user.login.clone();
                let senha = user.senha.clone();
                let dias = user.dias;
                let limite = user.limite;
                // Remove o usuário do sistema operacional antes de criar, se existir
                if Command::new("id").arg(&login).status().map(|s| s.success()).unwrap_or(false) {
                    let _ = Command::new("pkill").args(["-u", &login]).status();
                    let _ = Command::new("userdel").arg(&login).status();
                }
                with_retry(|| {
                    let login = login.clone();
                    let senha = senha.clone();
                    let user = user.clone();
                    let pool = pool.clone();
                    let db = db.clone();
                    Box::pin(async move {
                        // 1. Cria usuário no SO
                        adicionar_usuario_sistema(
                            &login,
                            &senha,
                            dias as u32,
                            limite as u32
                        ).map_err(|e| e.to_string())?;
                        // 2. Insere no banco em transação
                        let mut transaction = pool.begin().await.map_err(|e| e.to_string())?;
                        sqlx::query(
                            "INSERT INTO users (login, senha, dias, limite, uuid, tipo, dono, byid) \
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)\
                             ON CONFLICT (login) DO UPDATE SET \
                                senha = EXCLUDED.senha,\
                                dias = EXCLUDED.dias,\
                                limite = EXCLUDED.limite,\
                                uuid = EXCLUDED.uuid,\
                                tipo = EXCLUDED.tipo,\
                                dono = EXCLUDED.dono,\
                                byid = EXCLUDED.byid"
                        )
                        .bind(&user.login)
                        .bind(&user.senha)
                        .bind(user.dias as i32)
                        .bind(user.limite as i32)
                        .bind(&user.uuid)
                        .bind(&user.tipo)
                        .bind(&user.dono)
                        .bind(user.byid as i32)
                        .execute(&mut *transaction)
                        .await
                        .map_err(|e| e.to_string())?;
                        transaction.commit().await.map_err(|e| e.to_string())?;
                        // 3. Atualiza cache em memória
                        let mut db = db.lock().await;
                        db.insert(user.login.clone(), user.clone());
                        Ok(())
                    })
                }).await
            }));
        }
        // Processa resultados e atualiza métricas
        for result in join_all(tasks).await {
            match result {
                Ok(Ok(_)) => {
                    sync_status.lock().await.update(1, None);
                },
                Ok(Err(e)) => {
                    sync_status.lock().await.update(0, Some(e.to_string()));
                    error!("Erro ao adicionar usuário: {}", e);
                },
                Err(e) => {
                    sync_status.lock().await.update(0, Some(e.to_string()));
                    error!("Erro na task de adição: {}", e);
                }
            }
        }
        // Adiciona um pequeno delay entre os lotes
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    // Atualiza configurações do Xray e V2Ray em paralelo
    let (xray_result, v2ray_result) = tokio::join!(
        atualizar_configs_xray(&usuarios, config_cache.clone()),
        atualizar_configs_v2ray(&usuarios, config_cache.clone())
    );
    if let Err(e) = xray_result {
        error!("Erro ao atualizar configurações Xray: {}", e);
        sync_status.lock().await.update(0, Some(e.to_string()));
    }
    if let Err(e) = v2ray_result {
        error!("Erro ao atualizar configurações V2Ray: {}", e);
        sync_status.lock().await.update(0, Some(e.to_string()));
    }
    Ok(())
}

// Funções auxiliares para atualizar configurações com cache
async fn atualizar_configs_xray(usuarios: &[User], config_cache: Arc<Mutex<ConfigCache>>) -> Result<(), String> {
    let config_path_xray = "/usr/local/etc/xray/config.json";
    let mut cache = config_cache.lock().await;

    if cache.need_refresh() || cache.xray_config.is_none() {
        if let Ok(content) = fs::read_to_string(config_path_xray) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                cache.xray_config = Some(json);
                cache.last_update = std::time::SystemTime::now();
            }
        }
    }

    if let Some(mut json) = cache.xray_config.clone() {
        let mut unique_ids = HashSet::new();
        let mut new_clients = Vec::new();

        for user in usuarios.iter().filter(|u| u.tipo == "xray") {
            if let Some(uuid) = &user.uuid {
                if !uuid.is_empty() && unique_ids.insert(uuid.clone()) {
                    new_clients.push(serde_json::json!({
                        "email": user.login,
                        "id": uuid,
                        "level": 0
                    }));
                }
            }
        }

        if let Some(inbounds) = json.get_mut("inbounds") {
            if let Some(inbound_array) = inbounds.as_array_mut() {
                for inbound in inbound_array {
                    if inbound["protocol"] == "vless" {
                        if let Some(settings) = inbound.get_mut("settings") {
                            settings["clients"] = serde_json::Value::Array(new_clients.clone());
                        }
                    }
                }
            }
        }

        // Salva as mudanças
        let tmp_path = "/usr/local/etc/xray/config.json.tmp";
        if let Ok(new_content) = serde_json::to_string_pretty(&json) {
            if fs::write(tmp_path, &new_content).is_ok() {
                if fs::rename(tmp_path, config_path_xray).is_ok() {
                    reiniciar_xray().await;
                    return Ok(());
                }
            }
        }
    }

    Err("Falha ao atualizar configuração do Xray".to_string())
}

async fn atualizar_configs_v2ray(usuarios: &[User], config_cache: Arc<Mutex<ConfigCache>>) -> Result<(), String> {
    let config_path_v2ray = "/etc/v2ray/config.json";
    let mut cache = config_cache.lock().await;

    if cache.need_refresh() || cache.v2ray_config.is_none() {
        if let Ok(content) = fs::read_to_string(config_path_v2ray) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                cache.v2ray_config = Some(json);
                cache.last_update = std::time::SystemTime::now();
            }
        }
    }

    if let Some(mut json) = cache.v2ray_config.clone() {
        let mut unique_ids = HashSet::new();
        let mut new_clients = Vec::new();

        for user in usuarios.iter().filter(|u| u.tipo == "v2ray") {
            if let Some(uuid) = &user.uuid {
                if !uuid.is_empty() && unique_ids.insert(uuid.clone()) {
                    new_clients.push(serde_json::json!({
                        "id": uuid,
                        "alterId": 0,
                        "email": user.login
                    }));
                }
            }
        }

        if let Some(inbounds) = json.get_mut("inbounds") {
            if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                if let Some(settings) = first_inbound.get_mut("settings") {
                    if let Some(clients) = settings.get_mut("clients") {
                        *clients = serde_json::Value::Array(new_clients);
                    }
                }
            }
        }

        let tmp_path = "/etc/v2ray/config.json.tmp";
        if let Ok(new_content) = serde_json::to_string_pretty(&json) {
            if fs::write(tmp_path, &new_content).is_ok() {
                if fs::rename(tmp_path, config_path_v2ray).is_ok() {
                    reiniciar_v2ray().await;
                    return Ok(());
                }
            }
        }
    }

    Err("Falha ao atualizar configuração do V2Ray".to_string())
}
