use std::process::Command;
use std::io::Error;

#[allow(dead_code)]
pub fn get_users() -> Result<String, Error> {
    let mut user_list = Vec::new();

    // Busca usuários SSH
    let ssh_output = Command::new("sh")
        .arg("-c")
        .arg(r"ps aux | grep '[s]shd:' | grep '\[priv\]' | awk -F'sshd: ' '{print $2}' | awk '{print $1}' | grep -v '^root$'")
        .output()?;

    let ssh_users = String::from_utf8_lossy(&ssh_output.stdout);
    for user in ssh_users.lines() {
        let user = user.trim();
        if !user.is_empty() && user != "root" && user != "unknown" && !user.contains("sshd") {
            user_list.push(user.to_string());
        }
    }

    // Busca usuários OpenVPN
    if let Ok(openvpn_output) = Command::new("sh")
        .arg("-c")
        .arg("cat /etc/openvpn/openvpn-status.log 2>/dev/null | grep -E '^[^,]+,' | cut -d',' -f1")
        .output()
    {
        let openvpn_users = String::from_utf8_lossy(&openvpn_output.stdout);
        for user in openvpn_users.lines() {
            let user = user.trim();
            if !user.is_empty() && user != "root" && user != "unknown" && !user_list.contains(&user.to_string()) {
                user_list.push(user.to_string());
            }
        }
    }

    Ok(user_list.join(","))
}

#[allow(dead_code)]
pub fn execute_command(command: &str, args: &[&str]) -> Result<(), Error> {
    Command::new(command)
        .args(args)
        .output()?;
    Ok(())
}
