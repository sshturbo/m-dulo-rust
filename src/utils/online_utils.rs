use std::fs::File;
use std::io::{self, BufRead, Error};
use std::process::Command as StdCommand;
use std::collections::HashMap;
use m_dulo::xray::app::stats::command::stats_service_client::StatsServiceClient;
use m_dulo::xray::app::stats::command::QueryStatsRequest;

#[derive(Debug, Clone)]
pub struct OnlineUser {
    pub login: String,
    pub downlink: Option<String>,
    pub uplink: Option<String>,
    pub tipo: String, // "ssh", "openvpn" ou "xray"
}

#[allow(dead_code)]
pub fn get_users() -> Result<Vec<String>, Error> {
    let mut user_list = Vec::new();

    // === SSH Users ===
    if let Ok(output) = StdCommand::new("ps")
        .arg("aux")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if line.contains("sshd:") && line.contains("[priv]") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 0 {
                    let process_info = parts[10..].join(" ");
                    if let Some(username_part) = process_info.split("sshd: ").nth(1) {
                        if let Some(username) = username_part.split_whitespace().next() {
                            if username != "root" && username != "unknown" && !username.is_empty() {
                                user_list.push(username.to_string());
                            }
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Erro ao rodar 'ps aux' para buscar usuários SSH.");
    }

    // === OpenVPN Users ===
    if let Ok(file) = File::open("/etc/openvpn/openvpn-status.log") {
        for line in io::BufReader::new(file).lines().flatten() {
            if let Some((user, _)) = line.split_once(',') {
                let user = user.trim();
                if !user.is_empty() && user != "root" && user != "unknown" {
                    user_list.push(user.to_string());
                }
            }
        }
    }

    Ok(user_list)
}

#[allow(dead_code)]
pub fn execute_command(command: &str, args: &[&str]) -> Result<(), Error> {
    StdCommand::new(command)
        .args(args)
        .output()?;
    Ok(())
}

pub async fn get_xray_online_users() -> Result<Vec<(String, String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let mut client = StatsServiceClient::connect("http://127.0.0.1:1085").await?;
    let request = tonic::Request::new(QueryStatsRequest {
        pattern: "".to_string(),
        reset: false,
    });
    let response = client.query_stats(request).await?.into_inner();
    let mut users: HashMap<String, (String, String)> = HashMap::new();
    for stat in response.stat {
        let name = stat.name;
        let value = stat.value.to_string();
        if name.starts_with("user>>>") && name.contains(">>>traffic>>>") {
            let parts: Vec<&str> = name.split(">>>").collect();
            if parts.len() >= 4 {
                let usuario = parts[1].to_string();
                let traf_type = parts[3];
                let entry = users.entry(usuario).or_insert((String::new(), String::new()));
                match traf_type {
                    "downlink" => entry.0 = value.clone(),
                    "uplink" => entry.1 = value.clone(),
                    _ => {}
                }
            }
        }
    }
    let result = users.into_iter().map(|(usuario, (down, up))| (usuario, down, up)).collect();
    Ok(result)
}

pub async fn get_all_online_users() -> Result<Vec<OnlineUser>, Box<dyn std::error::Error + Send + Sync>> {
    let mut user_list = Vec::new();

    // === SSH Users ===
    if let Ok(output) = StdCommand::new("ps").arg("aux").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("sshd:") && line.contains("[priv]") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 0 {
                    let process_info = parts[10..].join(" ");
                    if let Some(username_part) = process_info.split("sshd: ").nth(1) {
                        if let Some(username) = username_part.split_whitespace().next() {
                            if username != "root" && username != "unknown" && !username.is_empty() {
                                user_list.push(OnlineUser {
                                    login: username.to_string(),
                                    downlink: None,
                                    uplink: None,
                                    tipo: "ssh".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // === OpenVPN Users ===
    if let Ok(file) = File::open("/etc/openvpn/openvpn-status.log") {
        for line in io::BufReader::new(file).lines().flatten() {
            if let Some((user, _)) = line.split_once(',') {
                let user = user.trim();
                if !user.is_empty() && user != "root" && user != "unknown" {
                    user_list.push(OnlineUser {
                        login: user.to_string(),
                        downlink: None,
                        uplink: None,
                        tipo: "openvpn".to_string(),
                    });
                }
            }
        }
    }

    // === Xray Users ===
    if let Ok(xray_users) = get_xray_online_users().await {
        for (login, downlink, uplink) in xray_users {
            user_list.push(OnlineUser {
                login,
                downlink: Some(downlink),
                uplink: Some(uplink),
                tipo: "xray".to_string(),
            });
        }
    }

    Ok(user_list)
}
