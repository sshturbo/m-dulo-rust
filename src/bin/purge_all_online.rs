use redis::AsyncCommands;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Conectar ao Redis
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_async_connection().await?;
    
    println!("Iniciando limpeza completa de dados de usuários online...");
    
    // 1. Encontrar todas as chaves com padrão "online:*"
    let all_keys: Vec<String> = con.keys("online:*").await?;
    println!("Encontradas {} chaves com padrão 'online:*'", all_keys.len());
    
    // 2. Remover todas as chaves encontradas
    for key in &all_keys {
        let _: () = con.del(key).await?;
    }
    
    // 3. Garantir que o set de usuários online seja removido
    let _: () = con.del("online_users").await?;
    
    println!("Limpeza concluída! Removidas {} chaves no total.", all_keys.len() + 1);
    
    Ok(())
} 