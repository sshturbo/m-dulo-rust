use std::process::Command;

pub async fn reiniciar_v2ray() {
    let status = Command::new("systemctl")
        .arg("restart")
        .arg("v2ray.service")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✅ Serviço V2Ray reiniciado com sucesso.");
        },
        Ok(_) | Err(_) => {
            eprintln!("❌ Falha ao reiniciar o serviço V2Ray. Verifique se está instalado e configurado corretamente.");
        }
    }
}
