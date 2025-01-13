use sqlx::{Pool, Sqlite};
use std::process::Command;
use crate::models::user::User;
use crate::models::edit::EditRequest;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::utils::user_utils::process_user_data;

pub type Database = Arc<Mutex<HashMap<String, User>>>;

pub async fn editar_usuario(
    db: Database,
    pool: &Pool<Sqlite>,
    edit_req: EditRequest
) -> Result<(), String> {
    let mut db = db.lock().await;

    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE login = ?")
        .bind(&edit_req.login_antigo)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Erro ao verificar usuário: {}", e))?;

    if existing_user.is_none() {
        return Err("Usuário antigo não encontrado no banco de dados!".to_string());
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
        dias: edit_req.dias,
        limite: edit_req.limite,
        uuid: edit_req.uuid.clone(),
    };

    let result = sqlx::query(
        "UPDATE users SET login = ?, senha = ?, dias = ?, limite = ?, uuid = ? WHERE login = ?"
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
            process_user_data(new_user).await?;
            Ok(())
        }
        Err(e) => Err(format!("Erro ao atualizar usuário no banco de dados: {}", e))
    }
}
