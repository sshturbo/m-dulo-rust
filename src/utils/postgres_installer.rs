use std::process::Command;
use log::{info, error};
use url::Url;
use crate::config::Config;

/// Instala o PostgreSQL usando o gerenciador de pacotes do sistema.
pub async fn instalar_postgres() -> Result<(), String> {
    info!("Verificando se o PostgreSQL está instalado...");

    // Verifica se o comando psql existe
    if which::which("psql").is_ok() {
        info!("PostgreSQL já está instalado.");
        // Mesmo que já esteja instalado, tenta criar banco/usuário
        if let Err(e) = criar_banco_usuario_url().await {
            error!("Erro ao criar banco/usuário: {}", e);
            return Err(e);
        }
        return Ok(());
    }

    info!("PostgreSQL não encontrado, iniciando instalação...");

    // Atualiza a lista de pacotes
    let status = Command::new("sudo")
        .arg("apt-get").arg("update")
        .status();
    if let Err(e) = status {
        error!("Erro ao atualizar lista de pacotes: {}", e);
        return Err("Erro ao atualizar lista de pacotes".into());
    }

    // Instala o PostgreSQL
    let status = Command::new("sudo")
        .arg("apt-get").arg("install").arg("-y").arg("postgresql").arg("postgresql-contrib")
        .status();
    match status {
        Ok(s) if s.success() => {
            info!("PostgreSQL instalado com sucesso.");
            // Após instalar, cria banco/usuário
            if let Err(e) = criar_banco_usuario_url().await {
                error!("Erro ao criar banco/usuário: {}", e);
                return Err(e);
            }
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao instalar PostgreSQL, código de saída: {}", s);
            Err("Falha ao instalar PostgreSQL".into())
        },
        Err(e) => {
            error!("Erro ao instalar PostgreSQL: {}", e);
            Err("Erro ao instalar PostgreSQL".into())
        }
    }
}

/// Garante que o serviço do PostgreSQL está rodando.
pub async fn iniciar_postgres() -> Result<(), String> {
    info!("Iniciando serviço do PostgreSQL...");
    let status = Command::new("sudo")
        .arg("service").arg("postgresql").arg("start")
        .status();
    match status {
        Ok(s) if s.success() => {
            info!("Serviço do PostgreSQL iniciado.");
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao iniciar serviço do PostgreSQL, código de saída: {}", s);
            Err("Falha ao iniciar serviço do PostgreSQL".into())
        },
        Err(e) => {
            error!("Erro ao iniciar serviço do PostgreSQL: {}", e);
            Err("Erro ao iniciar serviço do PostgreSQL".into())
        }
    }
}

/// Cria banco, usuário e senha a partir da URL do config.json
pub async fn criar_banco_usuario_url() -> Result<(), String> {
    let database_url = &Config::get().database_url;
    let url = Url::parse(database_url).map_err(|e| format!("Erro ao parsear database_url: {}", e))?;

    let usuario = url.username();
    let senha = url.password().unwrap_or("");
    let nome_banco = url.path().trim_start_matches('/');

    info!("Criando usuário '{}' e banco '{}' no PostgreSQL...", usuario, nome_banco);

    // Cria o usuário
    let status = Command::new("sudo")
        .arg("-u").arg("postgres")
        .arg("psql")
        .arg("-c")
        .arg(format!("DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = '{usuario}') THEN CREATE ROLE {usuario} WITH LOGIN PASSWORD '{senha}'; END IF; END $$;"))
        .status();

    if let Err(e) = status {
        error!("Erro ao criar usuário: {}", e);
        return Err("Erro ao criar usuário".into());
    }

    // Cria o banco de dados
    let output = Command::new("sudo")
        .arg("-u").arg("postgres")
        .arg("psql")
        .arg("-c")
        .arg(format!("CREATE DATABASE {} OWNER {};", nome_banco, usuario))
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                info!("Banco de dados '{}' criado com sucesso.", nome_banco);
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("already exists") {
                    info!("Banco de dados '{}' já existe.", nome_banco);
                } else {
                    error!("Falha ao criar banco de dados '{}': {}", nome_banco, stderr.trim());
                    return Err(format!("Falha ao criar banco de dados '{}': {}", nome_banco, stderr.trim()));
                }
            }
        },
        Err(e) => {
            error!("Erro ao criar banco de dados '{}': {}", nome_banco, e);
            return Err("Erro ao criar banco de dados".into());
        }
    }

    // Concede privilégios
    let status = Command::new("sudo")
        .arg("-u").arg("postgres")
        .arg("psql")
        .arg("-c")
        .arg(format!("GRANT ALL PRIVILEGES ON DATABASE {} TO {};", nome_banco, usuario))
        .status();

    if let Err(e) = status {
        error!("Erro ao conceder privilégios: {}", e);
        return Err("Erro ao conceder privilégios".into());
    }

    info!("Usuário, banco e privilégios criados/configurados com sucesso!");
    Ok(())
} 