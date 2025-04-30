use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use log::{info, error};
use sqlx::PgPool;
use futures_util::StreamExt;
use std::pin::Pin;
use tokio_stream::wrappers::LinesStream;
use std::fs;
use serde::Deserialize;
use rand::{distributions::Alphanumeric, Rng};
use crate::db::buscar_subdominio;

/// Instala o cloudflared usando o método recomendado via repositório APT.
#[allow(dead_code)]
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

#[derive(Deserialize)]
struct Config {
    cloudflare_api_key: String,
    cloudflare_domain: String,
}

pub async fn start_cloudflared_process(pool: PgPool) {
    info!("Iniciando processo do cloudflared...");

    // Ler config.json
    let config_str = fs::read_to_string("config.json").expect("Falha ao ler config.json");
    let config: Config = serde_json::from_str(&config_str).expect("Falha ao parsear config.json");

    // Exportar variável de ambiente
    std::env::set_var("CLOUDFLARE_API_TOKEN", &config.cloudflare_api_key);

    // Verifica se já existe subdomínio/túnel salvo no banco
    let subdominio_salvo = buscar_subdominio(&pool).await.ok().flatten();
    let (tunnel_name, _full_domain) = if let Some(subdominio) = subdominio_salvo {
        // Extrai o nome do túnel do subdomínio salvo (antes do primeiro ponto)
        let tunnel_name = subdominio.split('.').next().unwrap_or("").to_string();
        (tunnel_name, subdominio)
    } else {
        // Gerar subdomínio aleatório
        let subdomain: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        let full_domain = format!("{}.{}", subdomain, config.cloudflare_domain);
        let tunnel_name = subdomain.clone();

        // Criar túnel autenticado
        let status = tokio::process::Command::new("cloudflared")
            .arg("tunnel")
            .arg("create")
            .arg(&tunnel_name)
            .status()
            .await
            .expect("Falha ao criar túnel");
        if !status.success() {
            error!("Falha ao criar túnel cloudflared");
            return;
        }

        // Associar subdomínio
        let status = tokio::process::Command::new("cloudflared")
            .arg("tunnel")
            .arg("route")
            .arg("dns")
            .arg(&tunnel_name)
            .arg(&full_domain)
            .status()
            .await
            .expect("Falha ao associar subdomínio");
        if !status.success() {
            error!("Falha ao associar subdomínio cloudflared");
            return;
        }

        // Salva no banco
        if let Err(e) = crate::db::salvar_subdominio(&pool, &full_domain).await {
            error!("Erro ao salvar subdomínio no banco: {}", e);
        }

        (tunnel_name, full_domain)
    };

    // Rodar o túnel sempre apontando para http://localhost:9001
    let mut child = match tokio::process::Command::new("cloudflared")
        .arg("tunnel")
        .arg("run")
        .arg(&tunnel_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            error!("Erro ao iniciar cloudflared: {}", e);
            return;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    if stdout.is_none() && stderr.is_none() {
        error!("Nenhum output para ler do processo cloudflared");
        return;
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
            info!("cloudflared: {}", line);
        }
    }

    error!("Conexão com cloudflared perdida, reconectando em 5 segundos...");
    let _ = child.wait().await;
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
}
