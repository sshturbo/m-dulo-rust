use sqlx::{Pool, Sqlite};
use std::process::Command;
use crate::models::delete_global::ExcluirGlobalRequest;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use thiserror::Error;
use std::fs;
use serde_json::Value;
use crate::utils::backup_utils::backup_database;

#[derive(Error, Debug)]
pub enum ExcluirGlobalError {
    #[error("Erro ao verificar usuário no banco de dados")]
    VerificarUsuario,
    #[error("Falha ao executar comando")]
    FalhaComando,
    #[error("Nenhum usuário encontrado para ser excluído")]
    NenhumUsuarioEncontrado,
    #[error("Falha ao excluir usuário")]
    ExcluirUsuario,
    #[error("Erro ao remover usuário do banco de dados")]
    RemoverUsuarioBanco,
}

pub async fn excluir_global(
    pool: Pool<Sqlite>,
    payload: ExcluirGlobalRequest,
) -> Result<(), ExcluirGlobalError> {
    let mut uuids_to_remove_v2ray = Vec::new();
    let mut uuids_to_remove_xray = Vec::new();
    let mut usuarios_existentes: Vec<String> = Vec::new();
    let mut deve_reiniciar_v2ray = false;

    for usuario in &payload.usuarios {
        let user_exists = sqlx::query("SELECT 1 FROM users WHERE login = ?")
            .bind(&usuario.usuario)
            .fetch_optional(&pool)
            .await
            .map_err(|_| ExcluirGlobalError::VerificarUsuario)?
            .is_some();

        if !user_exists {
            continue;
        }

        let output = Command::new("id")
            .arg(&usuario.usuario)
            .output()
            .map_err(|_| ExcluirGlobalError::FalhaComando)?;

        if output.status.success() {
            usuarios_existentes.push(usuario.usuario.clone());
        }

        let tipo: Option<String> = sqlx::query_scalar("SELECT tipo FROM users WHERE login = ?")
            .bind(&usuario.usuario)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

        if let Some(uuid) = &usuario.uuid {
            match tipo.as_deref() {
                Some("xray") => uuids_to_remove_xray.push(uuid.clone()),
                _ => uuids_to_remove_v2ray.push(uuid.clone()),
            }
        }
    }

    if usuarios_existentes.is_empty() {
        return Err(ExcluirGlobalError::NenhumUsuarioEncontrado);
    }

    for usuario in usuarios_existentes {
        let _ = Command::new("pkill")
            .args(["-u", &usuario])
            .status();

        let _ = Command::new("userdel")
            .arg(&usuario)
            .status()
            .map_err(|_| ExcluirGlobalError::ExcluirUsuario)?;

        let _ = sqlx::query("DELETE FROM users WHERE login = ?")
            .bind(&usuario)
            .execute(&pool)
            .await
            .map_err(|_| ExcluirGlobalError::RemoverUsuarioBanco)?;
    }

    if !uuids_to_remove_v2ray.is_empty() {
        let config_path = "/etc/v2ray/config.json";
        if std::path::Path::new(config_path).exists() {
            if let Ok(content) = fs::read_to_string(config_path) {
                if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                    if let Some(inbounds) = json.get_mut("inbounds") {
                        if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                            if let Some(settings) = first_inbound.get_mut("settings") {
                                if let Some(clients) = settings.get_mut("clients") {
                                    if let Some(clients_array) = clients.as_array_mut() {
                                        clients_array.retain(|client| {
                                            !uuids_to_remove_v2ray.contains(&client["id"].as_str().unwrap_or("").to_string())
                                        });
                                        let tmp_path = "/etc/v2ray/config.json.tmp";
                                        if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                            if fs::write(tmp_path, new_content).is_ok() {
                                                let _ = fs::rename(tmp_path, config_path);
                                                deve_reiniciar_v2ray = true;
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
    if !uuids_to_remove_xray.is_empty() {
        let config_path = "/usr/local/etc/xray/config.json";
        if std::path::Path::new(config_path).exists() {
            if let Ok(content) = fs::read_to_string(config_path) {
                if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                    if let Some(inbounds) = json.get_mut("inbounds") {
                        for inbound in inbounds.as_array_mut().unwrap() {
                            if inbound["protocol"] == "vless" {
                                if let Some(clients) = inbound["settings"]["clients"].as_array_mut() {
                                    clients.retain(|client| {
                                        !uuids_to_remove_xray.contains(&client["id"].as_str().unwrap_or("").to_string())
                                    });
                                }
                            }
                        }
                    }
                    let tmp_path = "/usr/local/etc/xray/config.json.tmp";
                    if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                        if fs::write(tmp_path, new_content).is_ok() {
                            let _ = fs::rename(tmp_path, config_path);
                            let _ = Command::new("systemctl").arg("restart").arg("xray.service").status();
                        }
                    }
                }
            }
        }
    }

    if deve_reiniciar_v2ray {
        reiniciar_v2ray().await;
    }

    // Backup do banco de dados após exclusão global
    if let Err(e) = backup_database("db/database.sqlite", "/opt/backup-mdulo", "database.sqlite") {
        eprintln!("Erro ao fazer backup do banco de dados: {}", e);
    }

    Ok(())
}
