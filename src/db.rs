use sqlx::{Pool, Sqlite, SqlitePool};
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;

async fn setup_database_dir() -> std::io::Result<()> {
    // Criar diretório db se não existir
    let db_dir = Path::new("db");
    if !db_dir.exists() {
        fs::create_dir(db_dir)?;
        // Definir permissões 755 para o diretório
        fs::set_permissions(db_dir, fs::Permissions::from_mode(0o755))?;
    }

    // Criar arquivo do banco se não existir
    let db_file = db_dir.join("banco.sqlite");
    if !db_file.exists() {
        fs::File::create(&db_file)?;
        // Definir permissões 644 para o arquivo
        fs::set_permissions(&db_file, fs::Permissions::from_mode(0o644))?;
    }

    Ok(())
}

pub async fn initialize_db() -> Result<Pool<Sqlite>, sqlx::Error> {
    // Configurar diretório e arquivo do banco
    setup_database_dir()
        .await
        .expect("Falha ao criar diretório/arquivo do banco de dados");

    // Conectar ao banco usando o caminho correto
    let database_url = "sqlite:db/banco.sqlite";
    let pool = SqlitePool::connect(database_url).await?;
    
    // Criar tabela se não existir
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
