# Módulo Rust

## Requisitos

- Rust (versão mais recente)
- Cargo (gerenciador de pacotes do Rust)

## Como executar

1. Clone o repositório:
    ```sh
    git clone <URL-do-repositório>
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

- **Rota:** `/create`
- **Método:** WebSocket
- **Descrição:** Cria um novo item.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ CRIAR: { login: 'teste3', senha: '102030', dias: 30, limite: 1 } }));
    };
    ```

### Deletar

- **Rota:** `/delete`
- **Método:** WebSocket
- **Descrição:** Deleta um item.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ EXCLUIR: { usuario: 'teste3', uuid: null } }));
    };
    ```

### Excluir Global

- **Rota:** `/remove_all`
- **Método:** WebSocket
- **Descrição:** Remove todos os itens.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ EXCLUIR_GLOBAL: { usuarios: [{ usuario: 'teste3', uuid: null }] } }));
    };
    ```
