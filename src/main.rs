pub mod models {
    pub mod user;
    pub mod delete;
    pub mod delete_global;
}
mod routes;
mod ws_handler; 
mod db;

use axum::{
    routing::{get, delete},
    Router,
};
use tokio::net::TcpListener;
use crate::db::initialize_db;
use crate::routes::excluir::excluir_usuario;
use crate::routes::excluir_global::excluir_global;
use crate::ws_handler::handler::websocket_handler;
use env_logger;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();
    
    let pool = initialize_db()
        .await
        .expect("Falha ao inicializar o banco de dados. Verifique as permissões do diretório.");
    
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/excluir/:usuario/:uuid", delete(excluir_usuario))
        .route("/excluir/:usuario", delete(excluir_usuario))
        .with_state(pool);

    println!("Servidor WebSocket rodando em ws://127.0.0.1:9001");
    let listener = TcpListener::bind("127.0.0.1:9001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
