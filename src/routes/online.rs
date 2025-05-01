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
                let uuid = user.downlink.as_ref().or(user.uplink.as_ref()).unwrap_or(&"".to_string()).clone();
                let uuid = if let Some(uuid) = user.login.split(':').nth(1) { uuid.to_string() } else { uuid };
                let key = format!("online:{}:{}", user.login, uuid);
                let inicio_sessao: Option<String> = redis_conn.hget(&key, "inicio_sessao").await.ok();
                let inicio_sessao = match inicio_sessao {
                    Some(ref val) if !val.is_empty() => val.clone(),
                    _ => current_date.format("%d/%m/%Y %H:%M:%S").to_string(),
                };
                let mut fields: Vec<(&str, String)> = vec![
                    ("dono", dono.clone()),
                    ("byid", byid.to_string()),
                    ("limite", limite.to_string()),
                    ("tipo", user.tipo.clone()),
                ];
                if user.tipo == "xray" {
                    let downlink = user.downlink.clone().unwrap_or_default();
                    let uplink = user.uplink.clone().unwrap_or_default();
                    let saved_downlink: String = redis_conn.hget(&key, "downlink").await.unwrap_or("0".to_string());
                    let saved_uplink: String = redis_conn.hget(&key, "uplink").await.unwrap_or("0".to_string());
                    let saved_downlink_val: u64 = saved_downlink.parse().unwrap_or(0);
                    let saved_uplink_val: u64 = saved_uplink.parse().unwrap_or(0);
                    let downlink_val: u64 = downlink.parse().unwrap_or(0);
                    let uplink_val: u64 = uplink.parse().unwrap_or(0);
                    let mut no_change_count: u32 = redis_conn.hget(&key, "no_change_count").await.unwrap_or(0);
                    let mut status = "On".to_string();
                    let min_bytes = 1024 * 5; // Reduzindo para 5KB para melhor compatibilidade com o intervalo de 2s
                    let prev_status: String = redis_conn.hget(&key, "status").await.unwrap_or("Off".to_string());
                    let mut inicio_sessao_val = redis_conn.hget(&key, "inicio_sessao").await.unwrap_or_else(|_| current_date.format("%d/%m/%Y %H:%M:%S").to_string());
                    
                    // Verifica se houve mudança significativa no tráfego
                    let traffic_changed = downlink_val > saved_downlink_val + min_bytes || uplink_val > saved_uplink_val + min_bytes;
                    let has_minimum_traffic = downlink_val > min_bytes || uplink_val > min_bytes;
                    
                    if traffic_changed && has_minimum_traffic {
                        status = "On".to_string();
                        no_change_count = 0;
                        fields.push(("downlink", downlink.clone()));
                        fields.push(("uplink", uplink.clone()));
                        if prev_status == "Off" {
                            inicio_sessao_val = current_date.format("%d/%m/%Y %H:%M:%S").to_string();
                        }
                    } else {
                        no_change_count += 1;
                        // Ajustando para considerar o intervalo de 2 segundos do handler
                        if no_change_count >= 3 { // 3 verificações = ~6 segundos (2s * 3)
                            status = "Off".to_string();
                        }
                        fields.push(("downlink", saved_downlink));
                        fields.push(("uplink", saved_uplink));
                    }
                    fields.push(("status", status.clone()));
                    fields.push(("no_change_count", no_change_count.to_string()));
                    fields.push(("inicio_sessao", inicio_sessao_val.clone()));
                    fields.push(("last_seen", chrono::Local::now().timestamp().to_string()));
                    let fields_ref: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
                    let _: () = redis_conn.hset_multiple(&key, &fields_ref).await?;
                    let _: () = redis_conn.sadd("online_users", &user.login).await?;
                    // Remove imediatamente se ficou Off
                    if status == "Off" {
                        let _: () = redis_conn.del(&key).await?;
                        let _: () = redis_conn.srem("online_users", &user.login).await?;
                        continue; // Não conta para conexoes_simultaneas
                    }
                } else {
                    // SSH/OpenVPN: lógica antiga
                    fields.push(("status", "On".to_string()));
                    fields.push(("inicio_sessao", inicio_sessao.clone()));
                    let fields_ref: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
                    let _: () = redis_conn.hset_multiple(&key, &fields_ref).await?;
                    let _: () = redis_conn.sadd("online_users", &user.login).await?;
                }
                // Atualiza usuarios_online para o login (apenas status On)
                let pattern = format!("online:{}:*", user.login);
                let keys: Vec<String> = redis_conn.keys(pattern).await.unwrap_or_default();
                let mut usuarios_online = 0;
                for k in &keys {
                    let st: String = redis_conn.hget(k, "status").await.unwrap_or("Off".to_string());
                    if st == "On" {
                        usuarios_online += 1;
                    }
                }
                let _: () = redis_conn.hset(&key, "usuarios_online", usuarios_online).await?;

                let limite_usize = limite as usize;
                if (user.tipo == "ssh" || user.tipo == "openvpn") && usuarios_online > limite_usize && limite > 0 {
                    let _ = std::process::Command::new("pkill")
                        .arg("-u")
                        .arg(&user.login)
                        .output();
                }
            }

            // Timeout para Xray e cleanup geral: remove qualquer chave online:{login}:* não atualizada (last_seen > 8s)
            let redis_online_users: Vec<String> = redis_conn.smembers("online_users").await.unwrap_or_default();
            for login in &redis_online_users {
                let pattern = format!("online:{}:*", login);
                let keys: Vec<String> = redis_conn.keys(&pattern).await.unwrap_or_default();
                for key in keys {
                    let last_seen: i64 = redis_conn.hget(&key, "last_seen").await.unwrap_or(0);
                    let now = chrono::Local::now().timestamp();
                    if now - last_seen > 8 { // Aumentando para 8 segundos (4 intervalos de 2s)
                        let _: () = redis_conn.del(&key).await?;
                        let _: () = redis_conn.srem("online_users", login).await?;
                    }
                }
            }

            // Remove usuários que não estão mais online do set
            let redis_online_users: Vec<String> = redis_conn.smembers("online_users").await.unwrap_or_default();
            for login in &redis_online_users {
                let status: String = redis_conn.hget(format!("online:{}", login), "status").await.unwrap_or("Off".to_string());
                if status == "Off" {
                    // Remove toda a hash do usuário
                    let _: () = redis_conn.del(format!("online:{}", login)).await?;
                    // Remove do set de online
                    let _: () = redis_conn.srem("online_users", login).await?;
                }
            }
        } else {
            error!("Falha ao obter usuários.");
        }

        let elapsed_time = start_time.elapsed();
        let sleep_duration = if elapsed_time < Duration::from_millis(500) {
            Duration::from_millis(500) - elapsed_time // Reduzindo o intervalo de sleep para 500ms
        } else {
            Duration::from_millis(0)
        };

        sleep(sleep_duration).await;
    }
}