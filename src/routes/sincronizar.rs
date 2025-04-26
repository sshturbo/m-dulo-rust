use crate::models::user::User;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::process::Command;
use thiserror::Error;
use crate::utils::user_utils::{remover_uuids_xray, remover_uuid_v2ray, adicionar_usuario_sistema};
use std::fs;
use serde_json::Value;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use crate::utils::restart_xray::reiniciar_xray;
use futures::future::join_all;

const BATCH_SIZE: usize = 50; // Tamanho do lote para processamento

pub type Database = Arc<Mutex<HashMap<String, User>>>;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Erro ao verificar usuário: {0}")]
    VerificarUsuario(String),
    #[error("Erro ao inserir usuário no banco de dados: {0}")]
    InserirUsuarioBanco(String)
}

// Função principal que recebe a lista e inicia o processamento em background
pub async fn sincronizar_usuarios(db: Database, pool: &Pool<Sqlite>, usuarios: Vec<User>) -> Result<String, SyncError> {
    let pool = pool.clone();
    let usuarios_len = usuarios.len();
    
    // Inicia o processamento em background
    tokio::spawn(async move {
        if let Err(e) = processar_usuarios_em_lotes(db, &pool, usuarios).await {
            eprintln!("Erro no processamento em background: {}", e);
        }
    });

    // Retorna resposta imediata
    Ok(format!("Iniciado processamento de {} usuários em background", usuarios_len))
}

// Função que processa os usuários em lotes
async fn processar_usuarios_em_lotes(db: Database, pool: &Pool<Sqlite>, usuarios: Vec<User>) -> Result<(), SyncError> {
    let mut db = db.lock().await;

    // Buscar todos os usuários atuais do banco
    let usuarios_atuais: Vec<User> = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool)
        .await
        .map_err(|e| SyncError::VerificarUsuario(e.to_string()))?;

    // Processa remoções em paralelo com chunks
    let chunks_remocao: Vec<_> = usuarios_atuais
        .iter()
        .filter(|user_atual| !usuarios.iter().any(|u| u.login == user_atual.login))
        .collect::<Vec<_>>()
        .chunks(BATCH_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();

    for chunk in chunks_remocao {
        let mut tasks = Vec::new();
        let mut logins_to_delete = Vec::new();

        // Processa remoções em paralelo
        for user_atual in chunk {
            let uuid = user_atual.uuid.clone();
            let login = user_atual.login.clone();
            let tipo = user_atual.tipo.clone();
            
            tasks.push(tokio::spawn(async move {
                match tipo.as_str() {
                    "xray" => {
                        if let Some(uuid) = uuid {
                            remover_uuids_xray(&vec![uuid]).await;
                        }
                    },
                    _ => {
                        if let Some(uuid) = uuid {
                            remover_uuid_v2ray(&uuid).await;
                        }
                    }
                }
                let _ = Command::new("pkill").args(["-u", &login]).status();
                let _ = Command::new("userdel").arg(&login).status();
                login
            }));
            logins_to_delete.push(user_atual.login.clone());
        }

        // Aguarda todas as remoções do lote
        join_all(tasks).await;

        // Remove do banco em lote
        if !logins_to_delete.is_empty() {
            let placeholders = std::iter::repeat("?")
                .take(logins_to_delete.len())
                .collect::<Vec<_>>()
                .join(",");
            
            let query = format!("DELETE FROM users WHERE login IN ({})", placeholders);
            let mut query = sqlx::query(&query);
            for login in &logins_to_delete {
                query = query.bind(login);
            }
            query.execute(pool).await.ok();
        }
    }

    // Processa adições/atualizações em paralelo com chunks
    let chunks_adicao: Vec<_> = usuarios
        .chunks(BATCH_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();

    for chunk in chunks_adicao {
        let mut tasks = Vec::new();
        let mut db_tasks = Vec::new();

        // Processa criação de usuários em paralelo
        for user in &chunk {
            let user_clone = user.clone();
            tasks.push(tokio::spawn(async move {
                adicionar_usuario_sistema(
                    &user_clone.login,
                    &user_clone.senha,
                    user_clone.dias as u32,
                    user_clone.limite as u32
                )
            }));

            // Prepara inserções no banco
            db_tasks.push(
                sqlx::query(
                    "INSERT OR REPLACE INTO users (login, senha, dias, limite, uuid, tipo, dono, byid) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&user.login)
                .bind(&user.senha)
                .bind(user.dias as i64)
                .bind(user.limite as i64)
                .bind(&user.uuid)
                .bind(&user.tipo)
                .bind(&user.dono)
                .bind(user.byid as i64)
                .execute(pool)
            );

            // Atualiza o cache em memória
            db.insert(user.login.clone(), user.clone());
        }

        // Executa em paralelo a criação dos usuários e inserções no banco
        let (system_results, db_results) = tokio::join!(
            join_all(tasks),
            join_all(db_tasks)
        );

        // Verifica erros nas operações do banco
        for result in db_results {
            if let Err(e) = result {
                eprintln!("Erro ao inserir no banco: {}", e);
            }
        }

        // Pequeno delay entre lotes para não sobrecarregar
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Atualiza as configurações do Xray e V2Ray em paralelo
    let (xray_result, v2ray_result) = tokio::join!(
        atualizar_configs_xray(&usuarios),
        atualizar_configs_v2ray(&usuarios)
    );

    Ok(())
}

// Função auxiliar para atualizar configurações do Xray
async fn atualizar_configs_xray(usuarios: &[User]) {
    let config_path_xray = "/usr/local/etc/xray/config.json";
    if std::path::Path::new(config_path_xray).exists() {
        if let Ok(content) = fs::read_to_string(config_path_xray) {
            if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                let mut unique_logins = std::collections::HashSet::new();
                let mut unique_uuids = std::collections::HashSet::new();
                let mut new_clients = Vec::new();
                let mut all_valid = true;

                for user in usuarios.iter().rev() {
                    if user.tipo == "xray" {
                        if let Some(uuid) = &user.uuid {
                            if !uuid.is_empty()
                                && unique_logins.insert(user.login.clone())
                                && unique_uuids.insert(uuid.clone()) {
                                new_clients.push(serde_json::json!({
                                    "email": user.login,
                                    "id": uuid,
                                    "level": 0
                                }));
                            }
                        } else {
                            all_valid = false;
                            break;
                        }
                    }
                }

                if all_valid {
                    let mut updated = false;
                    if let Some(inbounds) = json.get_mut("inbounds") {
                        for inbound in inbounds.as_array_mut().unwrap() {
                            if inbound["protocol"] == "vless" {
                                if let Some(settings) = inbound.get_mut("settings") {
                                    settings["clients"] = serde_json::Value::Array(new_clients.clone());
                                    updated = true;
                                }
                            }
                        }
                    }
                    if updated {
                        let tmp_path = "/usr/local/etc/xray/config.json.tmp";
                        if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                            if fs::write(tmp_path, new_content).is_ok() {
                                let _ = fs::rename(tmp_path, config_path_xray);
                                reiniciar_xray().await;
                            }
                        }
                    }
                }
            }
        }
    }
}

// Função auxiliar para atualizar configurações do V2Ray
async fn atualizar_configs_v2ray(usuarios: &[User]) {
    let config_path_v2ray = "/etc/v2ray/config.json";
    if std::path::Path::new(config_path_v2ray).exists() {
        if let Ok(content) = fs::read_to_string(config_path_v2ray) {
            if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                if let Some(inbounds) = json.get_mut("inbounds") {
                    if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                        if let Some(settings) = first_inbound.get_mut("settings") {
                            if let Some(clients) = settings.get_mut("clients") {
                                if let Some(clients_array) = clients.as_array_mut() {
                                    clients_array.clear();
                                    for user in usuarios {
                                        if user.tipo == "v2ray" {
                                            if let Some(uuid) = &user.uuid {
                                                clients_array.push(serde_json::json!({
                                                    "id": uuid,
                                                    "alterId": 0,
                                                    "email": user.login
                                                }));
                                            }
                                        }
                                    }
                                    let tmp_path = "/etc/v2ray/config.json.tmp";
                                    if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                        if fs::write(tmp_path, new_content).is_ok() {
                                            let _ = fs::rename(tmp_path, config_path_v2ray);
                                            reiniciar_v2ray().await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
