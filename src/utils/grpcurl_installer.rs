use std::process::Command;
use std::fs;
use crate::utils::logging;

pub fn install_grpcurl() -> Result<(), Box<dyn std::error::Error>> {
    logging::init_logging();
    log::info!("Iniciando verificação/instalação do grpcurl...");
    // Detecta arquitetura
    let arch = String::from_utf8(Command::new("uname").arg("-m").output()?.stdout)?.trim().to_string();
    let arch_str = match arch.as_str() {
        "x86_64" => "linux_x86_64",
        "aarch64" => "linux_arm64",
        _ => {
            log::error!("Arquitetura não suportada: {}", arch);
            return Err("Arquitetura não suportada".into());
        }
    };
    // Pega a última versão
    let version_output = Command::new("curl")
        .args(["-s", "https://api.github.com/repos/fullstorydev/grpcurl/releases/latest"])
        .output()?;
    let version_json = String::from_utf8(version_output.stdout)?;
    let version = version_json
        .lines()
        .find(|l| l.contains("tag_name"))
        .and_then(|l| l.split('"').nth(3))
        .ok_or("Não foi possível obter a versão do grpcurl")?;
    let url = format!(
        "https://github.com/fullstorydev/grpcurl/releases/download/{}/grpcurl_{}_{}.tar.gz",
        version,
        version.trim_start_matches('v'),
        arch_str
    );
    let tar_file = format!("grpcurl_{}_{}.tar.gz", version.trim_start_matches('v'), arch_str);
    log::info!("Baixando grpcurl de {}", url);
    // Baixa o arquivo
    Command::new("wget").arg(&url).status()?;
    // Extrai
    Command::new("tar").args(["-xzf", &tar_file]).status()?;
    // Move para /usr/local/bin
    Command::new("sudo").args(["mv", "grpcurl", "/usr/local/bin/"]).status()?;
    // Remove o tar.gz
    let _ = fs::remove_file(&tar_file);
    // Testa instalação
    Command::new("grpcurl").arg("--version").status()?;
    log::info!("grpcurl instalado com sucesso!");
    Ok(())
} 