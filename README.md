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
- **Descrição:** Esta rota recebe um usuario em json e cria um novo usuario. O uuid do v2ray pode se passo e tambem pode ser passo como null.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ CRIAR:{ "login": "teste2", "senha": "102030", "dias": 30, "limite": 1, "uuid": null } }));
    };
    ```

### Deletar

- **Rota:** `EXCLUIR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe um usuario em json e remove um usuario por vez. O uuid do v2ray pode se passo e tambem pode ser passo como null.
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
- **Descrição:** Essa rota recebe uma listar de usuarios em json e remove todos os usuarios de uma vez. O uuid do v2ray pode se passo e tambem pode ser passo como null.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ EXCLUIR_GLOBAL:{"usuarios":[{"usuario":"teste2","uuid": null},{"usuario":"teste1","uuid": null}]} }));
    };
    ```


### Sincronização

- **Rota:** `SINCRONIZAR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe uma lista de usuarios em json e sincroniza todos os usuarios de uma vez se o usaurio ja existir ele e excluido e adicionado novamente. O uuid do v2ray pode se passo e tambem pode ser passo como null.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/ws');
    socket.onopen = () => {
        socket.send(JSON.stringify({ SINCRONIZAR:[{"login":"user1","senha":"password1","dias":30,"limite":5,"uuid":"uuid1"},{"login":"user2","senha":"password2","dias":30,"limite":5,"uuid": null}] }));
    };
    ```