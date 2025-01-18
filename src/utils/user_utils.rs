use chrono::{Duration, Utc};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;
use crate::models::user::User;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use crate::utils::email::gerar_email_aleatorio;

pub async fn process_user_data(user: User) -> Result<(), String> {
    adicionar_usuario_sistema(&user.login, &user.senha, user.dias.try_into().unwrap(), user.limite.try_into().unwrap())?;

    if std::path::Path::new("/etc/v2ray/config.json").exists() {
        if let Some(ref uuid) = user.uuid {
            if adicionar_uuid_ao_v2ray(uuid, &user.login, user.dias.try_into().unwrap())? {
                reiniciar_v2ray().await;
            }
        }
    }
    Ok(())
}

pub fn adicionar_usuario_sistema(username: &str, password: &str, dias: u32, sshlimiter: u32) -> Result<(), String> {
    let final_date = (Utc::now() + Duration::days(dias as i64)).format("%Y-%m-%d").to_string();

    Command::new("useradd")
        .args(["-M", "-s", "/bin/false", "-e", &final_date, username])
        .status()
        .map_err(|_| "Falha ao criar usuário com useradd".to_string())
        .and_then(|status| if status.success() { Ok(()) } else { Err("Comando useradd falhou".to_string()) })?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("echo \"{}:{}\" | chpasswd", username, password))
        .status()
        .map_err(|_| "Falha ao definir senha com chpasswd".to_string())
        .and_then(|status| if status.success() { Ok(()) } else { Err("Comando chpasswd falhou".to_string()) })?;

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

pub fn adicionar_uuid_ao_v2ray(uuid: &str, nome_usuario: &str, dias: u32) -> Result<bool, String> {
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
            return Ok(false);
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
