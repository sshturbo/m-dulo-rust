use crate::models::user::User;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use rand::{distributions::Alphanumeric, Rng};
use chrono::{Duration, Utc};
use zbus::Connection;
use std::process::Command;

pub type Database = Arc<Mutex<HashMap<String, User>>>;

pub async fn criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), String> {
    let mut db = db.lock().await;

    let existing_user = sqlx::query_scalar::<_, String>("SELECT login FROM users WHERE login = ?")
        .bind(&user.login)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Erro ao verificar usuário: {}", e))?;

    if existing_user.is_some() {
        return Err("Usuário já existe no banco de dados!".to_string());
    }

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
    .map_err(|e| format!("Erro ao inserir usuário: {}", e))?;

    db.insert(user.login.clone(), user.clone());
    println!("Usuário criado com sucesso!");
    process_user_data(user).await?;
    Ok(())
}

pub async fn process_user_data(user: User) -> Result<(), String> {
    adicionar_usuario_sistema(&user.login, &user.senha, user.dias, user.limite)?;

    if std::path::Path::new("/etc/v2ray/config.json").exists() {
        if let Some(ref uuid) = user.uuid {
            if adicionar_uuid_ao_v2ray(uuid, &user.login, user.dias)? {
                let connection = Connection::system().await.map_err(|_| "Falha ao conectar ao D-Bus")?;
                let proxy = zbus::Proxy::new(&connection, "org.freedesktop.systemd1", "/org/freedesktop/systemd1", "org.freedesktop.systemd1.Manager").await.map_err(|_| "Falha ao criar proxy")?;
                proxy.call_method("RestartUnit", &("v2ray.service", "replace")).await.map_err(|_| "Falha ao reiniciar o serviço V2Ray")?;
            }
        }
    }
    Ok(())
}

fn gerar_email_aleatorio(tamanho: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(tamanho)
        .map(char::from)
        .collect::<String>() + "@gmail.com"
}

fn adicionar_usuario_sistema(username: &str, password: &str, dias: u32, sshlimiter: u32) -> Result<(), String> {
    let final_date = (Utc::now() + Duration::days(dias as i64)).format("%Y-%m-%d").to_string();

    Command::new("useradd")
        .args(["-M", "-s", "/bin/false", "-e", &final_date, username])
        .status()
        .map_err(|_| "Falha ao criar usuário com useradd".to_string())?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("echo \"{}:{}\" | chpasswd", username, password))
        .status()
        .map_err(|_| "Falha ao definir senha com chpasswd".to_string())?;

    fs::create_dir_all("/etc/SSHPlus/senha").map_err(|_| "Falha ao criar diretório de senha")?;
    fs::write(format!("/etc/SSHPlus/senha/{}", username), password)
        .map_err(|_| "Falha ao criar arquivo de senha")?;

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/root/usuarios.db")
        .map_err(|_| "Falha ao abrir arquivo usuarios.db")?;

    writeln!(file, "{} {}", username, sshlimiter).map_err(|_| "Falha ao escrever no usuarios.db")?;
    Ok(())
}

fn adicionar_uuid_ao_v2ray(uuid: &str, nome_usuario: &str, dias: u32) -> Result<bool, String> {
    let config_file = "/etc/v2ray/config.json";

    if !std::path::Path::new(config_file).exists() {
        return Err("Arquivo de configuração do V2Ray não encontrado.".to_string());
    }

    let email = gerar_email_aleatorio(10);
    let final_date = (Utc::now() + Duration::days(dias as i64)).format("%Y-%m-%d").to_string();

    let json_content = fs::read_to_string(config_file).map_err(|_| "Falha ao ler o arquivo de configuração.")?;
    let mut json_value: serde_json::Value = serde_json::from_str(&json_content).map_err(|_| "Falha ao parsear o JSON.")?;

    if let Some(clients) = json_value["inbounds"][0]["settings"]["clients"].as_array_mut() {
        if !clients.iter().any(|client| client["id"] == uuid) {
            clients.push(serde_json::json!({
                "id": uuid,
                "alterId": 0,
                "email": email
            }));
        } else {
            return Ok(false);  // UUID já existente
        }
    }

    fs::write(config_file, serde_json::to_string_pretty(&json_value).map_err(|_| "Erro ao serializar JSON")?)
        .map_err(|_| "Falha ao salvar o arquivo de configuração.")?;

    let mut registro_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/etc/SSHPlus/RegV2ray")
        .map_err(|_| "Falha ao abrir o arquivo de registro V2Ray.")?;

    writeln!(registro_file, "{} | {} | {}", uuid, nome_usuario, final_date)
        .map_err(|_| "Falha ao escrever no registro V2Ray.")?;

    Ok(true)
}