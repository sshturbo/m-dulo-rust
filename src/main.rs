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
    pub mod logging;
    pub mod restart_v2ray;
    pub mod restart_xray;
    pub mod user_utils;
    pub mod online_utils;
    pub mod postgres_installer;
}
mod config;

use axum::{
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use crate::db::initialize_db;
use crate::ws_handler::handler::{websocket_handler, websocket_online_handler, websocket_sync_status_handler, websocket_domain_handler};
use std::sync::Arc;
use std::collections::HashMap;
use crate::routes::criar::Database;
use tokio::sync::Mutex;
use axum::Extension; 
use crate::routes::online::monitor_online_users;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Carregar configuração do config.json
    config::Config::load_from_file("config.json");

    // Ativa/desativa logs conforme config
    if let Some(false) = config::Config::get().logs_enabled {
        utils::logging::disable_logs();
    } else {
        utils::logging::enable_logs();
    }
    utils::logging::init_logging();

    // Instala e inicia o PostgreSQL antes de qualquer coisa
    if let Err(e) = utils::postgres_installer::instalar_postgres().await {
        eprintln!("Erro ao instalar PostgreSQL: {}", e);
        return Ok(());
    }

    // Inicializa o banco de dados antes de qualquer outra coisa
    let pool = initialize_db()
        .await
        .expect("Falha ao inicializar o banco de dados. Verifique as permissões do diretório.");

    // Inicializa conexão Redis
    let redis_client = redis::Client::open("redis://127.0.0.1/").expect("Erro ao conectar no Redis");

    // Inicia o processo do cloudflared em uma nova task
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        crate::ws_handler::cloudflared::start_cloudflared_process(pool_clone).await;
    });

    // Agora que o banco de dados está inicializado, podemos iniciar a tarefa do monitoramento de usuários
    let redis_conn_clone = redis_client.get_async_connection().await.expect("Erro ao obter conexão Redis");
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        if let Err(err) = monitor_online_users(redis_conn_clone, &pool_clone).await {
            eprintln!("Erro ao monitorar usuários: {:?}", err);
        }
    });

    // Inicializa o banco de dados com o tipo Database para uso no app
    let db: Database = Arc::new(Mutex::new(HashMap::new())); 

    // Criação das rotas do servidor
    let app = Router::new()
        .route("/", get(websocket_handler))
        .route("/online", get(websocket_online_handler))
        .route("/sync-status", get(websocket_sync_status_handler))
        .route("/domain", get(websocket_domain_handler))
        .layer(Extension(db)) 
        .with_state(pool.clone()); 

    // Inicia o servidor na porta 9001
    println!("Modulos rodando no endereço ws://0.0.0.0:9001");
    let listener = TcpListener::bind("0.0.0.0:9001").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
