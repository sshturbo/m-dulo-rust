use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub usuario: String,
    pub uuid: Option<String>,
}
