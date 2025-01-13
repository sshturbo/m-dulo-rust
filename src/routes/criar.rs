use crate::models::user::User;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::utils::user_utils::process_user_data;

pub type Database = Arc<Mutex<HashMap<String, User>>>;

pub async fn criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), String> {
    let mut db = db.lock().await;

    let existing_user = sqlx::query_scalar::<_, String>("SELECT login FROM users WHERE login = ?")
        .bind(&user.login)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Erro ao verificar usuário: {}", e))?;

    if existing_user.is_some() {
        return Err("Usuário já existe no banco de dados!".to_string());
    }

    sqlx::query(
        "INSERT INTO users (login, senha, dias, limite, uuid) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user.login)
    .bind(&user.senha)
    .bind(user.dias as i64)
    .bind(user.limite as i64)
    .bind(&user.uuid)
    .execute(pool)
    .await
    .map_err(|e| format!("Erro ao inserir usuário: {}", e))?;

    db.insert(user.login.clone(), user.clone());
    println!("Usuário criado com sucesso!");
    process_user_data(user).await?;
    Ok(())
}