use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use crate::config::Config;
use log::{info, error};

pub async fn initialize_db() -> Result<PgPool, sqlx::Error> {
    // Obter string de conexão do config.json
    let database_url = &Config::get().database_url;

    // Conectar ao banco de dados PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    // Criar tabelas, se não existirem
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            login TEXT NOT NULL UNIQUE,
            senha TEXT NOT NULL,
            dias INTEGER NOT NULL,
            limite INTEGER NOT NULL,
            uuid TEXT,
            tipo TEXT NOT NULL DEFAULT 'v2ray',
            suspenso TEXT DEFAULT 'não',
            dono TEXT DEFAULT 'admin',
            byid INTEGER NOT NULL
        )"
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS online (
            id SERIAL PRIMARY KEY,
            byid INTEGER NOT NULL,
            login TEXT NOT NULL UNIQUE,
            limite INTEGER NOT NULL,
            usuarios_online INTEGER DEFAULT 0,
            inicio_sessao TEXT NOT NULL,
            status TEXT DEFAULT 'On'
        )"
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS dominio_cloudflare (
            id SERIAL PRIMARY KEY,
            subdominio TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

// Função para salvar o subdomínio
pub async fn salvar_subdominio(pool: &PgPool, subdominio: &str) -> Result<(), sqlx::Error> {
    info!("Tentando salvar subdomínio: {}", subdominio);

    // Executa dentro de uma transação para garantir consistência
    let mut tx = pool.begin().await?;

    // Deleta registros existentes
    sqlx::query("DELETE FROM dominio_cloudflare")
        .execute(&mut *tx)
        .await?;

    // Insere o novo registro
    let result = sqlx::query(
        "INSERT INTO dominio_cloudflare (subdominio) VALUES ($1)"
    )
    .bind(subdominio)
    .execute(&mut *tx)
    .await?;

    // Commit da transação
    tx.commit().await?;

    info!("Subdomínio salvo com sucesso. Rows affected: {}", result.rows_affected());
    Ok(())
}

pub async fn buscar_subdominio(pool: &PgPool) -> Result<Option<String>, sqlx::Error> {
    info!("Buscando subdomínio no banco...");
    
    let result = sqlx::query_scalar::<_, String>(
        "SELECT subdominio FROM dominio_cloudflare ORDER BY id DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await;

    match &result {
        Ok(Some(subdominio)) => info!("Subdomínio encontrado: {}", subdominio),
        Ok(None) => info!("Nenhum subdomínio encontrado no banco"),
        Err(e) => error!("Erro ao buscar subdomínio: {}", e),
    }

    result
}
