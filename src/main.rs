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
    pub mod restart_xray;
    pub mod user_utils;
    pub mod online_utils;
    pub mod backup_utils;
}
mod config;

use axum::{
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use crate::db::initialize_db;
use crate::ws_handler::handler::{websocket_handler, websocket_online_handler, websocket_sync_status_handler, websocket_domain_handler};
use env_logger::Env; 
use std::sync::Arc;
use std::collections::HashMap;
use crate::routes::criar::Database;
use tokio::sync::Mutex;
use axum::Extension; 
use crate::routes::online::monitor_online_users;
use crate::utils::backup_utils::restore_backup;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Restaurar backup do banco de dados, se existir
    let backup_path = "/opt/backup-mdulo/database.sqlite";
    let db_dir = "db";
    let db_file = "database.sqlite";

    match restore_backup(backup_path, db_dir, db_file) {
        Ok(_) => {
            if Path::new(backup_path).exists() {
                println!("✅ Backup do banco de dados restaurado com sucesso!");
            } else {
                println!("Nenhum backup encontrado em {}", backup_path);
            }
        },
        Err(e) => eprintln!("Erro ao restaurar backup do banco de dados: {}", e),
    }

    // Carregar configuração do config.json
    config::Config::load_from_file("config.json");
    // Inicializa o logger com o filtro de log configurado
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    // Inicializa o banco de dados antes de qualquer outra coisa
    let pool = initialize_db()
        .await
        .expect("Falha ao inicializar o banco de dados. Verifique as permissões do diretório.");

    // Inicia o processo do cloudflared em uma nova task
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        crate::ws_handler::cloudflared::start_cloudflared_process(pool_clone).await;
    });

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
