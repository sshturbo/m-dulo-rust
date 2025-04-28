use crate::utils::online_utils::get_users;
use redis::AsyncCommands;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use log::error;
use sqlx::Row;

pub async fn monitor_online_users(mut redis_conn: redis::aio::Connection, pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let start_time = std::time::Instant::now();
        let current_date = chrono::Local::now().naive_local();

        // Obtem a lista de usuários do sistema
        if let Ok(user_list) = get_users() {
            let mut user_count = std::collections::HashMap::new();
            for user in &user_list {
                if user.trim().is_empty() {
                    continue;
                }
                *user_count.entry(user.to_string()).or_insert(0) += 1;
            }

            // Marca todos como Off no Redis
            let online_users: Vec<String> = redis_conn.smembers("online_users").await.unwrap_or_default();
            for login in &online_users {
                let _: () = redis_conn.hset(format!("online:{}", login), "status", "Off").await?;
            }

            // Atualiza apenas os que estão online
            for user in &user_list {
                if !user.trim().is_empty() {
                    let _: () = redis_conn.hset(format!("online:{}", user), "status", "On").await?;
                    let _: () = redis_conn.sadd("online_users", user).await?;
                }
            }

            // Remove usuários que não estão mais online do set
            let online_users: Vec<String> = redis_conn.smembers("online_users").await.unwrap_or_default();
            for login in &online_users {
                let status: String = redis_conn.hget(format!("online:{}", login), "status").await.unwrap_or("Off".to_string());
                if status == "Off" {
                    let _: () = redis_conn.srem("online_users", login).await?;
                }
            }

            // Atualiza ou insere usuários online no Redis
            for (user, count) in &user_count {
                if user.is_empty() {
                    continue;
                }
                // Buscar dono e byid do banco usando sqlx::query
                let row = sqlx::query("SELECT dono, byid FROM users WHERE login = $1")
                    .bind(user)
                    .fetch_optional(pool)
                    .await?;
                let (dono, byid) = if let Some(row) = row {
                    (row.try_get::<String, _>("dono").unwrap_or_default(), row.try_get::<i32, _>("byid").unwrap_or(0))
                } else {
                    (String::new(), 0)
                };
                let _: () = redis_conn.hset_multiple(
                    format!("online:{}", user),
                    &[
                        ("usuarios_online", count.to_string().as_str()),
                        ("status", "On"),
                        ("inicio_sessao", &current_date.format("%d/%m/%Y %H:%M:%S").to_string()),
                        ("dono", &dono),
                        ("byid", &byid.to_string()),
                    ]
                ).await?;
            }
        } else {
            error!("Falha ao obter usuários.");
        }

        let elapsed_time = start_time.elapsed();
        let sleep_duration = if elapsed_time < Duration::from_secs(1) {
            Duration::from_secs(1) - elapsed_time
        } else {
            Duration::from_secs(0)
        };

        sleep(sleep_duration).await;
    }
}