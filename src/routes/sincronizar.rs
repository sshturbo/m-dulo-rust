use sqlx::{Pool, Sqlite};

pub async fn sincronizar_usuarios(_pool: &Pool<Sqlite>) -> Result<String, String> {
    Ok("Funcionalidade de sincronização será implementada em breve".to_string())
}
