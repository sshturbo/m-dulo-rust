use sqlx::{Pool, Sqlite};
use crate::models::user::User;

pub async fn editar_usuario(_user: User, _pool: &Pool<Sqlite>) -> Result<String, String> {
    Ok("Funcionalidade de edição será implementada em breve".to_string())
}
