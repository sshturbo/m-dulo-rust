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

pub async fn sincronizar_usuarios(db: Database, pool: &Pool<Sqlite>, usuarios: Vec<User>) -> Result<(), String> {
    let mut deve_reiniciar_v2ray = false;

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

pub async fn verificar_e_criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), String> {
    let existing_user = sqlx::query_scalar::<_, String>("SELECT login FROM users WHERE login = ?")
        .bind(&user.login)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Erro ao verificar usuário: {}", e))?;

    if existing_user.is_some() {
        excluir_usuario_sistema(&user.login, &user.uuid, pool).await?;
    }

    criar_usuario(db, pool, user).await
}

pub async fn excluir_usuario_sistema(usuario: &str, uuid: &Option<String>, pool: &Pool<Sqlite>) -> Result<(), String> {
    // Verificar se o usuário existe no sistema
    let output = Command::new("id")
        .arg(usuario)
        .output()
        .expect("Falha ao executar comando");

    if !output.status.success() {
        return Err("Usuário não encontrado no sistema".to_string());
    }

    // Se UUID foi fornecido, tentar remover do V2Ray
    if let Some(uuid) = uuid {
        if v2ray_instalado() {
            remover_uuid_v2ray(uuid).await;
        }
    }

    // Matar todos os processos do usuário
    let _ = Command::new("pkill")
        .args(["-u", usuario])
        .status();

    // Excluir o usuário do sistema
    let _ = Command::new("userdel")
        .arg(usuario)
        .status()
        .expect("Falha ao excluir usuário");

    // Remover do banco de dados
    match sqlx::query("DELETE FROM users WHERE login = ?")
        .bind(usuario)
        .execute(pool)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Erro ao excluir usuário do banco: {}", e))
    }
}

pub async fn criar_usuario(db: Database, pool: &Pool<Sqlite>, user: User) -> Result<(), String> {
    let mut db = db.lock().await;

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

    // Se o arquivo de configuração do V2Ray existir, adiciona UUID
    if v2ray_instalado() {
        if let Some(ref uuid) = uuid {
            adicionar_uuid_ao_v2ray(uuid, username, dias);
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

fn adicionar_uuid_ao_v2ray(uuid: &str, nome_usuario: &str, dias: u32) {
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
                
                if let Ok(_) = fs::write(config_file, serde_json::to_string_pretty(&json_value).unwrap()) {
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
        return; // V2Ray não está instalado
    }

    // Ler e parsear o arquivo de configuração
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

                                // Salvar as alterações
                                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                    if fs::write(config_path, new_content).is_ok() {
                                        // Reiniciar o serviço V2Ray
                                        let _ = Command::new("systemctl")
                                            .args(["restart", "v2ray"])
                                            .status();
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

async fn reiniciar_v2ray() {
    let _ = Command::new("systemctl")
        .arg("restart")
        .arg("v2ray")
        .status()
        .expect("Falha ao reiniciar o serviço V2Ray");
}

fn v2ray_instalado() -> bool {
    std::path::Path::new("/etc/v2ray/config.json").exists()
}
