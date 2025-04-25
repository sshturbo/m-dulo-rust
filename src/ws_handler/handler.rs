use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use axum::response::IntoResponse;
use sqlx::{Pool, Sqlite};
use crate::routes::{
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
use crate::config::Config;
use thiserror::Error;
use crate::routes::online_monitor::monitor_users;
use axum::http::StatusCode;
use crate::routes::criar::{Database, CriarError};
use crate::routes::criar::criar_usuario;
use log::{info, error};
use std::time::Duration;
use serde_json::json;

#[derive(Error, Debug)]
pub enum WsHandlerError {
    // #[error("Token não configurado")]
    // TokenNaoConfigurado,
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
    #[error("Status: {0}")]
    Status(StatusCode),
}

impl From<StatusCode> for WsHandlerError {
    fn from(status: StatusCode) -> Self {
        WsHandlerError::Status(status)
    }
}

impl IntoResponse for CriarError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
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
    
    let mut interval = tokio::time::interval(Duration::from_secs(2));  // Atualização a cada 2 segundos
    let mut last_update = String::new();
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match monitor_users(pool.clone()).await {
                    Ok(users) => {
                        let current_update = users.to_string();
                        // Só envia se houver mudança no estado dos usuários online
                        if current_update != last_update {
                            match socket.send(Message::Text(current_update.clone())).await {
                                Ok(_) => {
                                    last_update = current_update;
                                },
                                Err(e) => {
                                    if e.to_string().contains("Broken pipe") || 
                                       e.to_string().contains("Connection reset by peer") {
                                        info!("Cliente desconectado do WebSocket /online");
                                        break;
                                    }
                                    error!("Erro ao enviar mensagem: {}", e);
                                    break;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        error!("Erro ao monitorar usuários: {}", e);
                        if let Err(send_err) = socket
                            .send(Message::Text(json!({
                                "status": "error",
                                "message": "Erro ao monitorar usuários",
                                "details": e.to_string()
                            }).to_string()))
                            .await 
                        {
                            error!("Erro ao enviar mensagem de erro: {}", send_err);
                            break;
                        }
                    }
                }
            }
            
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Close(_)) => {
                        info!("Cliente solicitou fechamento da conexão WebSocket /online");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        if let Err(e) = socket.send(Message::Pong(data)).await {
                            error!("Erro ao responder ping: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Erro ao receber mensagem do cliente: {}", e);
                        break;
                    }
                    _ => {} // Ignora outros tipos de mensagem
                }
            }
        }
    }

    info!("Conexão WebSocket /online encerrada");
}

async fn handle_message(text: &str, db: Database, pool: &Pool<Sqlite>) -> Result<String, WsHandlerError> {
    let parts: Vec<&str> = text.splitn(3, ':').collect();
    if parts.len() != 3 {
        return Err(WsHandlerError::FormatoInvalido);
    }

    let (token, comando, dados) = (parts[0], parts[1], parts[2]);
    
    let expected_token = &Config::get().api_token;
    if token != expected_token {
        return Err(WsHandlerError::TokenInvalido);
    }

    match comando {
        "CRIAR" => {
            let user: User = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosUsuarioInvalidos)?;
            if user.tipo != "v2ray" && user.tipo != "xray" {
                return Err(WsHandlerError::DadosUsuarioInvalidos);
            }
            criar_usuario(db.clone(), &pool, user).await?;
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
            if usuarios.iter().any(|u| u.tipo != "v2ray" && u.tipo != "xray") {
                return Err(WsHandlerError::DadosUsuarioInvalidos);
            }
            sincronizar_usuarios(db, pool, usuarios).await.map_err(WsHandlerError::SincronizarUsuarios)?;
            Ok("Usuários sincronizados com sucesso!".to_string())
        },
        "EDITAR" => {
            let edit_req: EditRequest = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosEdicaoInvalidos)?;
            if edit_req.tipo != "v2ray" && edit_req.tipo != "xray" {
                return Err(WsHandlerError::DadosUsuarioInvalidos);
            }
            editar_usuario(db, pool, edit_req).await.map_err(WsHandlerError::EditarUsuario)?;
            Ok("Usuário editado com sucesso!".to_string())
        },
        _ => Err(WsHandlerError::FormatoInvalido)
    }
}
