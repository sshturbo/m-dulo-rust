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

type Database = Arc<Mutex<HashMap<String, User>>>;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    pool: axum::extract::State<Pool<Sqlite>>,
) -> impl IntoResponse {
    let db: Database = Arc::new(Mutex::new(HashMap::new()));
    ws.on_upgrade(move |socket| handle_socket(socket, db, pool.0))
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
                Err(e) => e,
            };
            let _ = socket.send(Message::Text(response)).await;
        }
    }
}

async fn handle_message(text: &str, db: Database, pool: &Pool<Sqlite>) -> Result<String, String> {
    let parts: Vec<&str> = text.splitn(3, ':').collect();
    if parts.len() != 3 {
        return Err("Formato inválido. Use TOKEN:COMANDO:DADOS (exemplo: TOKEN:CRIAR:{...})".to_string());
    }

    let (token, comando, dados) = (parts[0], parts[1], parts[2]);
    
    let expected_token = env::var("API_TOKEN").map_err(|_| "Token não configurado".to_string())?;
    if token != expected_token {
        return Err("Token inválido".to_string());
    }

    match comando {
        "CRIAR" => {
            let user: User = serde_json::from_str(dados)
                .map_err(|_| "Dados de usuário inválidos".to_string())?;
            criar_usuario(db, pool, user).await.map_err(|e| e.to_string())?;
            Ok("Usuário criado com sucesso!".to_string())
        },
        "EXCLUIR" => {
            let delete_req: DeleteRequest = serde_json::from_str(dados)
                .map_err(|_| "Dados de exclusão inválidos".to_string())?;
            excluir_usuario(Path((delete_req.usuario, delete_req.uuid)), State(pool.clone())).await // Simplificado
        },
        "EXCLUIR_GLOBAL" => {
            let excluir_global_req: ExcluirGlobalRequest = serde_json::from_str(dados)
                .map_err(|_| "Dados de exclusão global inválidos".to_string())?;
            match excluir_global(
                pool.clone(),
                excluir_global_req
            ).await {
                Ok(_) => Ok("Usuários excluídos com sucesso!".to_string()),
                Err(e) => Err(e)
            }
        },
        "SINCRONIZAR" => {
            let usuarios: Vec<User> = serde_json::from_str(dados)
                .map_err(|_| "Dados de usuários inválidos".to_string())?;
            sincronizar_usuarios(db, pool, usuarios).await?;
            Ok("Usuários sincronizados com sucesso!".to_string())
        },
        "EDITAR" => {
            let edit_req: EditRequest = serde_json::from_str(dados)
                .map_err(|_| "Dados de edição inválidos".to_string())?;
            editar_usuario(db, pool, edit_req).await?;
            Ok("Usuário editado com sucesso!".to_string())
        },
        _ => Err("Comando não reconhecido. Use CRIAR, EXCLUIR, EXCLUIR_GLOBAL, SINCRONIZAR ou EDITAR".to_string())
    }
}