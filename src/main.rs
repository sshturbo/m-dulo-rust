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
}
mod ws_handler; 
mod db;
mod utils {
    pub mod restart_v2ray;
    pub mod email;
    pub mod user_utils;
}

use axum::{
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use crate::db::initialize_db;
use crate::ws_handler::handler::websocket_handler;
use env_logger::Env; 
use std::sync::Arc;
use std::collections::HashMap;
use crate::routes::criar::Database;
use tokio::sync::Mutex;
use axum::Extension; 

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    dotenv::dotenv().ok();
    
    let pool = initialize_db()
        .await
        .expect("Falha ao inicializar o banco de dados. Verifique as permissões do diretório.");

    let db: Database = Arc::new(Mutex::new(HashMap::new())); 

    let app = Router::new()
        .route("/", get(websocket_handler))
        .layer(Extension(db)) 
        .with_state(pool.clone()); 

    println!("Modulos rodando no endereço ws://0.0.0.0:9001");
    let listener = TcpListener::bind("0.0.0.0:9001").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
