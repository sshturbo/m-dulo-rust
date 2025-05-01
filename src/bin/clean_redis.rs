use redis::AsyncCommands;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Conectar ao Redis
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_async_connection().await?;
    
    println!("Limpando dados de usuários online do Redis...");
    
    // 1. Obter todos os usuários online
    let online_users: Vec<String> = con.smembers("online_users").await?;
    println!("Encontrados {} usuários online", online_users.len());
    
    // 2. Remover todas as chaves de usuários online
    for login in &online_users {
        // Padrão para encontrar todas as chaves do usuário
        let pattern = format!("online:{}:*", login);
        let keys: Vec<String> = con.keys(&pattern).await?;
        
        println!("Removendo {} chaves para o usuário {}", keys.len(), login);
        
        // Remover cada chave individualmente
        for key in keys {
            let _: () = con.del(&key).await?;
        }
        
        // Remover a chave principal do usuário
        let _: () = con.del(format!("online:{}", login)).await?;
    }
    
    // 3. Limpar o set de usuários online
    let _: () = con.del("online_users").await?;
    
    println!("Limpeza concluída com sucesso!");
    
    Ok(())
} 