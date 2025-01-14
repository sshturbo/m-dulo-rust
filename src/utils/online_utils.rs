use std::process::Command;
use std::io::{self};

pub fn get_users() -> Result<String, io::Error> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep priv | grep Ss")
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut user_list = Vec::new();

    for line in output_str.lines() {
        if !line.contains("priv") {
            continue;
        }
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() >= 12 {
            let username = columns[11].trim();
            if !username.contains("-c") {
                user_list.push(username.to_string());
            }
        }
    }

    if let Ok(_) = Command::new("grep").arg("-q").arg("^[a-zA-Z0-9_-]+,[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+:[0-9]+").arg("/etc/openvpn/openvpn-status.log").output() {
        if let Ok(openvpn_output) = Command::new("grep").arg("-Eo").arg("^[a-zA-Z0-9_-]+,[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+:[0-9]+").arg("/etc/openvpn/openvpn-status.log").output() {
            let openvpn_output_str = String::from_utf8_lossy(&openvpn_output.stdout);
            for user in openvpn_output_str.lines() {
                let user = user.trim();
                if !user.contains("-c") {
                    user_list.push(user.to_string());
                }
            }
        }
    }

    Ok(user_list.join(","))
}
