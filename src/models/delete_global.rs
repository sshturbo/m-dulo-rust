use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usuario {
    pub usuario: String,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcluirGlobalRequest {
    pub usuarios: Vec<Usuario>,
}
