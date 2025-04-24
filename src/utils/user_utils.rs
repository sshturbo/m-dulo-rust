use chrono::{Duration, Utc};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;
use crate::models::user::User;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use crate::utils::email::gerar_email_aleatorio;
use std::path::Path;
use crate::utils::restart_xray::reiniciar_xray;
use tokio;

pub async fn process_user_data(user: User) -> Result<(), String> {
    adicionar_usuario_sistema(&user.login, &user.senha, user.dias.try_into().unwrap(), user.limite.try_into().unwrap())?;
    match user.tipo.as_str() {
        "xray" => {
            if Path::new("/usr/local/etc/xray/config.json").exists() {
                if let Some(ref uuid) = user.uuid {
                    adicionar_usuario_xray(&user.login, uuid, user.dias.try_into().unwrap())?;
                }
            }
        },
        _ => {
            if Path::new("/etc/v2ray/config.json").exists() {
                if let Some(ref uuid) = user.uuid {
                    if adicionar_uuid_ao_v2ray(uuid, &user.login, user.dias.try_into().unwrap())? {
                        reiniciar_v2ray().await;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn adicionar_usuario_sistema(username: &str, password: &str, dias: u32, sshlimiter: u32) -> Result<(), String> {
    use std::io::Write as IoWrite;

    // Verificar se o usuário já existe
    if Command::new("id").arg(username).output().map_err(|_| "Falha ao verificar usuário".to_string())?.status.success() {
        return Ok(());
    }

    let final_date = (Utc::now() + Duration::days(dias as i64)).format("%Y-%m-%d").to_string();
    // Criptografar a senha
    let perl_cmd = Command::new("perl")
        .arg("-e")
        .arg("print crypt($ARGV[0], 'password')")
        .arg(password)
        .output()
        .map_err(|_| "Falha ao criptografar senha".to_string())?;
    let pass = String::from_utf8_lossy(&perl_cmd.stdout).trim().to_string();

    let status = Command::new("useradd")
        .args(["-e", &final_date, "-M", "-s", "/bin/false", "-p", &pass, username])
        .status()
        .map_err(|_| "Falha ao criar usuário com useradd".to_string())?;
    if !status.success() {
        return Err("Comando useradd falhou".to_string());
    }

    // Registrar no banco de dados
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/root/usuarios.db")
        .map_err(|_| "Falha ao abrir arquivo usuarios.db")?;
    writeln!(file, "{} {}", username, sshlimiter).map_err(|_| "Falha ao escrever no usuarios.db")?;
    std::fs::create_dir_all("/etc/SSHPlus/senha").map_err(|_| "Falha ao criar diretório de senha")?;
    std::fs::write(format!("/etc/SSHPlus/senha/{}", username), password).map_err(|_| "Falha ao criar arquivo de senha")?;
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

pub fn adicionar_usuario_xray(username: &str, uuid: &str, _dias: u32) -> Result<(), String> {
    let config_file = "/usr/local/etc/xray/config.json";
    let backup_file = "/usr/local/etc/xray/config.json.bak";
    let email = username;
    // Verificar se o arquivo existe
    if !Path::new(config_file).exists() {
        return Err("Arquivo de configuração do Xray não encontrado.".to_string());
    }
    // Backup
    std::fs::copy(config_file, backup_file).map_err(|_| "Falha ao criar backup do config.json")?;
    // Novo cliente
    let novo_cliente = serde_json::json!({
        "email": email,
        "id": uuid,
        "level": 0
    });
    // Ler e modificar config
    let json_content = std::fs::read_to_string(config_file).map_err(|_| "Falha ao ler config.json")?;
    let mut json_value: serde_json::Value = serde_json::from_str(&json_content).map_err(|_| "Falha ao parsear JSON")?;
    let mut alterado = false;
    if let Some(inbounds) = json_value["inbounds"].as_array_mut() {
        for inbound in inbounds.iter_mut() {
            if inbound["protocol"] == "vless" {
                if let Some(clients) = inbound["settings"]["clients"].as_array_mut() {
                    clients.push(novo_cliente.clone());
                    alterado = true;
                }
            }
        }
    }
    if !alterado {
        // Reverter backup
        std::fs::copy(backup_file, config_file).ok();
        return Err("Nenhum inbound vless encontrado para adicionar o usuário.".to_string());
    }
    std::fs::write(config_file, serde_json::to_string_pretty(&json_value).map_err(|_| "Erro ao serializar JSON")?)
        .map_err(|_| "Falha ao salvar config.json")?;
    // Chama utilitário async para reiniciar xray
    // Como não podemos await em função sync, apenas dispara em background
    tokio::spawn(async {
        reiniciar_xray().await;
    });
    Ok(())
}

pub async fn remover_uuid_v2ray(uuid: &str) {
    let config_path = "/etc/v2ray/config.json";
    if !std::path::Path::new(config_path).exists() {
        return;
    }
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(inbounds) = json.get_mut("inbounds") {
                if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
                    if let Some(settings) = first_inbound.get_mut("settings") {
                        if let Some(clients) = settings.get_mut("clients") {
                            if let Some(clients_array) = clients.as_array_mut() {
                                clients_array.retain(|client| client["id"].as_str().unwrap_or("") != uuid);
                                if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                                    let _ = std::fs::write(config_path, new_content);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub async fn remover_uuids_xray(uuids: &[String]) {
    let config_path = "/usr/local/etc/xray/config.json";
    if !std::path::Path::new(config_path).exists() {
        return;
    }
    for uuid in uuids {
        let status = Command::new("jq")
            .arg(format!(
                "(.inbounds[] | select(.protocol == \"vless\") | .settings.clients) |= map(select(.id != \"{}\"))",
                uuid
            ))
            .arg(config_path)
            .stdout(std::process::Stdio::piped())
            .output();
        if let Ok(output) = status {
            if output.status.success() {
                // Salva o resultado no arquivo temporário e move para o original
                let tmp_path = "/usr/local/etc/xray/tmp_config.json";
                std::fs::write(tmp_path, &output.stdout).ok();
                std::fs::rename(tmp_path, config_path).ok();
            }
        }
    }
}