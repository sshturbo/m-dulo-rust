use std::process::Command;

pub async fn reiniciar_xray() {
    let check_status = Command::new("systemctl")
        .arg("status")
        .arg("xray.service")
        .status()
        .expect("Falha ao executar o comando systemctl");

    if !check_status.success() {
        eprintln!("❌ Xray não está instalado ou o serviço não está disponível.");
        return;
    }

    let status = Command::new("systemctl")
        .arg("restart")
        .arg("xray.service")
        .status()
        .expect("Falha ao executar o comando systemctl");

    if status.success() {
        println!("✅ Serviço Xray reiniciado com sucesso.");
    } else {
        eprintln!("❌ Falha ao reiniciar o serviço Xray.");
    }
} 