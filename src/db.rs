use sqlx::{Pool, Postgres, PgPool};
use dotenv::dotenv;
use std::env;

pub async fn initialize_db() -> Result<Pool<Postgres>, sqlx::Error> {
    dotenv().ok(); 
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL não encontrada no .env");

    let pool = PgPool::connect(&database_url).await?;
    
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            login TEXT NOT NULL UNIQUE,
            senha TEXT NOT NULL,
            dias INTEGER NOT NULL,
            limite INTEGER NOT NULL,
            uuid TEXT,
            suspenso TEXT DEFAULT 'não'
        )"
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS online (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id),
            login VARCHAR(255) NOT NULL UNIQUE,
            limite INTEGER NOT NULL,
            online_inicio TIMESTAMP,
            status VARCHAR(255) DEFAULT 'On'
        );"
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
