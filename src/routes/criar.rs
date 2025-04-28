use crate::models::user::User;
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::utils::user_utils::process_user_data;
use thiserror::Error;

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

pub async fn criar_usuario(db: Database, pool: &PgPool, user: User) -> Result<(), CriarError> {
    let mut db = db.lock().await;

    let existing_user = sqlx::query_scalar::<_, String>("SELECT login FROM users WHERE login = $1")
        .bind(&user.login)
        .fetch_optional(pool)
        .await
        .map_err(|e| CriarError::VerificarUsuario(e.to_string()))?;

    if existing_user.is_some() {
        return Err(CriarError::UsuarioJaExiste);
    }

    sqlx::query(
        "INSERT INTO users (login, senha, dias, limite, uuid, tipo, dono, byid) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(&user.login)
    .bind(&user.senha)
    .bind(user.dias as i32)
    .bind(user.limite as i32)
    .bind(&user.uuid)
    .bind(&user.tipo)
    .bind(&user.dono)
    .bind(user.byid as i32)
    .execute(pool)
    .await
    .map_err(|e| CriarError::InserirUsuario(e.to_string()))?;

    db.insert(user.login.clone(), user.clone());
    println!("Usuário criado com sucesso!");
    process_user_data(user).await.map_err(|_| CriarError::ProcessarDadosUsuario)?;

    
    Ok(())
}
