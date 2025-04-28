use std::process::Command;
use log::{info, error};

/// Instala o PostgreSQL usando o gerenciador de pacotes do sistema.
pub async fn instalar_postgres() -> Result<(), String> {
    info!("Verificando se o PostgreSQL está instalado...");

    // Verifica se o comando psql existe
    if which::which("psql").is_ok() {
        info!("PostgreSQL já está instalado.");
        // Mesmo que já esteja instalado, tenta criar o banco
        if let Err(e) = criar_banco_mdulo().await {
            error!("Erro ao criar banco mdulo: {}", e);
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
            // Após instalar, cria o banco mdulo
            if let Err(e) = criar_banco_mdulo().await {
                error!("Erro ao criar banco mdulo: {}", e);
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

/// Cria o banco de dados 'mdulo' usando o usuário postgres do sistema.
pub async fn criar_banco_mdulo() -> Result<(), String> {
    info!("Criando banco de dados 'mdulo' no PostgreSQL...");
    let status = Command::new("sudo")
        .arg("-u").arg("postgres")
        .arg("psql")
        .arg("-c")
        .arg("DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_database WHERE datname = 'mdulo') THEN CREATE DATABASE mdulo; END IF; END $$;")
        .status();
    match status {
        Ok(s) if s.success() => {
            info!("Banco de dados 'mdulo' criado ou já existente.");
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao criar banco de dados 'mdulo', código de saída: {}", s);
            Err("Falha ao criar banco de dados 'mdulo'".into())
        },
        Err(e) => {
            error!("Erro ao criar banco de dados 'mdulo': {}", e);
            Err("Erro ao criar banco de dados 'mdulo'".into())
        }
    }
} 