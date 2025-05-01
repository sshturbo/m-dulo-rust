use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;
use sqlx::PgPool;
use redis::Client as RedisClient;
use tokio::sync::oneshot;
use crate::proxy::{self, ConexoesAtivas};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};
use crate::utils::logging;

pub async fn start_proxy_server(
    addr: &str,
    pool: PgPool,
    redis_client: RedisClient,
    ativas: ConexoesAtivas,
) {
    let listener = TcpListener::bind(addr).await.expect("Erro ao abrir porta do proxy");
    println!("Proxy escutando em {}", addr);
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                logging::log_proxy_nova_conexao(&addr.to_string());
                let pool = pool.clone();
                let redis_client = redis_client.clone();
                let ativas = ativas.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_proxy_conn(stream, pool, redis_client, ativas).await {
                        logging::log_proxy_erro(&e.to_string());
                    }
                });
            }
            Err(e) => logging::log_proxy_erro(&format!("Erro ao aceitar conexão: {}", e)),
        }
    }
}

async fn handle_proxy_conn(
    client_stream: TcpStream,
    pool: PgPool,
    redis_client: RedisClient,
    ativas: ConexoesAtivas,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = client_stream.peer_addr()?.to_string();
    
    // Detecta se é handshake WebSocket (GET ... HTTP/1.1) ou TCP puro
    let mut peek_buf = [0u8; 14];
    let n = client_stream.peek(&mut peek_buf).await?;
    let is_ws = n >= 3 && &peek_buf[..3] == b"GET";

    if is_ws {
        logging::log_proxy_tipo_conexao("WebSocket");
        let ws_stream = accept_async(client_stream).await?;
        handle_ws_vless(ws_stream, pool, redis_client, ativas, &addr).await
    } else {
        logging::log_proxy_tipo_conexao("TCP");
        handle_tcp_vless(client_stream, pool, redis_client, ativas, &addr).await
    }
}

async fn handle_tcp_vless(
    mut client_stream: TcpStream,
    pool: PgPool,
    redis_client: RedisClient,
    ativas: ConexoesAtivas,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    // Lê o handshake VLess
    let mut handshake = [0u8; 17];
    if let Err(e) = client_stream.read_exact(&mut handshake).await {
        logging::log_proxy_erro(&format!("Erro ao ler handshake de {}: {}", addr, e));
        return Ok(());
    }

    let uuid_bytes = &handshake[1..17];
    let uuid = match Uuid::from_slice(uuid_bytes) {
        Ok(u) => u,
        Err(e) => {
            logging::log_proxy_erro(&format!("UUID inválido de {}: {}", addr, e));
            let _ = client_stream.write_all(b"UUID INVALIDO\n").await;
            return Ok(());
        }
    };

    if !proxy::validar_uuid(&pool, &uuid).await? {
        logging::log_proxy_uuid_invalido(&uuid, addr);
        let _ = client_stream.write_all(b"UUID INVALIDO\n").await;
        return Ok(());
    }
    logging::log_proxy_uuid_valido(&uuid, addr);

    let mut redis_conn = redis_client.get_async_connection().await?;
    let (tx, rx) = oneshot::channel();
    proxy::adicionar_conexao(&ativas, uuid, tx, &mut redis_conn).await?;
    
    let mut xray_stream = TcpStream::connect("127.0.0.1:80").await?;
    logging::log_proxy_xray_conectado(&uuid);
    xray_stream.write_all(&handshake).await?;
    
    logging::log_proxy_conexao_estabelecida(&uuid, "TCP");

    let (mut cr, mut cw) = client_stream.split();
    let (mut xr, mut xw) = xray_stream.split();
    let client_to_xray = tokio::io::copy(&mut cr, &mut xw);
    let xray_to_client = tokio::io::copy(&mut xr, &mut cw);
    
    tokio::select! {
        _ = client_to_xray => {
            logging::log_proxy_conexao_encerrada(&uuid, "Cliente desconectou");
        },
        _ = xray_to_client => {
            logging::log_proxy_conexao_encerrada(&uuid, "Xray desconectou");
        },
        _ = rx => {
            logging::log_proxy_conexao_encerrada(&uuid, "Conexão derrubada manualmente");
        },
    }
    
    proxy::remover_conexao(&ativas, &uuid, &mut redis_conn).await?;
    Ok(())
}

async fn handle_ws_vless(
    mut ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
    pool: PgPool,
    redis_client: RedisClient,
    ativas: ConexoesAtivas,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use futures_util::{StreamExt, SinkExt};
    
    // Lê a primeira mensagem binária do cliente
    let msg = ws_stream.next().await;
    let handshake = match msg {
        Some(Ok(Message::Binary(data))) if data.len() >= 17 => data,
        _ => {
            logging::log_proxy_erro(&format!("Handshake WS inválido de {}", addr));
            return Ok(());
        }
    };

    let uuid_bytes = &handshake[1..17];
    let uuid = match Uuid::from_slice(uuid_bytes) {
        Ok(u) => u,
        Err(e) => {
            logging::log_proxy_erro(&format!("UUID WS inválido de {}: {}", addr, e));
            let _ = ws_stream.send(Message::Text("UUID INVALIDO".into())).await;
            return Ok(());
        }
    };

    if !proxy::validar_uuid(&pool, &uuid).await? {
        logging::log_proxy_uuid_invalido(&uuid, addr);
        let _ = ws_stream.send(Message::Text("UUID INVALIDO".into())).await;
        return Ok(());
    }
    logging::log_proxy_uuid_valido(&uuid, addr);

    let mut redis_conn = redis_client.get_async_connection().await?;
    let (tx, rx) = oneshot::channel();
    proxy::adicionar_conexao(&ativas, uuid, tx, &mut redis_conn).await?;
    
    let (mut xray_ws, _) = connect_async("ws://127.0.0.1:80").await?;
    logging::log_proxy_xray_conectado(&uuid);
    xray_ws.send(Message::Binary(handshake)).await?;
    
    logging::log_proxy_conexao_estabelecida(&uuid, "WebSocket");

    let (mut cli_sink, mut cli_stream) = ws_stream.split();
    let (mut xray_sink, mut xray_stream) = xray_ws.split();
    
    let cli_to_xray = async {
        while let Some(msg) = cli_stream.next().await {
            if let Ok(m) = msg {
                if xray_sink.send(m).await.is_err() { break; }
            } else { break; }
        }
    };
    
    let xray_to_cli = async {
        while let Some(msg) = xray_stream.next().await {
            if let Ok(m) = msg {
                if cli_sink.send(m).await.is_err() { break; }
            } else { break; }
        }
    };

    tokio::select! {
        _ = cli_to_xray => {
            logging::log_proxy_conexao_encerrada(&uuid, "Cliente WS desconectou");
        },
        _ = xray_to_cli => {
            logging::log_proxy_conexao_encerrada(&uuid, "Xray WS desconectou");
        },
        _ = rx => {
            logging::log_proxy_conexao_encerrada(&uuid, "Conexão WS derrubada manualmente");
        },
    }
    
    proxy::remover_conexao(&ativas, &uuid, &mut redis_conn).await?;
    Ok(())
} 