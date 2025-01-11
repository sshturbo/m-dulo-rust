use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usuario {
    pub usuario: String,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcluirGlobalRequest {
    // Lista de usuários a serem excluídos globalmente
    pub usuarios: Vec<Usuario>,
}

