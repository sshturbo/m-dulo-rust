use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use log::{info, error};
use sqlx::PgPool;
use futures_util::StreamExt;
use std::pin::Pin;
use tokio_stream::wrappers::LinesStream;

/// Instala o cloudflared usando o método recomendado via repositório APT.
async fn instalar_cloudflared() -> Result<(), String> {
    use std::process::Command;
    use log::error;

    // 1. Cria o diretório de keyrings
    let status = Command::new("sudo")
        .arg("mkdir").arg("-p").arg("--mode=0755").arg("/usr/share/keyrings")
        .status();
    if let Err(e) = status {
        error!("Erro ao criar diretório de keyrings: {}", e);
        return Err("Erro ao criar diretório de keyrings".into());
    }

    // 2. Baixa e adiciona a chave GPG
    let status = Command::new("bash")
        .arg("-c")
        .arg("curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg | sudo tee /usr/share/keyrings/cloudflare-main.gpg >/dev/null")
        .status();
    if let Err(e) = status {
        error!("Erro ao baixar chave GPG: {}", e);
        return Err("Erro ao baixar chave GPG".into());
    }

    // 3. Adiciona o repositório do cloudflared
    let status = Command::new("bash")
        .arg("-c")
        .arg("echo 'deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared any main' | sudo tee /etc/apt/sources.list.d/cloudflared.list")
        .status();
    if let Err(e) = status {
        error!("Erro ao adicionar repositório cloudflared: {}", e);
        return Err("Erro ao adicionar repositório cloudflared".into());
    }

    // 4. Atualiza a lista de pacotes
    let status = Command::new("sudo")
        .arg("apt-get").arg("update")
        .status();
    if let Err(e) = status {
        error!("Erro ao atualizar lista de pacotes: {}", e);
        return Err("Erro ao atualizar lista de pacotes".into());
    }

    // 5. Instala o cloudflared
    let status = Command::new("sudo")
        .arg("apt-get").arg("install").arg("-y").arg("cloudflared")
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            error!("Falha ao instalar cloudflared, código de saída: {}", s);
            Err("Falha ao instalar cloudflared".into())
        },
        Err(e) => {
            error!("Erro ao instalar cloudflared: {}", e);
            Err("Erro ao instalar cloudflared".into())
        }
    }
}

pub async fn start_cloudflared_process(pool: PgPool) {
    info!("Iniciando processo do cloudflared...");

    // Verifica se cloudflared está instalado
    if which::which("cloudflared").is_err() {
        info!("Cloudflared não encontrado, iniciando instalação...");
        match instalar_cloudflared().await {
            Ok(_) => info!("Cloudflared instalado com sucesso"),
            Err(e) => {
                error!("Erro na instalação do cloudflared: {}", e);
                return;
            }
        }
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
