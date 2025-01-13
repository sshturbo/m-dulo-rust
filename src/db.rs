use sqlx::{Pool, Sqlite, SqlitePool};
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use dotenv::dotenv;
use std::env;

async fn setup_database_dir() -> std::io::Result<()> {
    dotenv().ok(); 
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL não encontrada no .env");
    
    let db_path_str = database_url.trim_start_matches("sqlite:");
    let db_path = Path::new(db_path_str);
    let db_dir = db_path.parent().unwrap();

    if !db_dir.exists() {
        fs::create_dir(db_dir)?;
        fs::set_permissions(db_dir, fs::Permissions::from_mode(0o755))?;
    }

    if !db_path.exists() {
        fs::File::create(&db_path)?;
        fs::set_permissions(&db_path, fs::Permissions::from_mode(0o644))?;
    }

    Ok(())
}

pub async fn initialize_db() -> Result<Pool<Sqlite>, sqlx::Error> {
    dotenv().ok(); 
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL não encontrada no .env");

    setup_database_dir()
        .await
        .expect("Falha ao criar diretório/arquivo do banco de dados");

    let pool = SqlitePool::connect(&database_url).await?;
    
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            login TEXT NOT NULL UNIQUE,
            senha TEXT NOT NULL,
            dias INTEGER NOT NULL,
            limite INTEGER NOT NULL,
            uuid TEXT
        )"
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
