use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use axum::response::IntoResponse;
use sqlx::PgPool;
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
use tokio::sync::broadcast;
use std::sync::atomic::{AtomicUsize, Ordering};

// Estrutura para controlar o status da sincronização
#[derive(Clone, Debug)]
struct SyncProgress {
    total: usize,
    processed: usize,
    errors: Vec<String>,
    status: String,
}

// Canal global para transmitir atualizações de sincronização
lazy_static::lazy_static! {
    static ref SYNC_CHANNEL: (broadcast::Sender<SyncProgress>, broadcast::Receiver<SyncProgress>) = broadcast::channel(100);
    static ref ACTIVE_SYNCS: AtomicUsize = AtomicUsize::new(0);
}

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
    pool: axum::extract::State<PgPool>,

) -> impl IntoResponse {
    let db: Database = Arc::new(Mutex::new(HashMap::new()));
    ws.on_upgrade(move |socket| handle_socket(socket, db, pool.0))
}

pub async fn websocket_online_handler(
    ws: WebSocketUpgrade,
    pool: axum::extract::State<PgPool>,

) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_online_socket(socket, pool.0))
}

pub async fn websocket_sync_status_handler(
    ws: WebSocketUpgrade,
    pool: axum::extract::State<PgPool>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_sync_status_socket(socket, pool.0))
}

// Handler para enviar o subdomínio Cloudflare via WebSocket
pub async fn websocket_domain_handler(
    ws: WebSocketUpgrade,
    State(pool): State<PgPool>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        // Autenticação por token
        if let Some(Ok(Message::Text(text))) = socket.recv().await {
            let token = text.trim();
            let expected_token = &Config::get().api_token;
            if token != expected_token {
                let _ = socket.send(Message::Text(serde_json::json!({
                    "status": "error",
                    "message": "Token inválido"
                }).to_string())).await;
                info!("Tentativa de conexão com token inválido na rota /domain");
                return;
            }
            info!("Cliente autenticado com sucesso na rota /domain");
        } else {
            let _ = socket.send(Message::Text(serde_json::json!({
                "status": "error",
                "message": "Token não fornecido"
            }).to_string())).await;
            info!("Tentativa de conexão sem token na rota /domain");
            return;
        }

        // Após autenticação, envia o subdomínio em formato JSON
        if let Ok(Some(subdominio)) = crate::db::buscar_subdominio(&pool).await {
            let response = serde_json::json!({
                "status": "success",
                "data": {
                    "subdomain": subdominio
                }
            });
            let _ = socket.send(Message::Text(response.to_string())).await;
        } else {
            let response = serde_json::json!({
                "status": "error",
                "message": "Subdomínio não encontrado"
            });
            let _ = socket.send(Message::Text(response.to_string())).await;
        }

        // Mantém a conexão ativa e processa mensagens
        while let Some(msg) = socket.recv().await {
            match msg {
                Ok(Message::Close(_)) => {
                    info!("Cliente solicitou fechamento da conexão WebSocket /domain");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    if let Err(e) = socket.send(Message::Pong(data)).await {
                        error!("Erro ao responder ping em /domain: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Erro ao receber mensagem do cliente em /domain: {}", e);
                    break;
                }
                _ => {}
            }
        }

        info!("Conexão WebSocket /domain encerrada");
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    db: Database,
    pool: PgPool,
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
    pool: PgPool,
) {
    info!("Cliente conectado ao WebSocket /online");
    
    // Aguarda a mensagem de autenticação
    if let Some(Ok(Message::Text(text))) = socket.recv().await {
        let token = text.trim();
        let expected_token = &Config::get().api_token;
        
        if token != expected_token {
            let _ = socket.send(Message::Text(json!({
                "status": "error",
                "message": "Token inválido"
            }).to_string())).await;
            info!("Tentativa de conexão com token inválido");
            return;
        }
        
        info!("Cliente autenticado com sucesso no WebSocket /online");
    } else {
        let _ = socket.send(Message::Text(json!({
            "status": "error",
            "message": "Token não fornecido"
        }).to_string())).await;
        info!("Tentativa de conexão sem token");
        return;
    }
    
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    let mut last_update = String::new();
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match monitor_users(pool.clone()).await {
                    Ok(users) => {
                        let current_update = users.to_string();
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

async fn handle_sync_status_socket(
    mut socket: WebSocket,
    _pool: PgPool, // Adicionando underscore para indicar que é intencional não usar a variável
) {
    info!("Cliente conectado ao WebSocket /sync-status");
    
    // Autenticação
    if let Some(Ok(Message::Text(text))) = socket.recv().await {
        let token = text.trim();
        let expected_token = &Config::get().api_token;
        
        if token != expected_token {
            let _ = socket.send(Message::Text(json!({
                "status": "error",
                "message": "Token inválido"
            }).to_string())).await;
            info!("Tentativa de conexão com token inválido no /sync-status");
            return;
        }
        
        info!("Cliente autenticado com sucesso no WebSocket /sync-status");
    } else {
        let _ = socket.send(Message::Text(json!({
            "status": "error",
            "message": "Token não fornecido"
        }).to_string())).await;
        info!("Tentativa de conexão sem token no /sync-status");
        return;
    }

    let mut rx = SYNC_CHANNEL.0.subscribe();
    
    // Envia status inicial
    let active_syncs = ACTIVE_SYNCS.load(Ordering::Relaxed);
    let _ = socket.send(Message::Text(json!({
        "status": "connected",
        "active_syncs": active_syncs,
        "message": if active_syncs > 0 { 
            "Sincronização em andamento" 
        } else { 
            "Nenhuma sincronização em andamento" 
        }
    }).to_string())).await;

    loop {
        tokio::select! {
            Ok(progress) = rx.recv() => {
                let status_json = json!({
                    "status": "sync_update",
                    "total": progress.total,
                    "processed": progress.processed,
                    "progress_percentage": if progress.total > 0 {
                        (progress.processed as f64 / progress.total as f64 * 100.0) as u32
                    } else {
                        0
                    },
                    "errors": progress.errors,
                    "state": progress.status
                });

                if let Err(e) = socket.send(Message::Text(status_json.to_string())).await {
                    error!("Erro ao enviar atualização de sincronização: {}", e);
                    break;
                }
            }
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Close(_)) => {
                        info!("Cliente solicitou fechamento da conexão WebSocket /sync-status");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        if let Err(e) = socket.send(Message::Pong(data)).await {
                            error!("Erro ao responder ping em /sync-status: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Erro ao receber mensagem do cliente em /sync-status: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    info!("Conexão WebSocket /sync-status encerrada");
}

async fn handle_message(text: &str, db: Database, pool: &PgPool) -> Result<String, WsHandlerError> {
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
            let db = db.clone();
            let pool = pool.clone();
            tokio::spawn(async move {
                if let Err(e) = criar_usuario(db, &pool, user).await {
                    log::error!("Erro ao criar usuário em background: {:?}", e);
                }
            });
            Ok("Usuário em processamento!".to_string())
        },
        "EXCLUIR" => {
            let delete_req: DeleteRequest = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosExclusaoInvalidos)?;
            let pool = pool.clone();
            tokio::spawn(async move {
                let _ = excluir_usuario(Path((delete_req.usuario, delete_req.uuid)), State(pool)).await;
            });
            Ok("Exclusão de usuário em processamento!".to_string())
        },
        "EXCLUIR_GLOBAL" => {
            let excluir_global_req: ExcluirGlobalRequest = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosExclusaoGlobalInvalidos)?;
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                let _ = excluir_global(pool_clone, excluir_global_req).await;
            });
            Ok("Processo de exclusão global iniciado em segundo plano!".to_string())
        },
        "SINCRONIZAR" => {
            let usuarios: Vec<User> = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosUsuarioInvalidos)?;
            if usuarios.iter().any(|u| u.tipo != "v2ray" && u.tipo != "xray") {
                return Err(WsHandlerError::DadosUsuarioInvalidos);
            }
            let db = db.clone();
            let pool = pool.clone();
            let usuarios_clone = usuarios.clone();
            tokio::spawn(async move {
                ACTIVE_SYNCS.fetch_add(1, Ordering::SeqCst);
                let _ = SYNC_CHANNEL.0.send(SyncProgress {
                    total: usuarios_clone.len(),
                    processed: 0,
                    errors: Vec::new(),
                    status: "Iniciando sincronização".to_string(),
                });
                let result = sincronizar_usuarios(db, &pool, usuarios_clone).await;
                ACTIVE_SYNCS.fetch_sub(1, Ordering::SeqCst);
                let _ = SYNC_CHANNEL.0.send(SyncProgress {
                    total: usuarios.len(),
                    processed: usuarios.len(),
                    errors: if let Err(ref e) = result {
                        vec![e.to_string()]
                    } else {
                        Vec::new()
                    },
                    status: if result.is_ok() {
                        "Sincronização concluída".to_string()
                    } else {
                        "Sincronização falhou".to_string()
                    },
                });
            });
            Ok("Sincronização iniciada em segundo plano!".to_string())
        },
        "EDITAR" => {
            let edit_req: EditRequest = serde_json::from_str(dados)
                .map_err(|_| WsHandlerError::DadosEdicaoInvalidos)?;
            if edit_req.tipo != "v2ray" && edit_req.tipo != "xray" {
                return Err(WsHandlerError::DadosUsuarioInvalidos);
            }
            let db = db.clone();
            let pool = pool.clone();
            tokio::spawn(async move {
                let _ = editar_usuario(db, &pool, edit_req).await;
            });
            Ok("Edição de usuário em processamento!".to_string())
        },
        _ => Err(WsHandlerError::FormatoInvalido)
    }
}
