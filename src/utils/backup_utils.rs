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

/// Restaura o backup do banco de dados SQLite do caminho de origem para o destino
pub fn restore_backup(backup_path: &str, db_dir: &str, db_file: &str) -> Result<(), Error> {
    if !Path::new(db_dir).exists() {
        fs::create_dir_all(db_dir)?;
    }
    
    let db_path = format!("{}/{}", db_dir, db_file);
    if Path::new(backup_path).exists() {
        fs::copy(backup_path, &db_path)?;
        Ok(())
    } else {
        Ok(()) // Retorna Ok se não existir backup para restaurar
    }
}