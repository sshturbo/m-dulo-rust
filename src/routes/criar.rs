use crate::models::user::User;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::process::Command;
use std::fs::{self, OpenOptions};
use std::io::Write;
use rand::{distributions::Alphanumeric, Rng};
use chrono::{Duration, Utc};

pub type Database = Arc<Mutex<HashMap<String, User>>>;

pub async fn criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), String> {
    let mut db = db.lock().await;

    // Versão sem macro, usando query normal
    let existing_user = sqlx::query_scalar::<_, String>("SELECT login FROM users WHERE login = ?")
        .bind(&user.login)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Erro ao verificar usuário: {}", e))?;

    if existing_user.is_some() {
        return Err("Usuário já existe no banco de dados!".to_string());
    }

    // Continuar com a inserção
    let result = sqlx::query(
        "INSERT INTO users (login, senha, dias, limite, uuid) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user.login)
    .bind(&user.senha)
    .bind(user.dias as i64)
    .bind(user.limite as i64)
    .bind(&user.uuid)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            db.insert(user.login.clone(), user.clone());
            println!("Usuário criado com sucesso!");
            process_user_data(user).await;
            Ok(())
        }
        Err(e) => Err(format!("Erro ao inserir usuário no banco de dados: {}", e))
    }
}

pub async fn process_user_data(user: User) {
    let username = &user.login;
    let password = &user.senha;
    let dias = user.dias;
    let sshlimiter = user.limite;
    let uuid = user.uuid;

    adicionar_usuario_sistema(username, password, dias, sshlimiter);

    // Se o arquivo de configuração do V2Ray existir, adiciona UUID e reinicia o serviço
    if std::path::Path::new("/etc/v2ray/config.json").exists() {
        if let Some(ref uuid) = uuid {
            if adicionar_uuid_ao_v2ray(uuid, username, dias) {
                let _ = Command::new("systemctl")
                    .arg("restart")
                    .arg("v2ray")
                    .status()
                    .expect("Falha ao reiniciar o serviço V2Ray");
            }
        }
    }
}

fn gerar_email_aleatorio(tamanho: usize) -> String {
    let local: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(tamanho)
        .map(char::from)
        .collect();
    format!("{}@gmail.com", local)
}

fn adicionar_usuario_sistema(username: &str, password: &str, dias: u32, sshlimiter: u32) {
    let final_date = (Utc::now() + Duration::days(dias as i64)).format("%Y-%m-%d").to_string();

    // Criar usuário usando comando useradd
    let _ = Command::new("useradd")
        .args([
            "-M",
            "-s", "/bin/false",
            "-e", &final_date,
            username
        ])
        .status()
        .expect("Falha ao criar usuário");

    // Definir senha do usuário
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

    // Criar arquivo com o nome de usuário e senha
    let password_file_path = format!("/etc/SSHPlus/senha/{}", username);
    fs::write(&password_file_path, password).expect("Falha ao criar arquivo de senha");

    // Adicionar ao arquivo usuarios.db
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/root/usuarios.db")
        .unwrap();
    writeln!(file, "{} {}", username, sshlimiter).unwrap();
}

fn adicionar_uuid_ao_v2ray(uuid: &str, nome_usuario: &str, dias: u32) -> bool {
    let config_file = "/etc/v2ray/config.json";
    
    if !std::path::Path::new(config_file).exists() {
        println!("Arquivo de configuração do V2Ray não encontrado. V2Ray parece não estar instalado.");
        return false;
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
                
                if let Ok(_) = fs::write(config_file, serde_json::to_string_pretty(&json_value).unwrap()) {
                    if let Ok(mut registro_file) = OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open("/etc/SSHPlus/RegV2ray") 
                    {
                        let _ = writeln!(registro_file, "{} | {} | {}", uuid, nome_usuario, final_date);
                        println!("UUID adicionado com sucesso ao V2Ray!");
                        return true;
                    }
                }
            }
        }
    }
    false
}
