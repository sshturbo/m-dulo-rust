use sqlx::{Pool, Sqlite};
use std::process::Command;
use crate::models::user::User;
use crate::models::edit::EditRequest;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::utils::user_utils::{process_user_data, remover_uuid_v2ray, remover_uuids_xray, atualizar_email_xray, atualizar_email_v2ray};
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
    pool: &Pool<Sqlite>,
    edit_req: EditRequest
) -> Result<(), EditarError> {
    let mut db = db.lock().await;

    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE login = ?")
        .bind(&edit_req.login_antigo)
        .fetch_optional(pool)
        .await
        .map_err(|e| EditarError::VerificarUsuario(e.to_string()))?;

    if existing_user.is_none() {
        return Err(EditarError::UsuarioNaoEncontrado);
    }

    // Verificação e remoção do uuid antigo se necessário
    if let Some(old_user) = &existing_user {
        let tipo_antigo = old_user.tipo.as_str();
        let uuid_antigo = old_user.uuid.as_ref();
        let tipo_novo = edit_req.tipo.as_str();
        let uuid_novo = edit_req.uuid.as_ref();
        // Se uuid mudou ou tipo mudou, remover do serviço antigo
        if uuid_antigo != uuid_novo || tipo_antigo != tipo_novo {
            if let Some(uuid) = uuid_antigo {
                match tipo_antigo {
                    "xray" => {
                        remover_uuids_xray(&vec![uuid.clone()]).await;
                    },
                    _ => {
                        remover_uuid_v2ray(uuid).await;
                    }
                }
            }
        } else if edit_req.login_antigo != edit_req.login_novo {
            // Se login mudou, mas tipo e uuid são os mesmos, atualizar o email no JSON
            if let (Some(uuid), "xray") = (uuid_antigo, tipo_antigo) {
                let _ = atualizar_email_xray(uuid, &edit_req.login_novo);
            } else if let (Some(uuid), "v2ray") = (uuid_antigo, tipo_antigo) {
                let _ = atualizar_email_v2ray(uuid, &edit_req.login_novo);
            }
        }
    }

    let new_user_check = sqlx::query_as::<_, User>("SELECT * FROM users WHERE login = ?")
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
        tipo: edit_req.tipo.clone(),
    };

    let result = sqlx::query(
        "UPDATE users SET login = ?, senha = ?, dias = ?, limite = ?, uuid = ?, tipo = ? WHERE login = ?"
    )
    .bind(&new_user.login)
    .bind(&new_user.senha)
    .bind(new_user.dias as i64)
    .bind(new_user.limite as i64)
    .bind(&new_user.uuid)
    .bind(&new_user.tipo)
    .bind(&edit_req.login_antigo)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            db.insert(new_user.login.clone(), new_user.clone());
            println!("Usuário editado com sucesso!");
            // Só chama process_user_data se tipo ou uuid mudaram (ou seja, houve remoção e precisa adicionar de novo)
            let tipo_antigo = existing_user.as_ref().unwrap().tipo.as_str();
            let uuid_antigo = existing_user.as_ref().unwrap().uuid.as_ref();
            let tipo_novo = edit_req.tipo.as_str();
            let uuid_novo = edit_req.uuid.as_ref();
            if uuid_antigo != uuid_novo || tipo_antigo != tipo_novo {
                process_user_data(new_user).await.map_err(|_| EditarError::ProcessarDadosUsuario)?;
            }
            Ok(())
        }
        Err(e) => Err(EditarError::AtualizarUsuarioBanco(e.to_string()))
    }
}
