use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub senha: String,
    pub dias: u32,
    pub limite: u32,
    pub uuid: Option<String>,
}
