use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};
use uuid::Uuid;
use sqlx::PgPool;
use redis::Client as RedisClient;
use tokio::sync::oneshot;
use crate::proxy::{self, ConexoesAtivas};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};
use crate::utils::logging;
use crate::config;
use std::io;
use std::os::unix::io::AsRawFd;
use std::os::fd::BorrowedFd;
use std::pin::Pin;
use nix::sys::socket::{setsockopt, sockopt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::BytesMut;
use httparse;

const BUFFER_SIZE: usize = 32 * 1024; // 32KB
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(300); 
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30); 
const MAX_HEADERS: usize = 32;

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
                
                // Configurar socket
                if let Err(e) = configure_socket(&stream) {
                    logging::log_proxy_erro(&format!("Erro ao configurar socket: {}", e));
                    continue;
                }
                
                tokio::spawn(async move {
                    if let Err(e) = handle_proxy_conn(stream, pool, redis_client, ativas).await {
                        logging::log_proxy_erro(&format!("Erro na conexão: {}", e));
                    }
                });
            }
            Err(e) => logging::log_proxy_erro(&format!("Erro ao aceitar conexão: {}", e)),
        }
    }
}

fn configure_socket(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?; // Desativa o algoritmo de Nagle
    // Configura keepalive usando BorrowedFd
    unsafe {
        let fd = BorrowedFd::borrow_raw(stream.as_raw_fd());
        if let Err(e) = setsockopt(&fd, sockopt::KeepAlive, &true) {
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }
    }
    Ok(())
}

async fn handle_proxy_conn(
    client_stream: TcpStream,
    pool: PgPool,
    redis_client: RedisClient,
    ativas: ConexoesAtivas,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = client_stream.peer_addr()?.to_string();
    
    // Buffer para detectar o tipo de conexão
    let mut peek_buf = [0u8; 14];
    let n = client_stream.peek(&mut peek_buf).await?;
    
    if n >= 3 && &peek_buf[..3] == b"GET" {
        logging::log_proxy_tipo_conexao("WebSocket");
        let ws_stream = accept_async(client_stream).await?;
        handle_ws_vless(ws_stream, pool, redis_client, ativas, &addr).await
    } else if n >= 4 && (&peek_buf[..4] == b"POST" || &peek_buf[..4] == b"GET ") {
        logging::log_proxy_tipo_conexao("XHTTP");
        handle_xhttp(client_stream, pool, redis_client, ativas, &addr).await
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
    
    // Lê o handshake VLess com timeout
    let mut handshake = [0u8; 17];
    match timeout(Duration::from_secs(5), client_stream.read_exact(&mut handshake)).await {
        Ok(Ok(_)) => (),
        Ok(Err(e)) => {
            logging::log_proxy_erro(&format!("Erro ao ler handshake de {}: {}", addr, e));
            return Ok(());
        }
        Err(_) => {
            logging::log_proxy_erro(&format!("Timeout ao ler handshake de {}", addr));
            return Ok(());
        }
    }

    let uuid_bytes = &handshake[1..17];
    let uuid = match Uuid::from_slice(uuid_bytes) {
        Ok(u) => {
            println!("[PROXY] UUID recebido (hex): {:?}", uuid_bytes);
            println!("[PROXY] UUID parseado: {}", u);
            u
        },
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
    
    let xray_addr = format!("127.0.0.1:{}", config::Config::get().xray_port);
    let mut xray_stream = TcpStream::connect(&xray_addr).await?;
    configure_socket(&xray_stream)?;
    
    logging::log_proxy_xray_conectado(&uuid);
    xray_stream.write_all(&handshake).await?;
    
    logging::log_proxy_conexao_estabelecida(&uuid, "TCP");

    let (mut cr, mut cw) = client_stream.split();
    let (mut xr, mut xw) = xray_stream.split();

    let mut buf_client = vec![0u8; BUFFER_SIZE];
    let mut buf_xray = vec![0u8; BUFFER_SIZE];

    let mut rx = Some(rx);
    loop {
        tokio::select! {
            result = cr.read(&mut buf_client) => {
                match result {
                    Ok(0) => {
                        logging::log_proxy_conexao_encerrada(&uuid, "Cliente desconectou normalmente");
                        break;
                    }
                    Ok(n) => {
                        if let Err(e) = xw.write_all(&buf_client[..n]).await {
                            logging::log_proxy_erro(&format!("Erro ao enviar dados para Xray: {}", e));
                            break;
                        }
                    }
                    Err(e) => {
                        logging::log_proxy_erro(&format!("Erro ao ler do cliente: {}", e));
                        break;
                    }
                }
            }
            result = xr.read(&mut buf_xray) => {
                match result {
                    Ok(0) => {
                        logging::log_proxy_conexao_encerrada(&uuid, "Xray desconectou normalmente");
                        break;
                    }
                    Ok(n) => {
                        if let Err(e) = cw.write_all(&buf_xray[..n]).await {
                            logging::log_proxy_erro(&format!("Erro ao enviar dados para cliente: {}", e));
                            break;
                        }
                    }
                    Err(e) => {
                        logging::log_proxy_erro(&format!("Erro ao ler do Xray: {}", e));
                        break;
                    }
                }
            }
            _ = async { if let Some(r) = rx.take() { r.await } else { std::future::pending().await } } => {
                logging::log_proxy_conexao_encerrada(&uuid, "Conexão derrubada manualmente");
                break;
            }
            _ = tokio::time::sleep(KEEPALIVE_INTERVAL) => {
                // Enviar keepalive
                if let Err(e) = xw.write_all(b"\0").await {
                    logging::log_proxy_erro(&format!("Erro ao enviar keepalive: {}", e));
                    break;
                }
            }
        }
    }
    
    proxy::remover_conexao(&ativas, &uuid, &mut redis_conn).await?;
    Ok(())
}

async fn handle_ws_vless(
    ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
    pool: PgPool,
    redis_client: RedisClient,
    ativas: ConexoesAtivas,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use futures_util::{StreamExt, SinkExt};
    
    let mut ws_stream = ws_stream;
    
    // Lê a primeira mensagem binária do cliente com timeout
    let msg = match timeout(Duration::from_secs(5), ws_stream.next()).await {
        Ok(Some(msg)) => match msg {
            Ok(msg) => msg,
            Err(e) => {
                logging::log_proxy_erro(&format!("Erro no WebSocket de {}: {}", addr, e));
                return Ok(());
            }
        },
        Ok(None) => {
            logging::log_proxy_erro(&format!("Conexão WS fechada por {}", addr));
            return Ok(());
        }
        Err(_) => {
            logging::log_proxy_erro(&format!("Timeout ao ler handshake de {}", addr));
            return Ok(());
        }
    };

    let handshake = match msg {
        Message::Binary(data) if data.len() >= 17 => data,
        _ => {
            logging::log_proxy_erro(&format!("Handshake WS inválido de {}", addr));
            return Ok(());
        }
    };

    let uuid_bytes = &handshake[1..17];
    let uuid = match Uuid::from_slice(uuid_bytes) {
        Ok(u) => {
            println!("[PROXY] UUID recebido (hex): {:?}", uuid_bytes);
            println!("[PROXY] UUID parseado: {}", u);
            u
        },
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
    
    let xray_addr = format!("ws://127.0.0.1:{}", config::Config::get().xray_port);
    let (mut xray_ws, _) = connect_async(&xray_addr).await?;
    
    logging::log_proxy_xray_conectado(&uuid);
    xray_ws.send(Message::Binary(handshake)).await?;
    
    logging::log_proxy_conexao_estabelecida(&uuid, "WebSocket");

    let (mut cli_sink, mut cli_stream) = ws_stream.split();
    let (mut xray_sink, mut xray_stream) = xray_ws.split();
    
    let mut last_activity = tokio::time::Instant::now();
    
    let mut rx = Pin::new(Box::new(rx));
    loop {
        tokio::select! {
            msg = cli_stream.next() => {
                match msg {
                    Some(Ok(m)) => {
                        last_activity = tokio::time::Instant::now();
                        if xray_sink.send(m).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        logging::log_proxy_erro(&format!("Erro no WebSocket do cliente: {}", e));
                        break;
                    }
                    None => {
                        logging::log_proxy_conexao_encerrada(&uuid, "Cliente WS desconectou");
                        break;
                    }
                }
            }
            msg = xray_stream.next() => {
                match msg {
                    Some(Ok(m)) => {
                        last_activity = tokio::time::Instant::now();
                        if cli_sink.send(m).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        logging::log_proxy_erro(&format!("Erro no WebSocket do Xray: {}", e));
                        break;
                    }
                    None => {
                        logging::log_proxy_conexao_encerrada(&uuid, "Xray WS desconectou");
                        break;
                    }
                }
            }
            _ = &mut rx => {
                logging::log_proxy_conexao_encerrada(&uuid, "Conexão WS derrubada manualmente");
                break;
            }
            _ = tokio::time::sleep(KEEPALIVE_INTERVAL) => {
                // Verificar inatividade
                if last_activity.elapsed() > CONNECTION_TIMEOUT {
                    logging::log_proxy_conexao_encerrada(&uuid, "Timeout por inatividade");
                    break;
                }
                // Enviar ping WebSocket
                if let Err(e) = cli_sink.send(Message::Ping(vec![])).await {
                    logging::log_proxy_erro(&format!("Erro ao enviar ping WS: {}", e));
                    break;
                }
            }
        }
    }
    
    proxy::remover_conexao(&ativas, &uuid, &mut redis_conn).await?;
    Ok(())
}

fn parse_uuid_from_buf<'a>(buf: &'a [u8], headers: &'a mut [httparse::Header<'a>; MAX_HEADERS]) -> Option<Uuid> {
    let mut req = httparse::Request::new(headers);
    if let Ok(status) = req.parse(buf) {
        if status.is_complete() {
            if let Some(uuid_str) = req.headers.iter()
                .find(|h| h.name.eq_ignore_ascii_case("X-UUID"))
                .map(|h| std::str::from_utf8(h.value).ok())
                .flatten()
                .or_else(|| {
                    req.path.and_then(|p| {
                        p.split('/').find(|s| Uuid::parse_str(s).is_ok())
                    })
                })
            {
                return Uuid::parse_str(uuid_str).ok();
            }
        }
    }
    None
}

async fn handle_xhttp(
    mut client_stream: TcpStream,
    pool: PgPool,
    redis_client: RedisClient,
    ativas: ConexoesAtivas,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(4096);
    let uuid = loop {
        let bytes_read = client_stream.read_buf(&mut buf).await?;
        if bytes_read == 0 {
            return Ok(());
        }
        let buf_clone = buf.clone();
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        if let Some(uuid) = parse_uuid_from_buf(&buf_clone, &mut headers) {
            break uuid;
        }
    };
    println!("[PROXY] UUID recebido: {}", uuid);
    if !proxy::validar_uuid(&pool, &uuid).await? {
        logging::log_proxy_uuid_invalido(&uuid, addr);
        let response = "HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nUUID Inválido";
        client_stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }
    logging::log_proxy_uuid_valido(&uuid, addr);
    let mut redis_conn = redis_client.get_async_connection().await?;
    let (tx, rx) = oneshot::channel();
    proxy::adicionar_conexao(&ativas, uuid, tx, &mut redis_conn).await?;
    let xray_addr = format!("127.0.0.1:{}", config::Config::get().xray_port);
    let mut xray_stream = TcpStream::connect(&xray_addr).await?;
    configure_socket(&xray_stream)?;
    logging::log_proxy_xray_conectado(&uuid);
    // Enviar cabeçalhos originais para o Xray
    xray_stream.write_all(&buf).await?;
    logging::log_proxy_conexao_estabelecida(&uuid, "XHTTP");
    let (mut cr, mut cw) = client_stream.split();
    let (mut xr, mut xw) = xray_stream.split();
    let mut buf_client = vec![0u8; BUFFER_SIZE];
    let mut buf_xray = vec![0u8; BUFFER_SIZE];
    let mut rx = Some(rx);
    loop {
        tokio::select! {
            result = cr.read(&mut buf_client) => {
                match result {
                    Ok(0) => {
                        logging::log_proxy_conexao_encerrada(&uuid, "Cliente XHTTP desconectou");
                        break;
                    }
                    Ok(n) => {
                        if let Err(e) = xw.write_all(&buf_client[..n]).await {
                            logging::log_proxy_erro(&format!("Erro ao enviar dados para Xray: {}", e));
                            break;
                        }
                    }
                    Err(e) => {
                        logging::log_proxy_erro(&format!("Erro ao ler do cliente XHTTP: {}", e));
                        break;
                    }
                }
            }
            result = xr.read(&mut buf_xray) => {
                match result {
                    Ok(0) => {
                        logging::log_proxy_conexao_encerrada(&uuid, "Xray desconectou");
                        break;
                    }
                    Ok(n) => {
                        if let Err(e) = cw.write_all(&buf_xray[..n]).await {
                            logging::log_proxy_erro(&format!("Erro ao enviar dados para cliente XHTTP: {}", e));
                            break;
                        }
                    }
                    Err(e) => {
                        logging::log_proxy_erro(&format!("Erro ao ler do Xray: {}", e));
                        break;
                    }
                }
            }
            _ = async { if let Some(r) = rx.take() { r.await } else { std::future::pending().await } } => {
                logging::log_proxy_conexao_encerrada(&uuid, "Conexão XHTTP derrubada manualmente");
                break;
            }
            _ = tokio::time::sleep(KEEPALIVE_INTERVAL) => {
                // Enviar keepalive HTTP
                if let Err(e) = cw.write_all(b"\r\n").await {
                    logging::log_proxy_erro(&format!("Erro ao enviar keepalive XHTTP: {}", e));
                    break;
                }
            }
        }
    }
    proxy::remover_conexao(&ativas, &uuid, &mut redis_conn).await?;
    Ok(())
} 