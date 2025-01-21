use crate::models::user::User;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::process::Command;
use std::fs::{self, OpenOptions};
use std::io::Write;
use chrono::{Duration, Utc};
use crate::utils::restart_v2ray::reiniciar_v2ray;
use crate::utils::email::gerar_email_aleatorio;
use thiserror::Error;

pub type Database = Arc<Mutex<HashMap<String, User>>>;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Erro ao verificar usuário: {0}")]
    VerificarUsuario(String),
    #[error("Erro ao excluir usuário do banco: {0}")]
    ExcluirUsuarioBanco(String),
    #[error("Erro ao inserir usuário no banco de dados: {0}")]
    InserirUsuarioBanco(String),
    #[error("Falha ao executar comando")]
    FalhaComando,
    #[error("Erro ao processar dados do usuário")]
    ProcessarDadosUsuario,
}

pub async fn sincronizar_usuarios(db: Database, pool: &Pool<Sqlite>, usuarios: Vec<User>) -> Result<(), SyncError> {
    let mut deve_reiniciar_v2ray = false;

    // Excluir todos os usuários recebidos do banco de dados
    for user in &usuarios {
        if let Err(e) = excluir_usuario_sistema(&user.login, &user.uuid, pool).await {
            eprintln!("Erro ao excluir usuário {}: {}", user.login, e);
        }
    }

    // Recriar todos os usuários
    for user in usuarios {
        let result = verificar_e_criar_usuario(db.clone(), pool, user.clone()).await;
        if let Err(e) = result {
            eprintln!("Erro ao sincronizar usuário {}: {}", user.login, e);
        } else if user.uuid.is_some() && v2ray_instalado() {
            deve_reiniciar_v2ray = true;
        }
    }

    if deve_reiniciar_v2ray && v2ray_instalado() {
        reiniciar_v2ray().await;
    }

    Ok(())
}

pub async fn verificar_e_criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), SyncError> {
    let existing_user = sqlx::query_scalar::<_, String>("SELECT login FROM users WHERE login = ?")
        .bind(&user.login)
        .fetch_optional(pool)
        .await
        .map_err(|e| SyncError::VerificarUsuario(e.to_string()))?;

    if existing_user.is_some() {
        if let Err(e) = excluir_usuario_sistema(&user.login, &user.uuid, pool).await {
            return Err(e);
        }
    }

    criar_usuario(db, pool, user).await
}

pub async fn excluir_usuario_sistema(usuario: &str, uuid: &Option<String>, pool: &Pool<Sqlite>) -> Result<(), SyncError> {
    let output = Command::new("id")
        .arg(usuario)
        .output()
        .map_err(|_| SyncError::FalhaComando)?;

    if !output.status.success() {
        // Não retornar erro se o usuário não for encontrado no sistema
        eprintln!("Usuário {} não encontrado no sistema", usuario);
    } else {
        if let Some(uuid) = uuid {
            if v2ray_instalado() {
                remover_uuid_v2ray(uuid).await;
            }
        }

        let _ = Command::new("pkill")
            .args(["-u", usuario])
            .status();

        let _ = Command::new("userdel")
            .arg(usuario)
            .status()
            .map_err(|_| SyncError::FalhaComando)?;
    }

    // Excluir o usuário do banco de dados
    sqlx::query("DELETE FROM users WHERE login = ?")
        .bind(usuario)
        .execute(pool)
        .await
        .map_err(|e| SyncError::ExcluirUsuarioBanco(e.to_string()))?;

    Ok(())
}

pub async fn criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), SyncError> {
    let mut db = db.lock().await;

    sqlx::query(
        "INSERT INTO users (login, senha, dias, limite, uuid) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user.login)
    .bind(&user.senha)
    .bind(user.dias as i64)
    .bind(user.limite as i64)
    .bind(&user.uuid)
    .execute(pool)
    .await
    .map_err(|e| SyncError::InserirUsuarioBanco(e.to_string()))?;

    db.insert(user.login.clone(), user.clone());
    println!("Usuário criado com sucesso!");
    process_user_data(user).await.map_err(|_| SyncError::ProcessarDadosUsuario)?;
    Ok(())
}

pub async fn process_user_data(user: User) -> Result<(), SyncError> {
    let username = &user.login;
    let password = &user.senha;
    let dias = user.dias;
    let sshlimiter = user.limite;
    let uuid = user.uuid;

    adicionar_usuario_sistema(username, password, dias.try_into().unwrap(), sshlimiter.try_into().unwrap());

    if v2ray_instalado() {
        if let Some(ref uuid) = uuid {
            adicionar_uuid_ao_v2ray(uuid, username, dias.try_into().unwrap()).await;
        }
    }

    Ok(())
}

fn adicionar_usuario_sistema(username: &str, password: &str, dias: u32, sshlimiter: u32) {
    let final_date = (Utc::now() + Duration::days(dias as i64)).format("%Y-%m-%d").to_string();

    let _ = Command::new("useradd")
        .args([
            "-M",
            "-s", "/bin/false",
            "-e", &final_date,
            username
        ])
        .status()
        .expect("Falha ao criar usuário");

    let echo_password = Command::new("echo")
        .arg(format!("{}:{}", username, password))
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Falha ao iniciar echo");

    let _ = Command::new("chpasswd")
        .stdin(echo_password.stdout.unwrap())
        .status()
        .expect("Falha ao definir senha");

    // Verificar e criar diretórios
    let password_dir = "/etc/SSHPlus/senha";
    fs::create_dir_all(password_dir).expect("Falha ao criar diretórios");

    let password_file_path = format!("/etc/SSHPlus/senha/{}", username);
    fs::write(&password_file_path, password).expect("Falha ao criar arquivo de senha");

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/root/usuarios.db")
        .unwrap();
    writeln!(file, "{} {}", username, sshlimiter).unwrap();
}

async fn adicionar_uuid_ao_v2ray(uuid: &str, nome_usuario: &str, dias: u32) {
    let config_file = "/etc/v2ray/config.json";
    
    if !std::path::Path::new(config_file).exists() {
        println!("Arquivo de configuração do V2Ray não encontrado. V2Ray parece não estar instalado.");
        return;
    }

    let email = gerar_email_aleatorio(10);
    let final_date = Command::new("date")
        .arg("+%Y-%m-%d")
        .arg(format!("-d +{} days", dias))
        .output()
        .expect("Falha ao calcular a data de expiração");
    let final_date = String::from_utf8(final_date.stdout).unwrap().trim().to_string();

    if let Ok(json_content) = fs::read_to_string(config_file) {
        if let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(&json_content) {
            if let Some(clients) = json_value["inbounds"][0]["settings"]["clients"].as_array_mut() {
                if !clients.iter().any(|client| client["id"] == uuid) {
                    clients.push(serde_json::json!({
                        "id": uuid,
                        "alterId": 0,
                        "email": email
                    }));
                }
                
                if fs::write(config_file, serde_json::to_string_pretty(&json_value).unwrap()).is_ok() {
                    if let Ok(mut registro_file) = OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open("/etc/SSHPlus/RegV2ray") 
                    {
                        let _ = writeln!(registro_file, "{} | {} | {}", uuid, nome_usuario, final_date);
                        println!("UUID adicionado com sucesso ao V2Ray!");
                    }
                }
            }
        }
    }
}

async fn remover_uuid_v2ray(uuid: &str) {
    let config_path = "/etc/v2ray/config.json";
    
    if !std::path::Path::new(config_path).exists() {
        return; 
    }

    if let Ok(content) = fs::read_to_string(config_path) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(inbounds) = json.get_mut("inbounds") {
                if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                    if let Some(settings) = first_inbound.get_mut("settings") {
                        if let Some(clients) = settings.get_mut("clients") {
                            if let Some(clients_array) = clients.as_array_mut() {
                                // Remover o cliente com o UUID especificado
                                clients_array.retain(|client| {
                                    client["id"].as_str().unwrap_or("") != uuid
                                });

                                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                    if fs::write(config_path, new_content).is_ok() {
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn v2ray_instalado() -> bool {
    std::path::Path::new("/etc/v2ray/config.json").exists()
}
