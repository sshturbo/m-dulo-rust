use crate::models::user::User;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::utils::user_utils::process_user_data;
use thiserror::Error;
use crate::utils::backup_utils::backup_database;

pub type Database = Arc<Mutex<HashMap<String, User>>>;

#[derive(Error, Debug)]
pub enum CriarError {
    #[error("Erro ao verificar usuário: {0}")]
    VerificarUsuario(String),
    #[error("Usuário já existe no banco de dados!")]
    UsuarioJaExiste,
    #[error("Erro ao inserir usuário: {0}")]
    InserirUsuario(String),
    #[error("Erro ao processar dados do usuário")]
    ProcessarDadosUsuario,
}

pub async fn criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), CriarError> { 
    let mut db = db.lock().await;

    let existing_user = sqlx::query_scalar::<_, String>("SELECT login FROM users WHERE login = ?")
        .bind(&user.login)
        .fetch_optional(pool)
        .await
        .map_err(|e| CriarError::VerificarUsuario(e.to_string()))?;

    if existing_user.is_some() {
        return Err(CriarError::UsuarioJaExiste);
    }

    sqlx::query(
        "INSERT INTO users (login, senha, dias, limite, uuid, tipo, dono, byid) VALUES (?, ?, ?, ?, ?, ?, ?, ?)" 
    )
    .bind(&user.login)
    .bind(&user.senha)
    .bind(user.dias as i64)
    .bind(user.limite as i64)
    .bind(&user.uuid)
    .bind(&user.tipo)
    .bind(&user.dono)
    .bind(user.byid as i64)
    .execute(pool)
    .await
    .map_err(|e| CriarError::InserirUsuario(e.to_string()))?;

    db.insert(user.login.clone(), user.clone());
    println!("Usuário criado com sucesso!");
    process_user_data(user).await.map_err(|_| CriarError::ProcessarDadosUsuario)?;

    // Backup do banco de dados
    if let Err(e) = backup_database("db/database.sqlite", "/opt/backup-mdulo", "database.sqlite") {
        eprintln!("Erro ao fazer backup do banco de dados: {}", e);
    }
    
    Ok(())
}
