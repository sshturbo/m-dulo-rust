use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use log::{info, error};
use sqlx::{Pool, Sqlite};
use futures_util::StreamExt;
use std::pin::Pin;
use tokio_stream::wrappers::LinesStream;

pub async fn start_cloudflared_process(pool: Pool<Sqlite>) {
    info!("Iniciando processo do cloudflared...");

    // Verifica se cloudflared está instalado
    if which::which("cloudflared").is_err() {
        info!("Cloudflared não encontrado, instalando...");
        let _ = std::process::Command::new("wget")
            .arg("https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-arm64.deb")
            .status();
        let _ = std::process::Command::new("sudo")
            .arg("dpkg")
            .arg("-i")
            .arg("cloudflared-linux-arm64.deb")
            .status();
        info!("Cloudflared instalado com sucesso");
    }

    loop {
        let mut child = match TokioCommand::new("cloudflared")
            .arg("tunnel")
            .arg("--url")
            .arg("http://localhost:9001")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                error!("Erro ao iniciar cloudflared: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if stdout.is_none() && stderr.is_none() {
            error!("Nenhum output para ler do processo cloudflared");
            break;
        }

        let mut readers: Vec<Pin<Box<dyn futures::Stream<Item = Result<String, std::io::Error>> + Send>>> = Vec::new();

        if let Some(out) = stdout {
            readers.push(LinesStream::new(BufReader::new(out).lines()).boxed());
        }
        if let Some(err) = stderr {
            readers.push(LinesStream::new(BufReader::new(err).lines()).boxed());
        }

        let mut streams = futures::stream::select_all(readers);

        while let Some(line) = streams.next().await {
            if let Ok(line) = line {
                if line.contains("https://") {
                    if let Some(url) = line.split_whitespace()
                        .find(|&word| word.starts_with("https://") && word.contains("trycloudflare.com"))
                    {
                        let subdominio = url.trim().to_string();
                        
                        match crate::db::salvar_subdominio(&pool, &subdominio).await {
                            Ok(_) => {
                                info!("✅ Subdomínio capturado e salvo: {}", subdominio);
                            },
                            Err(e) => error!("❌ Erro ao salvar subdomínio: {}", e),
                        }
                    }
                }
            }
        }

        error!("Conexão com cloudflared perdida, reconectando em 5 segundos...");
        let _ = child.wait().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
