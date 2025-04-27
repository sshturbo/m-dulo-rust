use sqlx::{Pool, Sqlite, SqlitePool};
use crate::config::Config;
use std::fs;
use std::path::Path;
use log::{info, error};

pub async fn initialize_db() -> Result<Pool<Sqlite>, sqlx::Error> {
    // Obter caminho do banco de dados do config.json
    let database_url = &Config::get().database_url;

    // Criar diretório se não existir
    let db_dir = Path::new("db");
    if !db_dir.exists() {
        fs::create_dir_all(db_dir).expect("Falha ao criar diretório para o banco");
    }

    // Criar o arquivo do banco de dados, se não existir
    let db_path = Path::new(database_url);
    if !db_path.exists() {
        fs::File::create(db_path).expect("Falha ao criar arquivo do banco de dados");
    }

    // Conectar ao banco de dados
    let pool = SqlitePool::connect(database_url).await?;

    // Criar tabelas, se não existirem
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            login TEXT NOT NULL UNIQUE,
            senha TEXT NOT NULL,
            dias INTEGER NOT NULL,
            limite INTEGER NOT NULL,
            uuid TEXT,
            tipo TEXT NOT NULL DEFAULT 'v2ray',
            suspenso TEXT DEFAULT 'não',
            dono TEXT DEFAULT 'admin',
            byid INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS online (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            byid INTEGER NOT NULL,
            login TEXT NOT NULL UNIQUE,
            limite INTEGER NOT NULL,
            usuarios_online INTEGER DEFAULT 0,
            inicio_sessao TEXT NOT NULL,
            status TEXT DEFAULT 'On'
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS dominio_cloudflare (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            subdominio TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

// Função para salvar o subdomínio
pub async fn salvar_subdominio(pool: &Pool<Sqlite>, subdominio: &str) -> Result<(), sqlx::Error> {
    info!("Tentando salvar subdomínio: {}", subdominio);

    // Executa dentro de uma transação para garantir consistência
    let mut tx = pool.begin().await?;

    // Deleta registros existentes
    sqlx::query("DELETE FROM dominio_cloudflare")
        .execute(&mut *tx)
        .await?;

    // Insere o novo registro
    let result = sqlx::query(
        "INSERT INTO dominio_cloudflare (subdominio) VALUES (?)"
    )
    .bind(subdominio)
    .execute(&mut *tx)
    .await?;

    // Commit da transação
    tx.commit().await?;

    info!("Subdomínio salvo com sucesso. Rows affected: {}", result.rows_affected());
    Ok(())
}

pub async fn buscar_subdominio(pool: &Pool<Sqlite>) -> Result<Option<String>, sqlx::Error> {
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
