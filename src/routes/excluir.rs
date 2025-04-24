use axum::extract::{Path, State};
use sqlx::{Pool, Sqlite};
use std::process::Command;
use log::{info, error};
use crate::utils::restart_v2ray::reiniciar_v2ray;
use thiserror::Error;
use crate::utils::user_utils::{remover_uuids_xray, remover_uuid_v2ray};

#[derive(Error, Debug)]
pub enum ExcluirError {
    #[error("Falha ao executar comando")]
    FalhaComando,
    #[error("Erro ao excluir usuário do banco: {0}")]
    ExcluirUsuarioBanco(String),
}

pub async fn excluir_usuario(
    Path((usuario, uuid)): Path<(String, Option<String>)>,
    State(pool): State<Pool<Sqlite>>,
) -> Result<String, ExcluirError> {
    info!("Tentativa de exclusão do usuário {}", usuario);

    let output = Command::new("id")
        .arg(&usuario)
        .output()
        .map_err(|_| ExcluirError::FalhaComando)?;

    if !output.status.success() {
        error!("Usuário {} não encontrado no sistema, excluindo do banco de dados", usuario);

        sqlx::query("DELETE FROM users WHERE login = ?")
            .bind(&usuario)
            .execute(&pool)
            .await
            .map_err(|e| ExcluirError::ExcluirUsuarioBanco(e.to_string()))?;

        info!("Usuário {} excluído com sucesso", usuario);
        return Ok("Usuário excluído com sucesso".to_string());
    }

    // Buscar o tipo do usuário no banco
    let tipo: Option<String> = sqlx::query_scalar("SELECT tipo FROM users WHERE login = ?")
        .bind(&usuario)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    if let Some(uuid) = uuid {
        match tipo.as_deref() {
            Some("xray") => {
                remover_uuids_xray(&vec![uuid.clone()]).await;
                // Reiniciar xray
                let _ = Command::new("systemctl").arg("restart").arg("xray.service").status();
            },
            Some("v2ray") | _ => {
                if std::path::Path::new("/etc/v2ray/config.json").exists() {
                    remover_uuid_v2ray(&uuid).await;
                    reiniciar_v2ray().await;
                } else {
                    info!("Arquivo /etc/v2ray/config.json não encontrado, ignorando remoção de UUID e reinício do V2Ray");
                }
            }
        }
    }

    let _ = Command::new("pkill")
        .args(["-u", &usuario])
        .status();

    let _ = Command::new("userdel")
        .arg(&usuario)
        .status()
        .map_err(|_| ExcluirError::FalhaComando)?;

    sqlx::query("DELETE FROM users WHERE login = ?")
        .bind(&usuario)
        .execute(&pool)
        .await
        .map_err(|e| ExcluirError::ExcluirUsuarioBanco(e.to_string()))?;

    info!("Usuário {} excluído com sucesso", usuario);
    Ok("Usuário excluído com sucesso".to_string())
}

