use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use axum::response::IntoResponse;
use sqlx::{Pool, Sqlite};
use crate::routes::{
    criar::criar_usuario,
    excluir::excluir_usuario,
    excluir_global::excluir_global,
    sincronizar::sincronizar_usuarios,
    editar::editar_usuario,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use axum::extract::{Path, State};
use crate::models::user::User;
use crate::models::delete::DeleteRequest;
use crate::models::delete_global::ExcluirGlobalRequest;
use crate::models::edit::EditRequest;
use std::env;
use thiserror::Error;
use crate::routes::online::monitor_users;
use tokio::time::Duration;
use log::info; // Adicione esta linha

type Database = Arc<Mutex<HashMap<String, User>>>;

#[derive(Error, Debug)]
pub enum WsHandlerError {
    #[error("Token não configurado")]
    TokenNaoConfigurado,
    #[error("Token inválido")]
    TokenInvalido,
    #[error("Formato inválido. Use TOKEN:COMANDO:DADOS (exemplo: TOKEN:CRIAR:{{...}})")]
    FormatoInvalido,
    #[error("Dados de usuário inválidos")]
    DadosUsuarioInvalidos,
    #[error("Dados de exclusão inválidos")]
    DadosExclusaoInvalidos,
    #[error("Dados de exclusão global inválidos")]
    DadosExclusaoGlobalInvalidos,
    #[error("Dados de edição inválidos")]
    DadosEdicaoInvalidos,
    #[error("Erro ao criar usuário: {0}")]
    CriarUsuario(#[from] crate::routes::criar::CriarError),
    #[error("Erro ao excluir usuário: {0}")]
    ExcluirUsuario(#[from] crate::routes::excluir::ExcluirError),
    #[error("Erro ao excluir usuários globais: {0}")]
    ExcluirGlobal(#[from] crate::routes::excluir_global::ExcluirGlobalError),
    #[error("Erro ao sincronizar usuários: {0}")]
    SincronizarUsuarios(#[from] crate::routes::sincronizar::SyncError),
    #[error("Erro ao editar usuário: {0}")]
    EditarUsuario(#[from] crate::routes::editar::EditarError),
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    pool: axum::extract::State<Pool<Sqlite>>,
) -> impl IntoResponse {
    let db: Database = Arc::new(Mutex::new(HashMap::new()));
    ws.on_upgrade(move |socket| handle_socket(socket, db, pool.0))
}

pub async fn websocket_online_handler(
    ws: WebSocketUpgrade,
    pool: axum::extract::State<Pool<Sqlite>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_online_socket(socket, pool.0))
}

async fn handle_socket(
    mut socket: WebSocket,
    db: Database,
    pool: Pool<Sqlite>,
) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let response = match handle_message(&text, db.clone(), &pool).await {
                Ok(msg) => msg,
                Err(e) => e.to_string(),
            };
            let _ = socket.send(Message::Text(response)).await;
        }
    }
}

async fn handle_online_socket(
    mut socket: WebSocket,
    pool: Pool<Sqlite>,
) {
    info!("Cliente conectado ao WebSocket /online");

    loop {
        info!("Chamando monitor_users");
        let online_users = match monitor_users(pool.clone()).await {
            Ok(users) => {
                if users.is_empty() {
                    serde_json::json!({"message": "Nenhum usuário online no momento."}).to_string()
                } else {
                    serde_json::to_string(&users).unwrap_or_else(|_| "[]".to_string())
                }
            },
            Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
        };
        info!("Verificação de usuários online realizada.");
        info!("Enviando usuários online: {}", online_users);
        if let Err(e) = socket.send(Message::Text(online_users.clone())).await {
            info!("Erro ao enviar mensagem: {}", e);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    info!("Cliente desconectado do WebSocket /online");
}

async fn handle_message(text: &str, db: Database, pool: &Pool<Sqlite>) -> Result<String, WsHandlerError> {
    let parts: Vec<&str> = text.splitn(3, ':').collect();
    if parts.len() != 3 {
        return Err(WsHandlerError::FormatoInvalido);
    }

    let (token, comando, dados) = (parts[0], parts[1], parts[2]);
    
    let expected_token = env::var("API_TOKEN").map_err(|_| WsHandlerError::TokenNaoConfigurado)?;
    if token != expected_token {
        return Err(WsHandlerError::TokenInvalido);
    }

    match comando {
        "CRIAR" => {
            let user: User = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosUsuarioInvalidos)?;
            criar_usuario(db, pool, user).await?;
            Ok("Usuário criado com sucesso!".to_string())
        },
        "EXCLUIR" => {
            let delete_req: DeleteRequest = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosExclusaoInvalidos)?;
            excluir_usuario(Path((delete_req.usuario, delete_req.uuid)), State(pool.clone())).await.map_err(WsHandlerError::ExcluirUsuario)
        },
        "EXCLUIR_GLOBAL" => {
            let excluir_global_req: ExcluirGlobalRequest = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosExclusaoGlobalInvalidos)?;
            excluir_global(pool.clone(), excluir_global_req).await.map_err(WsHandlerError::ExcluirGlobal)?;
            Ok("Usuários excluídos com sucesso!".to_string())
        },
        "SINCRONIZAR" => {
            let usuarios: Vec<User> = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosUsuarioInvalidos)?;
            sincronizar_usuarios(db, pool, usuarios).await?;
            Ok("Usuários sincronizados com sucesso!".to_string())
        },
        "EDITAR" => {
            let edit_req: EditRequest = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosEdicaoInvalidos)?;
            editar_usuario(db, pool, edit_req).await?;
            Ok("Usuário editado com sucesso!".to_string())
        },
        _ => Err(WsHandlerError::FormatoInvalido)
    }
}