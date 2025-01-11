use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditRequest {
    pub login_antigo: String,
    pub login_novo: String,
    pub senha: String,
    pub dias: u32,
    pub limite: u32,
    pub uuid: Option<String>, 
}
