use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub login: String,
    pub senha: String,
    pub dias: u32,
    pub limite: u32,
    pub uuid: Option<String>,
}
