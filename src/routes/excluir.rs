use axum::extract::{Path, State};
use sqlx::{Pool, Sqlite};
use std::process::Command;
use log::{info, error};
use std::fs;
use serde_json::Value;
use crate::utils::restart_v2ray::reiniciar_v2ray;

pub async fn excluir_usuario(
    Path((usuario, uuid)): Path<(String, Option<String>)>,
    State(pool): State<Pool<Sqlite>>,
) -> Result<String, String> {
    info!("Tentativa de exclusão do usuário {}", usuario);

    let output = Command::new("id")
        .arg(&usuario)
        .output()
        .map_err(|_| "Falha ao executar comando".to_string())?;

    if !output.status.success() {
        error!("Usuário {} não encontrado no sistema", usuario);
        return Err("Usuário não encontrado no sistema".to_string());
    }

    if let Some(uuid) = uuid {
        if std::path::Path::new("/etc/v2ray/config.json").exists() {
            remover_uuid_v2ray(&uuid).await;
            reiniciar_v2ray().await;
        } else {
            info!("Arquivo /etc/v2ray/config.json não encontrado, ignorando remoção de UUID e reinício do V2Ray");
        }
    }

    let _ = Command::new("pkill")
        .args(["-u", &usuario])
        .status();

    let _ = Command::new("userdel")
        .arg(&usuario)
        .status()
        .map_err(|_| "Falha ao excluir usuário".to_string())?;

    match sqlx::query("DELETE FROM users WHERE login = ?")
        .bind(&usuario)
        .execute(&pool)
        .await
    {
        Ok(_) => {
            info!("Usuário {} excluído com sucesso", usuario);
            Ok("Usuário excluído com sucesso".to_string())
        }
        Err(e) => {
            error!("Erro ao excluir usuário {} do banco: {}", usuario, e);
            Err(format!("Erro ao excluir usuário do banco: {}", e))
        }
    }
}

async fn remover_uuid_v2ray(uuid: &str) {
    let config_path = "/etc/v2ray/config.json";

    if !std::path::Path::new(config_path).exists() {
        return;
    }

    if let Ok(content) = fs::read_to_string(config_path) {
        if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
            if let Some(inbounds) = json.get_mut("inbounds") {
                if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                    if let Some(settings) = first_inbound.get_mut("settings") {
                        if let Some(clients) = settings.get_mut("clients") {
                            if let Some(clients_array) = clients.as_array_mut() {
                                clients_array.retain(|client| client["id"].as_str().unwrap_or("") != uuid);

                                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                    let _ = fs::write(config_path, new_content);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

