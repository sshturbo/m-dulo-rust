use std::process::Command;

pub async fn reiniciar_v2ray() {
    let check_status = Command::new("systemctl")
        .arg("status")
        .arg("v2ray.service")
        .status()
        .expect("Falha ao executar o comando systemctl");

    if !check_status.success() {
        eprintln!("❌ V2Ray não está instalado ou o serviço não está disponível.");
        return;
    }

    let status = Command::new("systemctl")
        .arg("restart")
        .arg("v2ray.service")
        .status()
        .expect("Falha ao executar o comando systemctl");

    if status.success() {
        println!("✅ Serviço V2Ray reiniciado com sucesso.");
    } else {
        eprintln!("❌ Falha ao reiniciar o serviço V2Ray.");
    }
}
