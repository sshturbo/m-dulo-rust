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
    status: String
}

pub async fn monitor_users(pool: Pool<Sqlite>) -> Result<serde_json::Value, Error> {
    let rows = sqlx::query_as::<_, OnlineUser>(
        "SELECT login, limite, inicio_sessao, usuarios_online, status FROM online WHERE status = 'On'"
    )
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        return Ok(json!({"message": "Nenhum usuário online no momento"}));
    }

    let mut users = Vec::new();

    for row in rows {
        let inicio_sessao = match NaiveDateTime::parse_from_str(&row.inicio_sessao, "%d/%m/%Y %H:%M:%S") {
            Ok(dt) => dt,
            Err(e) => {
                error!("Erro ao parsear inicio_sessao para usuário '{}': {}", row.login, e);
                continue;
            }
        };
        let current_time = Local::now().naive_local();
        let duration = current_time.signed_duration_since(inicio_sessao);

        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;

        let tempo_online = format!(
            "Usuário online há {} horas {} minutos e {} segundos",
            hours, minutes, seconds
        );

        users.push(json!({
            "login": row.login,
            "limite": row.limite,
            "online": row.usuarios_online,
            "tempo_online": tempo_online,
            "status": row.status
        }));
    }

    Ok(json!(users))
}
