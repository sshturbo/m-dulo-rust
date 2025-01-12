use sqlx::{Pool, Sqlite};
use std::process::Command;
use crate::models::user::User;
use crate::models::edit::EditRequest;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use rand::{distributions::Alphanumeric, Rng};
use chrono::{Duration, Utc};

pub type Database = Arc<Mutex<HashMap<String, User>>>;

pub async fn editar_usuario(
    db: Database,
    pool: &Pool<Sqlite>,
    edit_req: EditRequest
) -> Result<(), String> {
    let mut db = db.lock().await;

    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE login = ?")
        .bind(&edit_req.login_antigo)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Erro ao verificar usuário: {}", e))?;

    if existing_user.is_none() {
        return Err("Usuário antigo não encontrado no banco de dados!".to_string());
    }

    let _ = Command::new("pkill")
        .args(["-u", &edit_req.login_antigo])
        .status();

    let _ = Command::new("userdel")
        .arg(&edit_req.login_antigo)
        .status()
        .expect("Falha ao excluir usuário");

    let new_user = User {
        login: edit_req.login_novo.clone(),
        senha: edit_req.senha.clone(),
        dias: edit_req.dias,
        limite: edit_req.limite,
        uuid: edit_req.uuid.clone(),
    };

    let result = sqlx::query(
        "UPDATE users SET login = ?, senha = ?, dias = ?, limite = ?, uuid = ? WHERE login = ?"
    )
    .bind(&new_user.login)
    .bind(&new_user.senha)
    .bind(new_user.dias as i64)
    .bind(new_user.limite as i64)
    .bind(&new_user.uuid)
    .bind(&edit_req.login_antigo)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            db.insert(new_user.login.clone(), new_user.clone());
            println!("Usuário editado com sucesso!");
            process_user_data(new_user).await;
            Ok(())
        }
        Err(e) => Err(format!("Erro ao atualizar usuário no banco de dados: {}", e))
    }
}

async fn process_user_data(user: User) {
    let username = &user.login;
    let password = &user.senha;
    let dias = user.dias;
    let sshlimiter = user.limite;
    let uuid = user.uuid;

    adicionar_usuario_sistema(username, password, dias, sshlimiter);

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
