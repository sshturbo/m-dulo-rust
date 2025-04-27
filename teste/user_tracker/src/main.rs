use std::fs::File;
use std::io::{self, BufRead, Error};
use std::process::Command;

fn get_users() -> Result<String, Error> {
    let mut user_list = Vec::new();

    // === SSH Users ===
    if let Ok(output) = Command::new("ps")
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
    } else {
        eprintln!("Arquivo /etc/openvpn/openvpn-status.log não encontrado ou inacessível.");
    }

    // Converte a lista de usuários para uma string separada por vírgulas
    Ok(user_list.join(","))
}

fn main() {
    match get_users() {
        Ok(user_list) => {
            if !user_list.is_empty() {
                println!("Usuários conectados: {}", user_list);
            } else {
                println!("Nenhum usuário encontrado.");
            }
        }
        Err(e) => eprintln!("Erro ao obter usuários: {}", e),
    }
}
