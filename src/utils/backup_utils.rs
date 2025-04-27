use std::fs;
use std::io::Error;
use std::path::Path;

/// Faz backup do banco de dados SQLite para o diretório de destino, criando o diretório se necessário.
pub fn backup_database(origem: &str, destino_dir: &str, nome_arquivo: &str) -> Result<(), Error> {
    let destino_path = Path::new(destino_dir);
    if !destino_path.exists() {
        fs::create_dir_all(destino_path)?;
    }
    let destino_arquivo = destino_path.join(nome_arquivo);
    fs::copy(origem, destino_arquivo)?;
    Ok(())
} 