use redis::AsyncCommands;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use log::error;
use sqlx::Row;
use crate::utils::online_utils::get_all_online_users;

pub async fn monitor_online_users(mut redis_conn: redis::aio::Connection, pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let start_time = std::time::Instant::now();
        let current_date = chrono::Local::now().naive_local();

        // Obtem a lista de todos os usuários online (SSH, OpenVPN, Xray)
        if let Ok(online_users) = get_all_online_users().await {
            // Marca todos como Off no Redis
            let redis_online_users: Vec<String> = redis_conn.smembers("online_users").await.unwrap_or_default();
            for login in &redis_online_users {
                let _: () = redis_conn.hset(format!("online:{}", login), "status", "Off").await?;
            }

            // Atualiza ou insere usuários online no Redis
            for user in &online_users {
                // Buscar dono, byid e limite do banco usando sqlx::query
                let row = sqlx::query("SELECT dono, byid, limite FROM users WHERE login = $1")
                    .bind(&user.login)
                    .fetch_optional(pool)
                    .await?;
                let (dono, byid, limite) = if let Some(row) = row {
                    (
                        row.try_get::<String, _>("dono").unwrap_or_default(),
                        row.try_get::<i32, _>("byid").unwrap_or(0),
                        row.try_get::<i32, _>("limite").unwrap_or(0),
                    )
                } else {
                    (String::new(), 0, 0)
                };
                // Verifica se inicio_sessao já existe no Redis
                let inicio_sessao: Option<String> = redis_conn.hget(format!("online:{}", user.login), "inicio_sessao").await.ok();
                let inicio_sessao = match inicio_sessao {
                    Some(ref val) if !val.is_empty() => val.clone(),
                    _ => current_date.format("%d/%m/%Y %H:%M:%S").to_string(),
                };
                let mut fields: Vec<(&str, String)> = vec![
                    ("status", "On".to_string()),
                    ("inicio_sessao", inicio_sessao.clone()),
                    ("dono", dono.clone()),
                    ("byid", byid.to_string()),
                    ("limite", limite.to_string()),
                ];
                // Se for Xray, salva também downlink/uplink e faz dupla verificação
                if user.tipo == "xray" {
                    let downlink = user.downlink.clone().unwrap_or_default();
                    let uplink = user.uplink.clone().unwrap_or_default();
                    let prev_downlink: String = redis_conn.hget(format!("online:{}", user.login), "downlink").await.unwrap_or_default();
                    let prev_downlink_prev: String = redis_conn.hget(format!("online:{}", user.login), "downlink_prev").await.unwrap_or_default();
                    let prev_uplink: String = redis_conn.hget(format!("online:{}", user.login), "uplink").await.unwrap_or_default();
                    let prev_uplink_prev: String = redis_conn.hget(format!("online:{}", user.login), "uplink_prev").await.unwrap_or_default();
                    // Atualiza os campos prev
                    let _: () = redis_conn.hset(format!("online:{}", user.login), "downlink_prev", &prev_downlink).await?;
                    let _: () = redis_conn.hset(format!("online:{}", user.login), "uplink_prev", &prev_uplink).await?;
                    // Lógica de dupla verificação
                    let online = (downlink != prev_downlink && prev_downlink != prev_downlink_prev)
                        || (uplink != prev_uplink && prev_uplink != prev_uplink_prev);
                    let status = if online { "On".to_string() } else { "Off".to_string() };
                    fields.push(("downlink", downlink));
                    fields.push(("uplink", uplink));
                    fields[0] = ("status", status);
                }
                let fields_ref: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
                let _: () = redis_conn.hset_multiple(
                    format!("online:{}", user.login),
                    &fields_ref
                ).await?;
                let _: () = redis_conn.sadd("online_users", &user.login).await?;
            }

            // Remove usuários que não estão mais online do set
            let redis_online_users: Vec<String> = redis_conn.smembers("online_users").await.unwrap_or_default();
            for login in &redis_online_users {
                let status: String = redis_conn.hget(format!("online:{}", login), "status").await.unwrap_or("Off".to_string());
                if status == "Off" {
                    let _: () = redis_conn.srem("online_users", login).await?;
                }
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