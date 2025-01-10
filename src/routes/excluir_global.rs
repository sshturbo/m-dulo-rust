use axum::{
    extract::State,
    response::IntoResponse,
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};
use serde::Deserialize;
use std::process::Command;
use std::fs;
use serde_json::Value;

#[derive(Deserialize)]
pub struct ExcluirGlobalRequest {
    usuarios: Vec<UsuarioUuid>,
}

#[derive(Deserialize)]
pub struct UsuarioUuid {
    usuario: String,
    uuid: Option<String>,
}

pub async fn excluir_global(
    State(pool): State<Pool<Sqlite>>,
    Json(payload): Json<ExcluirGlobalRequest>,
) -> impl IntoResponse {
    let mut uuids_to_remove = Vec::new();

    for item in payload.usuarios {
        let usuario = item.usuario;
        let uuid = item.uuid;

        // Verificar se o usuário existe no sistema
        let output = Command::new("id")
            .arg(&usuario)
            .output()
            .expect("Falha ao executar comando");

        if !output.status.success() {
            continue; 
        }

        // Se UUID foi fornecido, adicionar à lista de remoção
        if let Some(uuid) = uuid {
            uuids_to_remove.push(uuid);
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
        let _ = sqlx::query("DELETE FROM users WHERE login = ?")
            .bind(&usuario)
            .execute(&pool)
            .await;
    }

    // Remover todos os UUIDs do V2Ray e reiniciar o serviço
    if !uuids_to_remove.is_empty() {
        remover_uuids_v2ray(&uuids_to_remove).await;
    }

    (StatusCode::OK, "Usuários excluídos com sucesso").into_response()
}

async fn remover_uuids_v2ray(uuids: &[String]) {
    let config_path = "/etc/v2ray/config.json";
    
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
                                // Remover os clientes com os UUIDs especificados
                                clients_array.retain(|client| {
                                    !uuids.contains(&client["id"].as_str().unwrap_or("").to_string())
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
