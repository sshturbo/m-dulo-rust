use dashmap::DashMap;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::oneshot;
use sqlx::PgPool;
use redis::aio::Connection;
use crate::utils::logging;

pub type ConexoesAtivas = Arc<DashMap<Uuid, oneshot::Sender<()>>>;

/// Adiciona uma conexão ativa e registra o usuário no Redis
pub async fn adicionar_conexao(
    ativas: &ConexoesAtivas,
    uuid: Uuid,
    tx: oneshot::Sender<()>,
    redis_conn: &mut Connection,
) -> redis::RedisResult<()> {
    ativas.insert(uuid, tx);
    let _: () = redis::cmd("SADD")
        .arg("usuarios_online")
        .arg(uuid.to_string())
        .query_async(redis_conn)
        .await?;
    Ok(())
}

/// Remove uma conexão ativa e remove o usuário do Redis
pub async fn remover_conexao(
    ativas: &ConexoesAtivas,
    uuid: &Uuid,
    redis_conn: &mut Connection,
) -> redis::RedisResult<()> {
    ativas.remove(uuid);
    let _: () = redis::cmd("SREM")
        .arg("usuarios_online")
        .arg(uuid.to_string())
        .query_async(redis_conn)
        .await?;
    Ok(())
}

/// Lista todos os usuários online (UUIDs) a partir do Redis
#[allow(dead_code)]
pub async fn listar_usuarios_online(redis_conn: &mut Connection) -> redis::RedisResult<Vec<String>> {
    redis::cmd("SMEMBERS")
        .arg("usuarios_online")
        .query_async(redis_conn)
        .await
}

/// Derruba a conexão de um usuário específico
#[allow(dead_code)]
pub fn derrubar_conexao(ativas: &ConexoesAtivas, uuid: &Uuid) {
    if let Some((_, tx)) = ativas.remove(uuid) {
        let _ = tx.send(());
    }
}

/// Valida se o UUID existe no banco de dados PostgreSQL
pub async fn validar_uuid(pool: &PgPool, uuid: &Uuid) -> Result<bool, sqlx::Error> {
    let uuid_str = uuid.to_string();
    logging::log_proxy_tipo_conexao(&format!("Consultando UUID no banco: {}", uuid_str));
    
    // Consulta modificada para comparar strings e usar i32 (INT4)
    let row: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM users WHERE uuid::text = $1::text LIMIT 1")
        .bind(&uuid_str)
        .fetch_optional(pool)
        .await?;
        
    if row.is_some() {
        logging::log_proxy_uuid_valido(uuid, "banco");
    } else {
        logging::log_proxy_uuid_invalido(uuid, "banco");
    }
    Ok(row.is_some())
} 