use rand::{distributions::Alphanumeric, Rng};

pub fn gerar_email_aleatorio(tamanho: usize) -> String {
    let local: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(tamanho)
        .map(char::from)
        .collect();
    format!("{}@gmail.com", local)
}
