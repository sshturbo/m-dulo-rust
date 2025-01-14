use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usuario {
    pub usuario: String,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcluirGlobalRequest {
    pub usuarios: Vec<Usuario>,
}

