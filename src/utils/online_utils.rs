use std::process::Command;
use std::io::Error;

#[allow(dead_code)]
pub fn get_users() -> Result<String, Error> {
    // Executa o comando para obter os processos relacionados a "priv" em estado "Ss"
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep priv | grep Ss | awk -F 'sshd: ' '{print $2}' | awk -F ' ' '{print $1}'")
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut user_list = Vec::new();

    // Processa a saída do comando para filtrar os usuários
    for username in output_str.lines() {
        // Excluir usuários "root" e "unknown"
        if username != "root" && username != "unknown" {
            user_list.push(username.to_string());
        }
    }

    // Verifica e processa o arquivo do OpenVPN para adicionar usuários
    if let Ok(openvpn_output) = Command::new("grep")
        .arg("-Eo")
        .arg("^[a-zA-Z0-9_-]+,[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+:[0-9]+")
        .arg("/etc/openvpn/openvpn-status.log")
        .output()
    {
        let openvpn_output_str = String::from_utf8_lossy(&openvpn_output.stdout);
        for user in openvpn_output_str.lines() {
            let user = user.trim();
            // Adiciona somente usuários válidos e que não sejam "root" ou "unknown"
            if !user.contains("-c") && !user.is_empty() && user != "root" && user != "unknown" {
                user_list.push(user.to_string());
            }
        }
    }

    // Retorna a lista de usuários como uma string separada por vírgulas
    Ok(user_list.join(","))
}

#[allow(dead_code)]
pub fn execute_command(command: &str, args: &[&str]) -> Result<(), Error> {
    Command::new(command)
        .args(args)
        .output()?;
    Ok(())
}
