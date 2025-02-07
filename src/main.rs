pub mod models {
    pub mod user;
    pub mod delete;
    pub mod delete_global;
    pub mod edit;
}
mod routes {
    pub mod criar;
    pub mod excluir;
    pub mod excluir_global;
    pub mod sincronizar;
    pub mod editar;
    pub mod online; 
    pub mod online_monitor;
}
mod ws_handler; 
mod db;
mod utils {
    pub mod restart_v2ray;
    pub mod email;
    pub mod user_utils;
    pub mod online_utils;
}

use axum::{
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use crate::db::initialize_db;
use crate::ws_handler::handler::{websocket_handler, websocket_online_handler};
use env_logger::Env; 
use std::sync::Arc;
use std::collections::HashMap;
use crate::routes::criar::Database;
use tokio::sync::Mutex;
use axum::Extension; 
use crate::routes::online::monitor_online_users;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializa o logger com o filtro de log configurado
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    dotenv::dotenv().ok();
    
    // Inicializa o banco de dados antes de qualquer outra coisa
    let pool = initialize_db()
        .await
        .expect("Falha ao inicializar o banco de dados. Verifique as permissões do diretório.");

    // Agora que o banco de dados está inicializado, podemos iniciar a tarefa do monitoramento de usuários
    let cloned_pool = pool.clone();
    tokio::spawn(async move {
        if let Err(err) = monitor_online_users(cloned_pool).await {
            eprintln!("Erro ao monitorar usuários: {:?}", err);
        }
    });

    // Inicializa o banco de dados com o tipo Database para uso no app
    let db: Database = Arc::new(Mutex::new(HashMap::new())); 

    // Criação das rotas do servidor
    let app = Router::new()
        .route("/", get(websocket_handler))
        .route("/online", get(websocket_online_handler))
        .layer(Extension(db)) 
        .with_state(pool.clone()); 

    // Inicia o servidor na porta 9001
    println!("Modulos rodando no endereço ws://0.0.0.0:9001");
    let listener = TcpListener::bind("0.0.0.0:9001").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
