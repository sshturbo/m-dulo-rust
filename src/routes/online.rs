use sqlx::{Pool, Sqlite, SqlitePool};
use std::time::Duration;
use tokio::time::interval;
use chrono::{Local, NaiveDateTime, Duration as ChronoDuration};
use crate::utils::online_utils::get_users;
use crate::utils::restart_v2ray::reiniciar_v2ray;
use std::process::Command;
use std::fs;
use serde_json::Value;
use thiserror::Error;
use serde::Serialize;
use log::info; // Adicione esta linha

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Erro ao obter usuários: {0}")]
    GetUsersError(#[from] std::io::Error),
    #[error("Erro ao executar comando: {0}")]
    CommandError(String),
    #[error("Erro ao acessar banco de dados: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Erro ao manipular arquivo: {0}")]
    FileError(#[source] std::io::Error),
    #[error("Erro ao manipular JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

// Tornando o tipo `OnlineUser` público para ser usado em outros módulos
#[derive(Serialize)]
pub struct OnlineUser {
    pub login: String,
    pub limite: String,
    pub tempo_online: String,
}

async fn get_online_inicio(pool: &SqlitePool, user: &str) -> Result<Option<NaiveDateTime>, MonitorError> {
    let online_inicio: Option<NaiveDateTime> = sqlx::query_scalar!(
        "SELECT online_inicio FROM online WHERE login = ?",
        user
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| MonitorError::DatabaseError(sqlx::Error::RowNotFound))?
    .flatten();

    Ok(online_inicio)
}

pub async fn monitor_users(pool: Pool<Sqlite>) -> Result<Vec<OnlineUser>, MonitorError> {
    info!("Iniciando monitoramento de usuários"); // Adicione esta linha
    let mut interval = interval(Duration::from_secs(1));
    let mut online_users = Vec::new();

    interval.tick().await;

    match get_users() {
        Ok(users) => {
            if users.is_empty() {
                info!("Nenhum usuário online no momento.");
            } else {
                let user_list: Vec<&str> = users.split(',').collect();
                let mut user_count = std::collections::HashMap::new();

                for user in &user_list {
                    *user_count.entry(user).or_insert(0) += 1;
                }

                online_users.clear();

                for (user, count) in user_count {
                    let user_info = sqlx::query!(
                        "SELECT limite, dias, uuid FROM users WHERE login = ?",
                        user
                    )
                    .fetch_optional(&pool)
                    .await.map_err(MonitorError::DatabaseError)?;

                    if let Some(user_info) = user_info {
                        let limite = user_info.limite;
                        let dias = user_info.dias;
                        let uuid = user_info.uuid.as_deref();

                        // Verifica validade do usuário
                        let expiration_date = Local::now().naive_local() + ChronoDuration::days(dias as i64);
                        if Local::now().naive_local() > expiration_date {
                            execute_command("pkill", &["-u", user])?;
                            execute_command("userdel", &[user])?;

                            sqlx::query!(
                                "UPDATE users SET suspenso = 'sim' WHERE login = ?",
                                user
                            )
                            .execute(&pool)
                            .await.map_err(MonitorError::DatabaseError)?;

                            if let Some(uuid) = uuid {
                                remover_uuid_v2ray(uuid).await?;
                            }
                        } else if count > limite {
                            execute_command("pkill", &["-u", user])?;
                        } else {
                            let now = Local::now().naive_local().format("%Y-%m-%d %H:%M:%S").to_string();
                            let limite_count = format!("{}/{}", limite, count);
                            sqlx::query!(
                                "INSERT INTO online (login, limite, online_inicio, online_fim, online) VALUES (?, ?, ?, NULL, 'on')
                                ON CONFLICT(login) DO UPDATE SET limite = ?, online_inicio = ?, online = 'on'",
                                user,
                                limite_count,
                                now,
                                limite_count,
                                now
                            )
                            .execute(&pool)
                            .await.map_err(MonitorError::DatabaseError)?;

                            if let Some(online_inicio) = get_online_inicio(&pool, user).await? {
                                let duration = Local::now().naive_local().signed_duration_since(online_inicio);
                                let hours = duration.num_hours();
                                let minutes = duration.num_minutes() % 60;
                                let seconds = duration.num_seconds() % 60;
                                let tempo_online = format!("usuário online há {} horas {} minutos e {} segundos", hours, minutes, seconds);

                                online_users.push(OnlineUser {
                                    login: user.to_string(),
                                    limite: limite_count,
                                    tempo_online,
                                });
                            }
                        }
                    }
                }

                // Marcar usuários offline
                let now = Local::now().naive_local().format("%Y-%m-%d %H:%M:%S").to_string();
                let user_list_string = user_list.join(",");
                sqlx::query!(
                    "UPDATE online SET online = 'off', online_fim = ? WHERE online = 'on' AND login NOT IN (?)",
                    now,
                    user_list_string
                )
                .execute(&pool)
                .await.map_err(MonitorError::DatabaseError)?;

                reiniciar_v2ray().await;
            }
        }
        Err(e) => eprintln!("Erro ao obter usuários: {}", e),
    }

    Ok(online_users)
}

async fn remover_uuid_v2ray(uuid: &str) -> Result<(), MonitorError> {
    let config_path = "/etc/v2ray/config.json";

    if !std::path::Path::new(config_path).exists() {
        return Ok(());
    }

    let content = fs::read_to_string(config_path).map_err(MonitorError::FileError)?;
    let mut json: Value = serde_json::from_str(&content).map_err(MonitorError::JsonError)?;

    if let Some(inbounds) = json.get_mut("inbounds") {
        if let Some(first_inbound) = inbounds.as_array_mut().unwrap().get_mut(0) {
            if let Some(settings) = first_inbound.get_mut("settings") {
                if let Some(clients) = settings.get_mut("clients") {
                    if let Some(clients_array) = clients.as_array_mut() {
                        clients_array.retain(|client| client["id"].as_str().unwrap_or("") != uuid);

                        let new_content = serde_json::to_string_pretty(&json).map_err(MonitorError::JsonError)?;
                        fs::write(config_path, new_content).map_err(MonitorError::FileError)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn execute_command(command: &str, args: &[&str]) -> Result<(), MonitorError> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|e| MonitorError::CommandError(e.to_string()))?;

    if !status.success() {
        return Err(MonitorError::CommandError(format!("Comando {} falhou", command)));
    }

    Ok(())
}
