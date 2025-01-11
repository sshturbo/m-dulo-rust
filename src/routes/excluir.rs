use axum::{
    extract::{Path, State},
    response::IntoResponse,
    http::StatusCode,
};
use sqlx::{Pool, Sqlite};
use std::process::Command;
use log::{info, error};
use std::fs;
use serde_json::Value;

pub async fn excluir_usuario(
    Path((usuario, uuid)): Path<(String, Option<String>)>,
    State(pool): State<Pool<Sqlite>>,
) -> impl IntoResponse {
    info!("Tentativa de exclusão do usuário {}", usuario);

    let output = Command::new("id")
        .arg(&usuario)
        .output()
        .expect("Falha ao executar comando");

    if !output.status.success() {
        error!("Usuário {} não encontrado no sistema", usuario);
        return (StatusCode::NOT_FOUND, "Usuário não encontrado no sistema").into_response();
    }

    if let Some(uuid) = uuid {
        remover_uuid_v2ray(&uuid).await;
    }

    let _ = Command::new("pkill")
        .args(["-u", &usuario])
        .status();

    let _ = Command::new("userdel")
        .arg(&usuario)
        .status()
        .expect("Falha ao excluir usuário");

    match sqlx::query("DELETE FROM users WHERE login = ?")
        .bind(&usuario)
        .execute(&pool)
        .await
    {
        Ok(_) => {
            info!("Usuário {} excluído com sucesso", usuario);
            (StatusCode::OK, "Usuário excluído com sucesso").into_response()
        }
        Err(e) => {
            error!("Erro ao excluir usuário {} do banco: {}", usuario, e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao excluir usuário do banco: {}", e)).into_response()
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
                                    if fs::write(config_path, new_content).is_ok() {
                                        let _ = Command::new("systemctl")
                                            .args(["restart", "v2ray"])
                                            .status();
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
