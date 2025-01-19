use crate::utils::online_utils::{get_users, execute_command};
use sqlx::{Pool, Postgres};
use chrono::TimeZone; 
use std::time::Duration;
use tokio::time::sleep;
use sqlx::Error;
use log::error;

pub async fn monitor_online_users(pool: Pool<Postgres>) -> Result<(), Error> {
    loop {
        if let Ok(users) = get_users() {
            let user_list: Vec<&str> = users.split(',').collect();
            let mut user_count = std::collections::HashMap::new();

            for user in &user_list {
                *user_count.entry(user).or_insert(0) += 1;
            }

            for (user, count) in &user_count {
                if user.is_empty() {
                    continue; 
                }
                let user = user.to_string();
                match sqlx::query!(
                    "SELECT id, login, dias, limite FROM users WHERE login = $1",
                    user
                )
                .fetch_optional(&pool)
                .await
                {
                    Ok(Some(row)) => {
                        let expiry_date = chrono::Local.timestamp_opt(row.dias as i64, 0).single().unwrap().naive_local();
                        let current_date = chrono::Local::now().naive_local();

                        if current_date > expiry_date {
                            execute_command("pkill", &["-u", &user]).unwrap();
                            execute_command("userdel", &[&user]).unwrap();
                            sqlx::query!(
                                "UPDATE users SET suspenso = 'sim' WHERE login = $1",
                                user
                            )
                            .execute(&pool)
                            .await?;
                        } else if *count > row.limite {
                            execute_command("pkill", &["-u", &user]).unwrap();
                        } else {
                            sqlx::query!(
                                "INSERT INTO online (login, limite, inicio_sessao, status, byid)
                                 VALUES ($1, $2, $3, 'On', $4)
                                 ON CONFLICT (login) DO UPDATE
                                 SET limite = $2, byid = $4",
                                user,
                                *count,
                                current_date.format("%d/%m/%Y %H:%M:%S").to_string(),
                                row.id
                            )
                            .execute(&pool)
                            .await?;
                        }
                    },
                    Ok(None) => {
                        error!("Usuário '{}' não encontrado no banco de dados.", user);
                    },
                    Err(e) => {
                        error!("Erro ao executar query SELECT para usuário '{}': {}", user, e);
                    }
                }
            }

            match sqlx::query!(
                "DELETE FROM online WHERE login NOT IN ($1)",
                &user_list.join(",")
            )
            .execute(&pool)
            .await
            {
                Ok(_) => (),
                Err(e) => error!("Erro ao executar query DELETE: {}", e),
            }
        } else {
            error!("Falha ao obter usuários.");
        }

        sleep(Duration::from_secs(1)).await;
    }
}
