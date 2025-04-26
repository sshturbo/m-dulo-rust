use sqlx::{Pool, Sqlite, Error};
use chrono::{NaiveDateTime, Local};
use serde_json::json;
use log::error;

#[derive(sqlx::FromRow)]
struct OnlineUser {
    login: String,
    limite: i64,
    inicio_sessao: String,
    usuarios_online: i64,
    status: String,
    dono: String
}

pub async fn monitor_users(pool: Pool<Sqlite>) -> Result<serde_json::Value, Error> {
    let rows = sqlx::query_as::<_, OnlineUser>(
        "SELECT o.login, o.limite, o.inicio_sessao, o.usuarios_online, o.status, u.dono 
         FROM online o
         JOIN users u ON o.login = u.login
         WHERE o.status = 'On' AND o.usuarios_online > 0
         ORDER BY o.login ASC"
    )
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        return Ok(json!({
            "status": "success",
            "message": "Nenhum usuário online no momento",
            "users": []
        }));
    }

    let mut users = Vec::new();
    let current_time = Local::now().naive_local();

    for row in rows {
        let inicio_sessao = match NaiveDateTime::parse_from_str(&row.inicio_sessao, "%d/%m/%Y %H:%M:%S") {
            Ok(dt) => dt,
            Err(e) => {
                error!("Erro ao parsear inicio_sessao para usuário '{}': {}", row.login, e);
                continue;
            }
        };
        
        let duration = current_time.signed_duration_since(inicio_sessao);
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;

        users.push(json!({
            "login": row.login,
            "limite": row.limite,
            "online": row.usuarios_online,
            "tempo_online": format!(
                "{:02}:{:02}:{:02}",
                hours,
                minutes,
                seconds
            ),
            "status": row.status,
            "dono": row.dono
        }));
    }

    Ok(json!({
        "status": "success",
        "total": users.len(),
        "users": users
    }))
}
