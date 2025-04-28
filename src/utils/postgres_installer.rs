use std::process::Command;
use log::{info, error};
use url::Url;
use crate::config::Config;
use std::fs;

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
        // Instala o Redis se necessário
        instalar_redis()?;
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
            // Instala o Redis se necessário
            instalar_redis()?;
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

fn get_postgres_port() -> Option<u16> {
    let conf_path = "/etc/postgresql/14/main/postgresql.conf";
    if let Ok(content) = fs::read_to_string(conf_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("port") && !line.starts_with("#") {
                if let Some(port_str) = line.split('=').nth(1) {
                    let port = port_str.trim().split_whitespace().next().unwrap_or("");
                    if let Ok(port_num) = port.parse::<u16>() {
                        return Some(port_num);
                    }
                }
            }
        }
    }
    None
}

fn set_postgres_port_5432() -> Result<(), String> {
    let conf_path = "/etc/postgresql/14/main/postgresql.conf";
    let content = fs::read_to_string(conf_path).map_err(|e| format!("Erro ao ler postgresql.conf: {}", e))?;
    let mut new_content = String::new();
    let mut found = false;
    for line in content.lines() {
        if line.trim().starts_with("port") && !line.trim().starts_with("#") {
            new_content.push_str("port = 5432\n");
            found = true;
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }
    if !found {
        new_content.push_str("port = 5432\n");
    }
    fs::write(conf_path, new_content).map_err(|e| format!("Erro ao escrever postgresql.conf: {}", e))?;
    info!("Porta do PostgreSQL ajustada para 5432 no postgresql.conf");
    Ok(())
}

/// Garante que o serviço do PostgreSQL está rodando e loga a porta configurada.
pub async fn iniciar_postgres() -> Result<(), String> {
    let porta = get_postgres_port().unwrap_or(5432);
    info!("Porta configurada no postgresql.conf: {}", porta);
    if porta != 5432 {
        info!("Atenção: o PostgreSQL está configurado para rodar na porta {} (não padrão). Alterando para 5432...", porta);
        set_postgres_port_5432()?;
    }
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

fn instalar_redis() -> Result<(), String> {
    info!("Verificando se o Redis está instalado...");
    if which::which("redis-server").is_ok() {
        info!("Redis já está instalado.");
        return Ok(());
    }
    info!("Redis não encontrado, iniciando instalação...");
    let status = Command::new("sudo")
        .arg("apt-get").arg("install").arg("-y").arg("redis-server")
        .status();
    match status {
        Ok(s) if s.success() => {
            info!("Redis instalado com sucesso.");
            // Habilita o serviço para iniciar automaticamente
            let _ = Command::new("sudo")
                .arg("systemctl").arg("enable").arg("redis-server")
                .status();
            // Inicia o serviço imediatamente
            let _ = Command::new("sudo")
                .arg("systemctl").arg("start").arg("redis-server")
                .status();
            Ok(())
        },
        Ok(s) => {
            error!("Falha ao instalar Redis, código de saída: {}", s);
            Err("Falha ao instalar Redis".into())
        },
        Err(e) => {
            error!("Erro ao instalar Redis: {}", e);
            Err("Erro ao instalar Redis".into())
        }
    }
} 