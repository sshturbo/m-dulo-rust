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
use crate::ws_handler::handler::websocket_handler;
use crate::ws_handler::handler::websocket_online_handler; // Adicione esta linha
use env_logger::Env; // Adicione esta linha

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init(); // Modifique esta linha
    dotenv::dotenv().ok();
    
    let pool = initialize_db()
        .await
        .expect("Falha ao inicializar o banco de dados. Verifique as permissões do diretório.");
    
    let app = Router::new()
        .route("/", get(websocket_handler))
        .route("/online", get(websocket_online_handler))
        .with_state(pool);

    println!("Modulos rodando no endereço ws://0.0.0.0:9001");
    let listener = TcpListener::bind("0.0.0.0:9001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
