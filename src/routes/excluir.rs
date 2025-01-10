use axum::{
    extract::{Path, State},
    response::IntoResponse,
    http::StatusCode,
};
use sqlx::{Pool, Sqlite};
use std::process::Command;
use std::fs;
use serde_json::Value;

pub async fn excluir_usuario(
    Path((usuario, uuid)): Path<(String, Option<String>)>,
    State(pool): State<Pool<Sqlite>>,
) -> impl IntoResponse {
    // Verificar se o usuário existe no sistema
    let output = Command::new("id")
        .arg(&usuario)
        .output()
        .expect("Falha ao executar comando");

    if !output.status.success() {
        return (StatusCode::NOT_FOUND, "Usuário não encontrado no sistema").into_response();
    }

    // Se UUID foi fornecido, tentar remover do V2Ray
    if let Some(uuid) = uuid {
        remover_uuid_v2ray(&uuid).await;
    }

    // Matar todos os processos do usuário
    let _ = Command::new("pkill")
        .args(["-u", &usuario])
        .status();

    // Excluir o usuário do sistema
    let _ = Command::new("userdel")
        .arg(&usuario)
        .status()
        .expect("Falha ao excluir usuário");

    // Remover do banco de dados
    match sqlx::query("DELETE FROM users WHERE login = ?")
        .bind(&usuario)
        .execute(&pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "Usuário excluído com sucesso").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao excluir usuário do banco: {}", e)).into_response()
    }
}

async fn remover_uuid_v2ray(uuid: &str) {
    let config_path = "/etc/v2ray/config.json";
    use log::{info, error};

pub async fn excluir_usuario(
    Path((usuario, uuid)): Path<(String, Option<String>)>,
    State(pool): State<Pool<Sqlite>>,
) -> impl IntoResponse {
    info!("Tentativa de exclusão do usuário {}", usuario);

    // Verificar se o usuário existe no sistema
    let output = Command::new("id")
        .arg(&usuario)
        .output()
        .expect("Falha ao executar comando");

    if !output.status.success() {
        error!("Usuário {} não encontrado no sistema", usuario);
        return (StatusCode::NOT_FOUND, "Usuário não encontrado no sistema").into_response();
    }

    // Se UUID foi fornecido, tentar remover do V2Ray
    if let Some(uuid) = uuid {
        remover_uuid_v2ray(&uuid).await;
    }

    // Matar todos os processos do usuário
    let _ = Command::new("pkill")
        .args(["-u", &usuario])
        .status();

    // Excluir o usuário do sistema
    let _ = Command::new("userdel")
        .arg(&usuario)
        .status()
        .expect("Falha ao excluir usuário");

    // Remover do banco de dados
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
        info!("V2Ray não está instalado");
        return; 
    }

    // Ler e parsear o arquivo de configuração
    if let Ok(content) = fs::read_to_string(config_path) {
        if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
            if let Some(inbounds) = json.get_mut("inbounds") {
                if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                    if let Some(settings) = first_inbound.get_mut("settings") {
                        if let Some(clients) = settings.get_mut("clients") {
                            if let Some(clients_array) = clients.as_array_mut() {
                                // Remover o cliente com o UUID especificado
                                clients_array.retain(|client| {
                                    client["id"].as_str().unwrap_or("") != uuid
                                });

                                // Salvar as alterações
                                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                    if fs::write(config_path, new_content).is_ok() {
                                        info!("Configuração do V2Ray atualizada com sucesso");
                                        // Reiniciar o serviço V2Ray
                                        let _ = Command::new("systemctl")
                                            .args(["restart", "v2ray"])
                                            .status();
                                    } else {
                                        error!("Falha ao salvar alterações na configuração do V2Ray");
                                    }
                                } else {
                                    error!("Falha ao serializar a configuração do V2Ray");
                                }
                            }
                        }
                    }
                }
            }
        } else {
            error!("Falha ao parsear a configuração do V2Ray");
        }
    } else {
        error!("Falha ao ler a configuração do V2Ray");
    }
}
    if !std::path::Path::new(config_path).exists() {
        return; // V2Ray não está instalado
    }

    // Ler e parsear o arquivo de configuração
    if let Ok(content) = fs::read_to_string(config_path) {
        if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
            if let Some(inbounds) = json.get_mut("inbounds") {
                if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                    if let Some(settings) = first_inbound.get_mut("settings") {
                        if let Some(clients) = settings.get_mut("clients") {
                            if let Some(clients_array) = clients.as_array_mut() {
                                // Remover o cliente com o UUID especificado
                                clients_array.retain(|client| {
                                    client["id"].as_str().unwrap_or("") != uuid
                                });

                                // Salvar as alterações
                                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                    if fs::write(config_path, new_content).is_ok() {
                                        // Reiniciar o serviço V2Ray
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
