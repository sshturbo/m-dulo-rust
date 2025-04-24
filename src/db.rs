use sqlx::{Pool, Sqlite, SqlitePool};
use dotenv::dotenv;
use std::env;
use std::fs;
use std::path::Path;

pub async fn initialize_db() -> Result<Pool<Sqlite>, sqlx::Error> {
    // Carregar variáveis de ambiente
    dotenv().ok();

    // Obter caminho do banco de dados
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "db/database.sqlite".to_string());

    // Criar diretório se não existir
    let db_dir = Path::new("db");
    if !db_dir.exists() {
        fs::create_dir_all(db_dir).expect("Falha ao criar diretório para o banco");
    }

    // Criar o arquivo do banco de dados, se não existir
    let db_path = Path::new(&database_url);
    if !db_path.exists() {
        fs::File::create(db_path).expect("Falha ao criar arquivo do banco de dados");
    }

    // Conectar ao banco de dados
    let pool = SqlitePool::connect(&database_url).await?;

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
            suspenso TEXT DEFAULT 'não'
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
            status TEXT DEFAULT 'On',
            FOREIGN KEY(byid) REFERENCES users(id)
        )",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
