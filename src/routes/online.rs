use crate::utils::online_utils::{get_users, execute_command};
use sqlx::{Pool, Sqlite};
use std::time::Duration;
use tokio::time::sleep;
use sqlx::Error;
use log::error;
use sqlx::Row;

pub async fn monitor_online_users(pool: Pool<Sqlite>) -> Result<(), Error> {
    loop {
        let start_time = std::time::Instant::now();
        let current_date = chrono::Local::now().naive_local();

        // Primeiro, obtem a lista de usuários do sistema
        if let Ok(users) = get_users() {
            let user_list: Vec<&str> = users.split(',').collect();
            let mut user_count = std::collections::HashMap::new();

            // Conta os usuários ativos
            for user in &user_list {
                if user.trim().is_empty() {
                    continue;
                }
                *user_count.entry(user.to_string()).or_insert(0) += 1;
            }

            // Primeiro marca todos como Off
            sqlx::query("UPDATE online SET status = 'Off'")
                .execute(&pool)
                .await?;

            // Depois atualiza apenas os que estão online
            for user in &user_list {
                if !user.trim().is_empty() {
                    sqlx::query("UPDATE online SET status = 'On' WHERE login = ?")
                        .bind(user.trim())
                        .execute(&pool)
                        .await?;
                }
            }

            // Remove usuários que não estão mais online
            sqlx::query("DELETE FROM online WHERE status = 'Off'")
                .execute(&pool)
                .await?;

            // Atualiza ou insere usuários online
            for (user, count) in &user_count {
                if user.is_empty() {
                    continue;
                }

                match sqlx::query(
                    "SELECT id, login, dias, limite FROM users WHERE login = ?"
                )
                .bind(user)
                .fetch_optional(&pool)
                .await
                {
                    Ok(Some(row)) => {
                        let expiry_date = chrono::Local::now() + chrono::Duration::days(row.get::<i64, _>("dias"));

                        if current_date > expiry_date.naive_local() {
                            execute_command("pkill", &["-u", &user]).unwrap();
                            execute_command("userdel", &[&user]).unwrap();
                            sqlx::query("UPDATE users SET suspenso = 'sim' WHERE login = ?")
                                .bind(user)
                                .execute(&pool)
                                .await?;
                            continue;
                        }

                        match sqlx::query(
                            "SELECT usuarios_online, limite FROM online WHERE login = ?"
                        )
                        .bind(user)
                        .fetch_optional(&pool)
                        .await {
                            Ok(Some(online_row)) => {
                                if online_row.get::<i64, _>("limite") != row.get::<i64, _>("limite") 
                                   || online_row.get::<i64, _>("usuarios_online") != *count {
                                    sqlx::query(
                                        "UPDATE online SET 
                                        limite = ?, 
                                        usuarios_online = ?,
                                        status = 'On'
                                        WHERE login = ?"
                                    )
                                    .bind(row.get::<i64, _>("limite"))
                                    .bind(*count)
                                    .bind(user)
                                    .execute(&pool)
                                    .await?;
                                }

                                if online_row.get::<i64, _>("usuarios_online") > online_row.get::<i64, _>("limite") {
                                    execute_command("pkill", &["-u", &user]).unwrap();
                                }
                            },
                            Ok(None) => {
                                sqlx::query(
                                    "INSERT INTO online (login, limite, usuarios_online, inicio_sessao, status, byid)
                                     VALUES (?, ?, ?, ?, 'On', ?)"
                                )
                                .bind(user)
                                .bind(row.get::<i64, _>("limite"))
                                .bind(*count)
                                .bind(current_date.format("%d/%m/%Y %H:%M:%S").to_string())
                                .bind(row.get::<i64, _>("id"))
                                .execute(&pool)
                                .await?;
                            },
                            Err(e) => {
                                error!("Erro ao executar query SELECT para usuário '{}': {}", user, e);
                            }
                        }
                    },
                    Ok(None) => {
                        error!("Usuário '{}' não encontrado no banco de dados", user);
                    },
                    Err(e) => {
                        error!("Erro ao executar query SELECT para usuário '{}': {}", user, e);
                    }
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