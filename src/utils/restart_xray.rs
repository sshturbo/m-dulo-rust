use std::process::Command;

pub async fn reiniciar_xray() {
    let status = Command::new("systemctl")
        .arg("restart")
        .arg("xray.service")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✅ Serviço Xray reiniciado com sucesso.");
        },
        Ok(_) | Err(_) => {
            eprintln!("❌ Falha ao reiniciar o serviço Xray. Verifique se está instalado e configurado corretamente.");
        }
    }
} 