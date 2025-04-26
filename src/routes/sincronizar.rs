use crate::models::user::User;
use sqlx::{Pool, Sqlite, Transaction};
use std::collections::{HashMap, HashSet};
use tokio::sync::{Mutex, broadcast};
use std::sync::Arc;
use std::process::Command;
use thiserror::Error;
use crate::utils::user_utils::{remover_uuids_xray, remover_uuid_v2ray, adicionar_usuario_sistema};
use std::fs;
use serde_json::Value;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use crate::utils::restart_xray::reiniciar_xray;
use futures::future::{join_all, timeout};
use std::time::Duration;
use log::{info, error, warn};
use tokio::time::sleep;

const BATCH_SIZE: usize = 50;
const MAX_RETRIES: u32 = 3;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Erro ao verificar usuário: {0}")]
    VerificarUsuario(String),
    #[error("Erro ao inserir usuário no banco de dados: {0}")]
    InserirUsuarioBanco(String),
    #[error("Timeout na operação: {0}")]
    Timeout(String),
    #[error("Erro na transação do banco: {0}")]
    TransacaoFalhou(String)
}

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
        let _ = self.tx.send(self.metrics.clone());
    }
}

// Função principal que recebe a lista e inicia o processamento em background
pub async fn sincronizar_usuarios(db: Database, pool: &Pool<Sqlite>, usuarios: Vec<User>) -> Result<String, SyncError> {
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
async fn with_retry<F, T, E>(operation: F) -> Result<T, E>
where
    F: Fn() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send + Sync,
    T: Send,
    E: std::fmt::Display + Send,
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
                    return Err("Timeout nas operações".into());
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
    pool: &Pool<Sqlite>, 
    usuarios: Vec<User>,
    sync_status: Arc<Mutex<SyncStatus>>,
    config_cache: Arc<Mutex<ConfigCache>>
) -> Result<(), SyncError> {
    let mut db = db.lock().await;
    let mut transaction = pool.begin().await
        .map_err(|e| SyncError::TransacaoFalhou(e.to_string()))?;

    // Buscar todos os usuários atuais do banco usando a transação
    let usuarios_atuais: Vec<User> = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&mut *transaction)
        .await
        .map_err(|e| SyncError::VerificarUsuario(e.to_string()))?;

    // Processamento em chunks otimizado
    let usuarios_para_remover: Vec<_> = usuarios_atuais
        .iter()
        .filter(|user_atual| !usuarios.iter().any(|u| u.login == user_atual.login))
        .collect();

    let usuarios_para_adicionar: Vec<_> = usuarios
        .iter()
        .filter(|user_novo| !usuarios_atuais.iter().any(|u| u.login == user_novo.login))
        .collect();

    // Processamento de remoções em paralelo com controle de erros
    for chunk in usuarios_para_remover.chunks(BATCH_SIZE) {
        let mut tasks = Vec::new();
        let mut logins_to_delete = Vec::new();

        for user in chunk {
            let user = user.clone();
            tasks.push(tokio::spawn(async move {
                with_retry(|| Box::pin(async move {
                    match user.tipo.as_str() {
                        "xray" => {
                            if let Some(uuid) = &user.uuid {
                                remover_uuids_xray(&vec![uuid.clone()]).await;
                            }
                        },
                        _ => {
                            if let Some(uuid) = &user.uuid {
                                remover_uuid_v2ray(uuid).await;
                            }
                        }
                    }
                    
                    let _ = Command::new("pkill").args(["-u", &user.login]).status();
                    let _ = Command::new("userdel").arg(&user.login).status();
                    Ok::<_, String>(user.login.clone())
                })).await
            }));
            logins_to_delete.push(user.login.clone());
        }

        // Processa resultados e atualiza métricas
        for result in join_all(tasks).await {
            match result {
                Ok(Ok(login)) => {
                    sync_status.lock().await.update(1, None);
                    info!("Usuário {} removido com sucesso", login);
                },
                Ok(Err(e)) => {
                    sync_status.lock().await.update(0, Some(e.to_string()));
                    error!("Erro ao remover usuário: {}", e);
                },
                Err(e) => {
                    sync_status.lock().await.update(0, Some(e.to_string()));
                    error!("Erro na task de remoção: {}", e);
                }
            }
        }

        // Remove do banco em lote usando a transação
        if !logins_to_delete.is_empty() {
            let placeholders = std::iter::repeat("?")
                .take(logins_to_delete.len())
                .collect::<Vec<_>>()
                .join(",");
            
            sqlx::query(&format!("DELETE FROM users WHERE login IN ({})", placeholders))
                .bind_all(logins_to_delete)
                .execute(&mut *transaction)
                .await
                .map_err(|e| SyncError::InserirUsuarioBanco(e.to_string()))?;
        }
    }

    // Processamento de adições em paralelo com controle de erros
    for chunk in usuarios_para_adicionar.chunks(BATCH_SIZE) {
        let mut tasks = Vec::new();

        for user in chunk {
            let user = user.clone();
            tasks.push(tokio::spawn(async move {
                with_retry(|| Box::pin(async move {
                    adicionar_usuario_sistema(
                        &user.login,
                        &user.senha,
                        user.dias as u32,
                        user.limite as u32
                    ).map_err(|e| e.to_string())
                })).await
            }));

            // Insere no banco usando a transação
            sqlx::query(
                "INSERT OR REPLACE INTO users (login, senha, dias, limite, uuid, tipo, dono, byid) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&user.login)
            .bind(&user.senha)
            .bind(user.dias as i64)
            .bind(user.limite as i64)
            .bind(&user.uuid)
            .bind(&user.tipo)
            .bind(&user.dono)
            .bind(user.byid as i64)
            .execute(&mut *transaction)
            .await
            .map_err(|e| SyncError::InserirUsuarioBanco(e.to_string()))?;

            // Atualiza o cache em memória
            db.insert(user.login.clone(), user.clone());
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
    }

    // Commit da transação
    transaction.commit().await
        .map_err(|e| SyncError::TransacaoFalhou(e.to_string()))?;

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
