use crate::models::user::User;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::process::Command;
use thiserror::Error;
use crate::utils::user_utils::{remover_uuids_xray, remover_uuid_v2ray};
use std::fs;
use serde_json::Value;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use crate::utils::restart_xray::reiniciar_xray;

pub type Database = Arc<Mutex<HashMap<String, User>>>;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Erro ao verificar usuário: {0}")]
    VerificarUsuario(String),
    #[error("Erro ao inserir usuário no banco de dados: {0}")]
    InserirUsuarioBanco(String)
}

pub async fn sincronizar_usuarios(db: Database, pool: &Pool<Sqlite>, usuarios: Vec<User>) -> Result<(), SyncError> {
    let mut db = db.lock().await;

    // Buscar todos os usuários atuais do banco
    let usuarios_atuais: Vec<User> = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool)
        .await
        .map_err(|e| SyncError::VerificarUsuario(e.to_string()))?;

    // Descobrir quais usuários devem ser removidos (não estão na lista nova)
    for user_atual in &usuarios_atuais {
        if !usuarios.iter().any(|u| u.login == user_atual.login) {
            // Remover do sistema e do serviço correto
            let uuid = user_atual.uuid.clone();
            match user_atual.tipo.as_str() {
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
            let _ = Command::new("pkill").args(["-u", &user_atual.login]).status();
            let _ = Command::new("userdel").arg(&user_atual.login).status();
            sqlx::query("DELETE FROM users WHERE login = ?")
                .bind(&user_atual.login)
                .execute(pool)
                .await
                .ok();
        }
    }

    // Adicionar ou atualizar todos os usuários recebidos
    for user in &usuarios {
        sqlx::query(
            "INSERT OR REPLACE INTO users (login, senha, dias, limite, uuid, tipo) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&user.login)
        .bind(&user.senha)
        .bind(user.dias as i64)
        .bind(user.limite as i64)
        .bind(&user.uuid)
        .bind(&user.tipo)
        .execute(pool)
        .await
        .map_err(|e| SyncError::InserirUsuarioBanco(e.to_string()))?;
        db.insert(user.login.clone(), user.clone());
    }

    // Atualizar config.json do Xray em lote
    let config_path_xray = "/usr/local/etc/xray/config.json";
    if std::path::Path::new(config_path_xray).exists() {
        if let Ok(content) = fs::read_to_string(config_path_xray) {
            if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                // Deduplicar usuários por login e uuid
                let mut unique_logins = std::collections::HashSet::new();
                let mut unique_uuids = std::collections::HashSet::new();
                let mut new_clients = Vec::new();
                let mut all_valid = true;
                for user in usuarios.iter().rev() { // .rev() para manter o último caso haja duplicidade
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
                new_clients.reverse();
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
                } else {
                    eprintln!("Erro: Usuário xray sem uuid válido. Configuração do Xray não foi atualizada.");
                }
            }
        }
    }

    // Atualizar config.json do V2Ray em lote
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
                                    for user in &usuarios {
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
    Ok(())
}
