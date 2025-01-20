# Módulo m-dulo-rust

## Requisitos

- Rust (versão mais recente)
- Cargo (gerenciador de pacotes do Rust)
- Postgres
- Docker
- Docker Compose

## Instalação rapida ja com o binario gerado.
    
    sudo wget -qO- https://raw.githubusercontent.com/sshturbo/m-dulo-rust/refs/heads/master/install.sh | sudo bash
    

## Como executar

1. Clone o repositório:
    ```sh
    git clone https://github.com/sshturbo/m-dulo-rust.git
    cd m-dulo-rust
    ```

2. Copie o arquivo `.env.example` para `.env` e configure o token:
    ```sh
    cp .env.example .env
    ```

3. Gere um token e adicione ao arquivo `.env`:
    ```sh
    echo "API_TOKEN=$(openssl rand -hex 16)" >> .env
    ```

4. Suba os containers do Docker Compose: 
   ```sh 
   docker-compose up -d
   ```

4. Compile o projeto:
    ```sh
    cargo build
    ```

5. Execute o projeto:
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
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        socket.send('seu_token_aqui:CRIAR:{"login": "teste2", "senha": "102030", "dias": 30, "limite": 1, "uuid": null}');
    };
    ```

### Deletar

- **Rota:** `EXCLUIR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe um usuario em json e remove um usuario por vez. O uuid do v2ray pode se passo e tambem pode ser passo como null.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        socket.send('seu_token_aqui:EXCLUIR:{"usuario": "teste3", "uuid": null}');
    };
    ```

### Excluir Global

- **Rota:** `EXCLUIR_GLOBAL`
- **Método:** WebSocket
- **Descrição:** Essa rota recebe uma listar de usuarios em json e remove todos os usuarios de uma vez. O uuid do v2ray pode se passo e tambem pode ser passo como null.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        socket.send('seu_token_aqui:EXCLUIR_GLOBAL:{"usuarios":[{"usuario":"teste2","uuid": null},{"usuario":"teste1","uuid": null}]}');
    };
    ```

### Sincronização

- **Rota:** `SINCRONIZAR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe uma lista de usuarios em json e sincroniza todos os usuarios de uma vez se o usaurio ja existir ele e excluido e adicionado novamente. O uuid do v2ray pode se passo e tambem pode ser passo como null.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        socket.send('seu_token_aqui:SINCRONIZAR:[{"login":"user1","senha":"password1","dias":30,"limite":5,"uuid":"uuid1"},{"login":"user2","senha":"password2","dias":30,"limite":5,"uuid": null}]');
    };
    ```

### Editar

- **Rota:** `EDITAR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe um usuário em json e edita as informações do usuário existente. O uuid do v2ray pode se passo e tambem pode ser passo como null.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        socket.send('seu_token_aqui:EDITAR:{"login_antigo": "teste2", "login_novo": "teste3", "senha": "nova_senha", "dias": 30, "limite": 1, "uuid": null}');
    };
    ```

### Online

- **Rota:** `ONLINE`
- **Método:** WebSocket
- **Descrição:** Esta rota se conecta ao WebSocket `ws://127.0.0.1:9001/online` e começa a enviar uma lista de usuários online em JSON com o login, limite e o tempo que está online. Se não houver nenhum usuário online, retorna a mensagem em JSON `{"message":"Nenhum usuário online no momento."}`.
- **Exemplo de uso:**
    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/online');
    socket.onopen = () => {
        console.log('Conectado ao servidor para verificar usuários online.');
    };
    socket.onmessage = (event) => {
        console.log('Usuários online:', event.data);
    };
    ```