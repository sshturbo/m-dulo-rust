use sqlx::{Pool, Sqlite};
use std::process::Command;
use std::fs;
use serde_json::Value;
use crate::models::delete_global::ExcluirGlobalRequest;
use crate::utils::restart_v2ray::reiniciar_v2ray;

pub async fn excluir_global(
    pool: Pool<Sqlite>,
    payload: ExcluirGlobalRequest,
) -> Result<(), String> {
    let mut uuids_to_remove = Vec::new();
    let mut usuarios_existentes: Vec<String> = Vec::new();

    for usuario in &payload.usuarios {
        let user_exists = sqlx::query("SELECT 1 FROM users WHERE login = ?")
            .bind(&usuario.usuario)
            .fetch_optional(&pool)
            .await
            .map_err(|_| "Erro ao verificar usuário no banco de dados".to_string())?
            .is_some();

        if !user_exists {
            continue;
        }

        let output = Command::new("id")
            .arg(&usuario.usuario)
            .output()
            .map_err(|_| "Falha ao executar comando".to_string())?;

        if output.status.success() {
            usuarios_existentes.push(usuario.usuario.clone());
        }

        if let Some(uuid) = &usuario.uuid {
            uuids_to_remove.push(uuid.clone());
        }
    }

    if usuarios_existentes.is_empty() {
        return Err("Nenhum usuário encontrado para ser excluído".to_string());
    }

    for usuario in usuarios_existentes {
        let _ = Command::new("pkill")
            .args(["-u", &usuario])
            .status();

        let _ = Command::new("userdel")
            .arg(&usuario)
            .status()
            .map_err(|_| "Falha ao excluir usuário".to_string())?;

        let _ = sqlx::query("DELETE FROM users WHERE login = ?")
            .bind(&usuario)
            .execute(&pool)
            .await
            .map_err(|_| "Erro ao remover usuário do banco de dados".to_string())?;
    }

    if !uuids_to_remove.is_empty() {
        remover_uuids_v2ray(&uuids_to_remove).await;
        reiniciar_v2ray().await;
    }

    Ok(())
}

async fn remover_uuids_v2ray(uuids: &[String]) {
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
                                clients_array.retain(|client| {
                                    !uuids.contains(&client["id"].as_str().unwrap_or("").to_string())
                                });

                                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                    if fs::write(config_path, new_content).is_ok() {
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
