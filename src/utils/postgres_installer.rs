use std::process::Command;
use log::{info, error};
use url::Url;
use crate::config::Config;
use std::process::Stdio;
use tokio::time::{sleep, Duration};
use tokio_postgres::NoTls;

/// Instala o Docker se necessário
fn instalar_docker() -> Result<(), String> {
    if which::which("docker").is_ok() {
        info!("Docker já está instalado.");
        return Ok(());
    }
    info!("Docker não encontrado, instalando...");
    let status = Command::new("sudo")
        .arg("apt-get").arg("update")
        .status();
    if let Err(e) = status {
        error!("Erro ao atualizar lista de pacotes: {}", e);
        return Err("Erro ao atualizar lista de pacotes".into());
    }
    let status = Command::new("sudo")
        .arg("apt-get").arg("install").arg("-y").arg("docker.io")
        .status();
    match status {
        Ok(s) if s.success() => {
            info!("Docker instalado com sucesso.");
            let _ = Command::new("sudo").arg("systemctl").arg("enable").arg("--now").arg("docker").status();
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao instalar Docker, código de saída: {}", s);
            Err("Falha ao instalar Docker".into())
        },
        Err(e) => {
            error!("Erro ao instalar Docker: {}", e);
            Err("Erro ao instalar Docker".into())
        }
    }
}

/// Sobe o container do PostgreSQL usando Docker
fn subir_postgres_docker() -> Result<(), String> {
    let database_url = &Config::get().database_url;
    let url = Url::parse(database_url).map_err(|e| format!("Erro ao parsear database_url: {}", e))?;
    let usuario = url.username();
    let senha = url.password().unwrap_or("");
    let nome_banco = url.path().trim_start_matches('/');
    info!("Verificando container Docker do PostgreSQL...");
    // Verifica se o container já existe
    let check = Command::new("sudo")
        .arg("docker").arg("ps").arg("-a")
        .arg("--format").arg("{{.Names}}")
        .output()
        .map_err(|e| format!("Erro ao checar containers: {}", e))?;
    let containers = String::from_utf8_lossy(&check.stdout);
    if containers.lines().any(|l| l == "postgres") {
        info!("Container PostgreSQL já existe, iniciando...");
        let status = Command::new("sudo")
            .arg("docker").arg("start").arg("postgres")
            .status();
        match status {
            Ok(s) if s.success() => {
                info!("Container PostgreSQL iniciado com sucesso.");
                return Ok(());
            },
            Ok(s) => {
                error!("Falha ao iniciar container PostgreSQL, código de saída: {}", s);
                return Err("Falha ao iniciar container PostgreSQL".into());
            },
            Err(e) => {
                error!("Erro ao iniciar container PostgreSQL: {}", e);
                return Err("Erro ao iniciar container PostgreSQL".into());
            }
        }
    }
    // Sobe novo container se não existir
    info!("Container PostgreSQL não existe, criando novo...");
    let status = Command::new("sudo")
        .arg("docker").arg("run").arg("-d")
        .arg("--name").arg("postgres")
        .arg("-e").arg(format!("POSTGRES_USER={}", usuario))
        .arg("-e").arg(format!("POSTGRES_PASSWORD={}", senha))
        .arg("-e").arg(format!("POSTGRES_DB={}", nome_banco))
        .arg("-p").arg("5432:5432")
        .arg("--restart").arg("unless-stopped")
        .arg("postgres:15")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {
            info!("Container PostgreSQL criado e iniciado com sucesso.");
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao criar container PostgreSQL, código de saída: {}", s);
            Err("Falha ao criar container PostgreSQL".into())
        },
        Err(e) => {
            error!("Erro ao criar container PostgreSQL: {}", e);
            Err("Erro ao criar container PostgreSQL".into())
        }
    }
}

/// Sobe o container do Redis usando Docker
fn subir_redis_docker() -> Result<(), String> {
    info!("Verificando container Docker do Redis...");
    // Verifica se o container já existe
    let check = Command::new("sudo")
        .arg("docker").arg("ps").arg("-a")
        .arg("--format").arg("{{.Names}}")
        .output()
        .map_err(|e| format!("Erro ao checar containers: {}", e))?;
    let containers = String::from_utf8_lossy(&check.stdout);
    if containers.lines().any(|l| l == "redis") {
        info!("Container Redis já existe, iniciando...");
        let status = Command::new("sudo")
            .arg("docker").arg("start").arg("redis")
            .status();
        match status {
            Ok(s) if s.success() => {
                info!("Container Redis iniciado com sucesso.");
                return Ok(());
            },
            Ok(s) => {
                error!("Falha ao iniciar container Redis, código de saída: {}", s);
                return Err("Falha ao iniciar container Redis".into());
            },
            Err(e) => {
                error!("Erro ao iniciar container Redis: {}", e);
                return Err("Erro ao iniciar container Redis".into());
            }
        }
    }
    // Sobe novo container se não existir
    info!("Container Redis não existe, criando novo...");
    let status = Command::new("sudo")
        .arg("docker").arg("run").arg("-d")
        .arg("--name").arg("redis")
        .arg("-p").arg("6379:6379")
        .arg("--restart").arg("unless-stopped")
        .arg("redis:7")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {
            info!("Container Redis criado e iniciado com sucesso.");
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao criar container Redis, código de saída: {}", s);
            Err("Falha ao criar container Redis".into())
        },
        Err(e) => {
            error!("Erro ao criar container Redis: {}", e);
            Err("Erro ao criar container Redis".into())
        }
    }
}

/// Instala e sobe PostgreSQL e Redis via Docker
pub async fn instalar_postgres() -> Result<(), String> {
    instalar_docker()?;
    subir_postgres_docker()?;
    subir_redis_docker()?;
    Ok(())
}

/// Aguarda até o PostgreSQL estar pronto para aceitar conexões
pub async fn aguardar_postgres_pronto(database_url: &str, tentativas: u32, intervalo: u64) -> Result<(), String> {
    for _ in 0..tentativas {
        match tokio_postgres::connect(database_url, NoTls).await {
            Ok((client, connection)) => {
                drop(client);
                drop(connection);
                return Ok(());
            }
            Err(_) => {
                sleep(Duration::from_secs(intervalo)).await;
            }
        }
    }
    Err("PostgreSQL não ficou pronto a tempo.".into())
} 