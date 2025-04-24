use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub login: String,
    pub senha: String,
    pub dias: i32,
    pub limite: i32,
    pub uuid: Option<String>,
    pub tipo: String,
}
