use sqlx::PgPool;
use std::process::Command;
use crate::models::delete_global::ExcluirGlobalRequest;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use thiserror::Error;
use std::fs;
use serde_json::Value;
use crate::utils::restart_xray::reiniciar_xray;

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
    pool: PgPool,
    payload: ExcluirGlobalRequest,
) -> Result<(), ExcluirGlobalError> {
    let mut usuarios_existentes: Vec<String> = Vec::new();

    // Primeiro passo: Excluir usuários do sistema e banco de dados
    for usuario in &payload.usuarios {
        let user_exists = sqlx::query("SELECT 1 FROM users WHERE login = $1")
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
    }

    if usuarios_existentes.is_empty() {
        return Err(ExcluirGlobalError::NenhumUsuarioEncontrado);
    }

    // Excluir usuários do sistema e banco de dados
    for usuario in usuarios_existentes {
        let _ = Command::new("pkill")
            .args(["-u", &usuario])
            .status();

        let _ = Command::new("userdel")
            .arg(&usuario)
            .status()
            .map_err(|_| ExcluirGlobalError::ExcluirUsuario)?;

        let _ = sqlx::query("DELETE FROM users WHERE login = $1")
            .bind(&usuario)
            .execute(&pool)
            .await
            .map_err(|_| ExcluirGlobalError::RemoverUsuarioBanco)?;
    }

    // Segundo passo: Reescrever configurações V2Ray e XRay com usuários restantes
    // Atualizar V2Ray
    let config_path_v2ray = "/etc/v2ray/config.json";
    if std::path::Path::new(config_path_v2ray).exists() {
        if let Ok(content) = fs::read_to_string(config_path_v2ray) {
            if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                let remaining_users: Vec<(String, String)> = sqlx::query_as("SELECT login, uuid FROM users WHERE tipo = 'v2ray'")
                    .fetch_all(&pool)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|(login, uuid): (String, String)| {
                        if !uuid.is_empty() {
                            Some((login, uuid))
                        } else {
                            None
                        }
                    })
                    .collect();

                let new_clients: Vec<Value> = remaining_users
                    .iter()
                    .map(|(login, uuid)| {
                        serde_json::json!({
                            "id": uuid,
                            "alterId": 0,
                            "email": login
                        })
                    })
                    .collect();

                if let Some(inbounds) = json.get_mut("inbounds") {
                    if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                        if let Some(settings) = first_inbound.get_mut("settings") {
                            settings["clients"] = serde_json::Value::Array(new_clients);
                        }
                    }
                }

                let tmp_path = "/etc/v2ray/config.json.tmp";
                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                    if fs::write(tmp_path, &new_content).is_ok() {
                        let _ = fs::rename(tmp_path, config_path_v2ray);
                        reiniciar_v2ray().await;
                    }
                }
            }
        }
    }

    // Atualizar XRay
    let config_path_xray = "/usr/local/etc/xray/config.json";
    if std::path::Path::new(config_path_xray).exists() {
        if let Ok(content) = fs::read_to_string(config_path_xray) {
            if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                let remaining_users: Vec<(String, String)> = sqlx::query_as("SELECT login, uuid FROM users WHERE tipo = 'xray'")
                    .fetch_all(&pool)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|(login, uuid): (String, String)| {
                        if !uuid.is_empty() {
                            Some((login, uuid))
                        } else {
                            None
                        }
                    })
                    .collect();

                let new_clients: Vec<Value> = remaining_users
                    .iter()
                    .map(|(login, uuid)| {
                        serde_json::json!({
                            "email": login,
                            "id": uuid,
                            "level": 0
                        })
                    })
                    .collect();

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

                let tmp_path = "/usr/local/etc/xray/config.json.tmp";
                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                    if fs::write(tmp_path, &new_content).is_ok() {
                        let _ = fs::rename(tmp_path, config_path_xray);
                        reiniciar_xray().await;
                    }
                }
            }
        }
    }


    Ok(())
}
