use redis::AsyncCommands;
use chrono::{NaiveDateTime, Local};
use serde_json::json;
use crate::utils::online_utils::get_all_online_users;

pub async fn monitor_users(mut redis_conn: redis::aio::Connection) -> Result<serde_json::Value, redis::RedisError> {
    let mut users = Vec::new();
    let current_time = Local::now().naive_local();
    if let Ok(online_users) = get_all_online_users().await {
        for user in online_users {
            // Conta conexões simultâneas para o login
            let pattern = format!("online:{}:*", user.login);
            let keys: Vec<String> = redis_conn.keys(pattern).await.unwrap_or_default();
            let usuarios_online = keys.len();
            // Pega a sessão mais recente (opcional: pode melhorar para pegar a mais antiga ou outra lógica)
            let key = keys.get(0).cloned().unwrap_or_else(|| format!("online:{}", user.login));
            let user_data: Option<redis::Value> = redis_conn.hgetall(&key).await.ok();
            let mut map = std::collections::HashMap::new();
            if let Some(redis::Value::Bulk(ref fields)) = user_data {
                let mut i = 0;
                while i + 1 < fields.len() {
                    if let (redis::Value::Data(ref k), redis::Value::Data(ref v)) = (&fields[i], &fields[i+1]) {
                        let k = String::from_utf8_lossy(k).to_string();
                        let v = String::from_utf8_lossy(v).to_string();
                        map.insert(k, v);
                    }
                    i += 2;
                }
            }
            let inicio_sessao = map.get("inicio_sessao").cloned().unwrap_or_default();
            let status = map.get("status").cloned().unwrap_or_else(|| if user.tipo == "xray" { "On".to_string() } else { "".to_string() });
            let limite = map.get("limite").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            let byid = map.get("byid").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            let dono = map.get("dono").cloned().unwrap_or_default();
            let inicio_sessao_dt = NaiveDateTime::parse_from_str(&inicio_sessao, "%d/%m/%Y %H:%M:%S").unwrap_or(current_time);
            let duration = current_time.signed_duration_since(inicio_sessao_dt);
            let hours = duration.num_hours();
            let minutes = duration.num_minutes() % 60;
            let seconds = duration.num_seconds() % 60;
            let tempo_online = if status == "On" {
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                "00:00:00".to_string()
            };
            if status == "On" {
                users.push(json!({
                    "login": user.login,
                    "tipo": user.tipo,
                    "limite": limite,
                    "conexoes_simultaneas": usuarios_online,
                    "tempo_online": tempo_online,
                    "status": status,
                    "dono": dono,
                    "byid": byid
                }));
            }
            let limite_i = limite as usize;
            if (user.tipo == "ssh" || user.tipo == "openvpn") && usuarios_online > limite_i && limite > 0 {
                let _ = std::process::Command::new("pkill")
                    .arg("-u")
                    .arg(&user.login)
                    .output();
            }
        }
    }
    Ok(json!({
        "status": "success",
        "total": users.len(),
        "users": users
    }))
}
