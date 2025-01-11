use axum::{
    extract::{Path, State},
    response::IntoResponse,
    http::StatusCode,
};
use sqlx::{Pool, Sqlite};
use std::process::Command;
use log::{info, error}; // Adicionar importações

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

async fn remover_uuid_v2ray(_uuid: &str) { // Adicionar função
    // Implementação da função
}
