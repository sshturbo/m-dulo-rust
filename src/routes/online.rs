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
                    ("usuarios_online", "1".to_string()),
                ];
                // Se for Xray, salva também downlink/uplink e faz dupla verificação
                if user.tipo == "xray" {
                    let downlink = user.downlink.clone().unwrap_or_default();
                    let uplink = user.uplink.clone().unwrap_or_default();
                    // Recupera histórico dos últimos 2 valores de downlink/uplink
                    let mut downlink_hist = vec![downlink.clone()];
                    let mut uplink_hist = vec![uplink.clone()];
                    for i in 1..2 {
                        let key = if i == 1 { "downlink_prev".to_string() } else { format!("downlink_prev{}", i) };
                        let val: String = redis_conn.hget(format!("online:{}", user.login), key.as_str()).await.unwrap_or_default();
                        downlink_hist.push(val);
                        let key = if i == 1 { "uplink_prev".to_string() } else { format!("uplink_prev{}", i) };
                        let val: String = redis_conn.hget(format!("online:{}", user.login), key.as_str()).await.unwrap_or_default();
                        uplink_hist.push(val);
                    }
                    // Atualiza histórico no Redis
                    for i in (1..2).rev() {
                        let prev_key = if i == 1 { "downlink_prev".to_string() } else { format!("downlink_prev{}", i) };
                        let prev_val = &downlink_hist[i-1];
                        let _: () = redis_conn.hset(format!("online:{}", user.login), prev_key.as_str(), prev_val).await?;
                        let prev_key = if i == 1 { "uplink_prev".to_string() } else { format!("uplink_prev{}", i) };
                        let prev_val = &uplink_hist[i-1];
                        let _: () = redis_conn.hset(format!("online:{}", user.login), prev_key.as_str(), prev_val).await?;
                    }
                    // Lógica: só marca online se houver pelo menos dois valores diferentes no histórico
                    let unique_down: std::collections::HashSet<_> = downlink_hist.iter().collect();
                    let unique_up: std::collections::HashSet<_> = uplink_hist.iter().collect();
                    let online = unique_down.len() > 1 || unique_up.len() > 1;
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