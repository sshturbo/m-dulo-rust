use sqlx::{Pool, Postgres, Error};
use chrono::{NaiveDateTime, Local};
use serde_json::json;
use log::error;

pub async fn monitor_users(pool: Pool<Postgres>) -> Result<String, Error> {
    let rows = match sqlx::query!(
        "SELECT login, limite, inicio_sessao FROM online"
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            error!("Erro ao executar query SELECT em monitor_users: {}", e);
            return Err(e);
        }
    };

    if rows.is_empty() {
        return Ok(serde_json::json!({"message": "Nenhum usuário online no momento."}).to_string());
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
            "tempo_online": tempo_online
        }));
    }

    match serde_json::to_string(&users) {
        Ok(json_str) => Ok(json_str),
        Err(e) => {
            error!("Erro ao converter usuários para JSON: {}", e);
            Err(sqlx::Error::RowNotFound) 
        }
    }
}
