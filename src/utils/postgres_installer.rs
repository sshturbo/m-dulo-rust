use std::process::Command;
use log::{info, error};
use url::Url;
use crate::config::Config;
use std::process::Stdio;

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
    info!("Subindo container Docker do PostgreSQL...");
    // Remove container antigo se existir
    let _ = Command::new("sudo")
        .arg("docker").arg("rm").arg("-f").arg("postgres")
        .output();
    // Sobe novo container
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
            info!("Container PostgreSQL iniciado com sucesso.");
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao iniciar container PostgreSQL, código de saída: {}", s);
            Err("Falha ao iniciar container PostgreSQL".into())
        },
        Err(e) => {
            error!("Erro ao iniciar container PostgreSQL: {}", e);
            Err("Erro ao iniciar container PostgreSQL".into())
        }
    }
}

/// Sobe o container do Redis usando Docker
fn subir_redis_docker() -> Result<(), String> {
    info!("Subindo container Docker do Redis...");
    // Remove container antigo se existir
    let _ = Command::new("sudo")
        .arg("docker").arg("rm").arg("-f").arg("redis")
        .output();
    // Sobe novo container
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
            info!("Container Redis iniciado com sucesso.");
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao iniciar container Redis, código de saída: {}", s);
            Err("Falha ao iniciar container Redis".into())
        },
        Err(e) => {
            error!("Erro ao iniciar container Redis: {}", e);
            Err("Erro ao iniciar container Redis".into())
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