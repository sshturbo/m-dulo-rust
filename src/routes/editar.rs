use sqlx::{Pool, Postgres};
use std::process::Command;
use crate::models::user::User;
use crate::models::edit::EditRequest;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::utils::user_utils::process_user_data;
use thiserror::Error;

pub type Database = Arc<Mutex<HashMap<String, User>>>;

#[derive(Error, Debug)]
pub enum EditarError {
    #[error("Erro ao verificar usuário: {0}")]
    VerificarUsuario(String),
    #[error("Usuário antigo não encontrado no banco de dados!")]
    UsuarioNaoEncontrado,
    #[error("Erro ao atualizar usuário no banco de dados: {0}")]
    AtualizarUsuarioBanco(String),
    #[error("Erro ao processar dados do usuário")]
    ProcessarDadosUsuario,
    #[error("Novo login já existe no banco de dados!")]
    NovoLoginExiste,
}

pub async fn editar_usuario(
    db: Database,
    pool: &Pool<Postgres>,
    edit_req: EditRequest
) -> Result<(), EditarError> {
    let mut db = db.lock().await;

    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE login = $1")
        .bind(&edit_req.login_antigo)
        .fetch_optional(pool)
        .await
        .map_err(|e| EditarError::VerificarUsuario(e.to_string()))?;

    if existing_user.is_none() {
        return Err(EditarError::UsuarioNaoEncontrado);
    }

    let new_user_check = sqlx::query_as::<_, User>("SELECT * FROM users WHERE login = $1")
        .bind(&edit_req.login_novo)
        .fetch_optional(pool)
        .await
        .map_err(|e| EditarError::VerificarUsuario(e.to_string()))?;

    if new_user_check.is_some() {
        return Err(EditarError::NovoLoginExiste);
    }

    let _ = Command::new("pkill")
        .args(["-u", &edit_req.login_antigo])
        .status();

    let _ = Command::new("userdel")
        .arg(&edit_req.login_antigo)
        .status()
        .expect("Falha ao excluir usuário");

    let new_user = User {
        login: edit_req.login_novo.clone(),
        senha: edit_req.senha.clone(),
        dias: edit_req.dias as i32,
        limite: edit_req.limite as i32,
        uuid: edit_req.uuid.clone(),
    };

    let result = sqlx::query(
        "UPDATE users SET login = $1, senha = $2, dias = $3, limite = $4, uuid = $5 WHERE login = $6"
    )
    .bind(&new_user.login)
    .bind(&new_user.senha)
    .bind(new_user.dias as i64)
    .bind(new_user.limite as i64)
    .bind(&new_user.uuid)
    .bind(&edit_req.login_antigo)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            db.insert(new_user.login.clone(), new_user.clone());
            println!("Usuário editado com sucesso!");
            process_user_data(new_user).await.map_err(|_| EditarError::ProcessarDadosUsuario)?;
            Ok(())
        }
        Err(e) => Err(EditarError::AtualizarUsuarioBanco(e.to_string()))
    }
}
