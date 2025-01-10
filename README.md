# Módulo m-dulo-rust

## Requisitos

- Rust (versão mais recente)
- Cargo (gerenciador de pacotes do Rust)

## Como executar

1. Clone o repositório:
    ```sh
    git clone https://github.com/sshturbo/m-dulo-rust.git
    cd m-dulo-rust
    ```

2. Compile o projeto:
    ```sh
    cargo build
    ```

3. Execute o projeto:
    ```sh
    cargo run
    ```

## Rotas via WebSocket

### Criar

- **Rota:** `CRIAR`
- **Método:** WebSocket
- **Descrição:** Cria um novo usuario.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ CRIAR: { login: 'teste3', senha: '102030', dias: 30, limite: 1 } }));
    };
    ```

### Deletar

- **Rota:** `EXCLUIR`
- **Método:** WebSocket
- **Descrição:** Remove um usuario por vez.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ EXCLUIR: { usuario: 'teste3', uuid: null } }));
    };
    ```

### Excluir Global

- **Rota:** `EXCLUIR_GLOBAL`
- **Método:** WebSocket
- **Descrição:** Remove todos os usuarios de uma vez.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ EXCLUIR_GLOBAL:{"usuarios":[{"usuario":"teste2","uuid": null},{"usuario":"teste1","uuid": null}]} }));
    };
    ```
