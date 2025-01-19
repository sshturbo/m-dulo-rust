use crate::utils::online_utils::{get_users, execute_command};
use sqlx::{Pool, Postgres};
use std::time::Duration;
use tokio::time::sleep;
use sqlx::Error;
use log::{error, info};

pub async fn monitor_online_users(pool: Pool<Postgres>) -> Result<(), Error> {
    loop {
        let start_time = std::time::Instant::now();

        // Obtém a lista de usuários online da tabela online
        let online_users: Vec<String> = sqlx::query!("SELECT login FROM online")
            .fetch_all(&pool)
            .await?
            .into_iter()
            .map(|row| row.login)
            .collect();

        if let Ok(users) = get_users() {
            info!("Lista de usuários obtida: {}", users);
            let user_list: Vec<&str> = users.split(',').collect();
            let mut user_count = std::collections::HashMap::new();

            for user in &user_list {
                if user.trim().is_empty() {
                    continue; 
                }
                *user_count.entry(user.to_string()).or_insert(0) += 1;
            }

            info!("Mapa de contagem de usuários: {:?}", user_count);

            // Verifica e remove usuários que estão na tabela online, mas não na lista
            for online_user in &online_users {
                if !user_list.contains(&online_user.as_str()) {
                    info!("Usuário '{}' não está na lista de usuários. Removendo da tabela online.", online_user);
                    sqlx::query!("DELETE FROM online WHERE login = $1", online_user)
                        .execute(&pool)
                        .await?;
                }
            }

            for (user, count) in &user_count {
                if user.is_empty() {
                    continue; 
                }
                info!("Processando usuário: {}", user);
                match sqlx::query!(
                    "SELECT id, login, dias, limite FROM users WHERE login = $1",
                    user
                )
                .fetch_optional(&pool)
                .await
                {
                    Ok(Some(row)) => {
                        info!("Usuário '{}' encontrado no banco de dados.", user);
                        let expiry_date = chrono::Local::now() + chrono::Duration::days(row.dias as i64);
                        let current_date = chrono::Local::now().naive_local();

                        if current_date > expiry_date.naive_local() {
                            info!("Usuário '{}' expirado. Executando comandos de suspensão.", user);
                            execute_command("pkill", &["-u", &user]).unwrap();
                            execute_command("userdel", &[&user]).unwrap();
                            sqlx::query!(
                                "UPDATE users SET suspenso = 'sim' WHERE login = $1",
                                user
                            )
                            .execute(&pool)
                            .await?;
                        } 
                        else {
                            // Verificando se o usuário já existe na tabela online
                            match sqlx::query!(
                                "SELECT usuarios_online, limite FROM online WHERE login = $1",
                                user
                            )
                            .fetch_optional(&pool)
                            .await {
                                Ok(Some(online_row)) => {
                                    // Verificando se o limite na tabela online está desatualizado
                                    if online_row.limite != row.limite {
                                        info!("Atualizando limite para o usuário '{}' de {} para {}", user, online_row.limite, row.limite);
                                        // Atualiza o limite na tabela online para corresponder ao limite da tabela users
                                        sqlx::query!(
                                            "UPDATE online SET limite = $1 WHERE login = $2",
                                            row.limite,
                                            user
                                        )
                                        .execute(&pool)
                                        .await?;
                                    }

                                    // Verificando se a quantidade de usuários online é diferente de count
                                    if let Some(online_users) = online_row.usuarios_online {
                                        if online_users != *count {
                                            info!("Atualizando a quantidade de usuários online de '{}' para {}", user, count);
                                            sqlx::query!(
                                                "UPDATE online SET usuarios_online = $1 WHERE login = $2",
                                                *count,
                                                user
                                            )
                                            .execute(&pool)
                                            .await?;
                                        }

                                        // Verificando se o limite foi excedido e executando pkill se necessário
                                        if online_users > online_row.limite {
                                            info!("Usuário '{}' excedeu o limite de online. Executando pkill.", user);
                                            execute_command("pkill", &["-u", &user]).unwrap();
                                        }
                                    }
                                },
                                Ok(None) => {
                                    // Caso o usuário ainda não tenha entrada na tabela online
                                    info!("Usuário '{}' ainda não está online. Adicionando à tabela online.", user);
                                    sqlx::query!(
                                        "INSERT INTO online (login, limite, usuarios_online, inicio_sessao, status, byid)
                                         VALUES ($1, $2, $3, $4, 'On', $5)",
                                        user,
                                        row.limite,
                                        *count,
                                        current_date.format("%d/%m/%Y %H:%M:%S").to_string(),
                                        row.id
                                    )
                                    .execute(&pool)
                                    .await?;
                                },
                                Err(e) => {
                                    error!("Erro ao executar query SELECT para usuário '{}': {}", user, e);
                                }
                            }
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
        } else {
            error!("Falha ao obter usuários.");
        }

        let elapsed_time = start_time.elapsed();
        info!("Tempo de processamento: {:?}", elapsed_time);

        let sleep_duration = if elapsed_time < Duration::from_secs(1) {
            Duration::from_secs(1) - elapsed_time
        } else {
            Duration::from_secs(0)
        };

        sleep(sleep_duration).await;
    }
}
