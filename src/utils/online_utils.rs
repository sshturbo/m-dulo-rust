use std::fs::File;
use std::io::{self, BufRead, Error};
use std::process::Command;

#[allow(dead_code)]
pub fn get_users() -> Result<Vec<String>, Error> {
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
    }

    Ok(user_list)
}

#[allow(dead_code)]
pub fn execute_command(command: &str, args: &[&str]) -> Result<(), Error> {
    Command::new(command)
        .args(args)
        .output()?;
    Ok(())
}
