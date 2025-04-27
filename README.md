# Módulo m-dulo-rust

## Requisitos

- Rust (versão mais recente)
- Cargo (gerenciador de pacotes do Rust)
- Sqlite

## Instalação rápida com o binário gerado

    ```
    wget -qO- https://raw.githubusercontent.com/sshturbo/m-dulo-rust/refs/heads/master/install.sh | sudo bash -s d6dbaa87ceda172a41971ad3796056d4
    ```
    
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

4. Compile o projeto:

    ```sh
    cargo build
    ```

5. Execute o projeto:

    ```
    cargo run
    ```

## Rotas via WebSocket

### Criar

- **Rota:** `CRIAR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe um usuário em json e cria um novo usuário. O uuid do v2ray pode ser passado ou ser null.
- **Exemplo de uso:**

    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        socket.send('seu_token_aqui:CRIAR:{
            "login": "teste2", 
            "senha": "102030", 
            "dias": 30, 
            "limite": 1, 
            "uuid": null, 
            "tipo": "xray",
            "dono": "revendedor1",
            "byid": 1
        }');
    };
    ```

### Deletar

- **Rota:** `EXCLUIR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe um usuário em json e remove um usuário por vez. O uuid do v2ray pode ser passado ou ser null.
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
- **Descrição:** Essa rota recebe uma lista de usuários em json e remove todos os usuários de uma vez. O uuid do v2ray pode ser passado ou ser null.
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
- **Descrição:** Esta rota recebe uma lista de usuários em json e sincroniza todos os usuários de uma vez. Se o usuário já existir, ele é excluído e adicionado novamente. O uuid do v2ray pode ser passado ou ser null.
- **Exemplo de uso:**

    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        socket.send('seu_token_aqui:SINCRONIZAR:[{
            "login": "user1",
            "senha": "password1",
            "dias": 30,
            "limite": 5,
            "uuid": "uuid1",
            "tipo": "v2ray",
            "dono": "revendedor1",
            "byid": 1
        },
        {
            "login": "user2",
            "senha": "password2",
            "dias": 30,
            "limite": 5,
            "uuid": null,
            "tipo": "xray",
            "dono": "revendedor2",
            "byid": 2
        }]');
    };
    ```

### Editar

- **Rota:** `EDITAR`
- **Método:** WebSocket
- **Descrição:** Esta rota recebe um usuário em json e edita as informações do usuário existente. O uuid do v2ray pode ser passado ou ser null. Os campos `dono` e `byid` são mantidos automaticamente do usuário original, não sendo necessário enviá-los na requisição.
- **Exemplo de uso:**

    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001');
    socket.onopen = () => {
        // Observe que dono e byid não precisam ser enviados na edição
        socket.send('seu_token_aqui:EDITAR:{
            "login_antigo": "teste2",
            "login_novo": "teste3",
            "senha": "nova_senha",
            "dias": 30,
            "limite": 1,
            "uuid": null,
            "tipo": "v2ray"
        }');
    };
    ```

### Online

- **Rota:** `ONLINE`
- **Método:** WebSocket
- **Descrição:** Esta rota se conecta ao WebSocket `ws://127.0.0.1:9001/online` e começa a enviar uma lista de usuários online em JSON com o login, limite, conexões simultâneas, tempo online, status e informações adicionais. Se não houver nenhum usuário online, retorna a mensagem em JSON `{"message":"Nenhum usuário online no momento."}`.

- **Exemplo de uso:**

    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/online');
    socket.onopen = () => {
        socket.send('seu_token_aqui'); // Envia o token de autenticação
    };
    socket.onmessage = (event) => {
        console.log('Usuários online:', event.data);
    };
    ```

- **Exemplo de resposta:**

    ```json
    {
        "status": "success",
        "total": 2,
        "users": [
            {
                "login": "user1",
                "limite": 2,
                "conexoes_simultaneas": 1,
                "tempo_online": "00:15:30",
                "status": "On",
                "dono": "revendedor1",
                "byid": 1
            }
        ]
    }
    ```

### Domain (Cloudflare)

- **Rota:** `/domain`
- **Método:** WebSocket
- **Descrição:** Esta rota retorna o subdomínio Cloudflare gerado pelo túnel. É necessário autenticação por token.
- **Como usar:**

    1. Conecte-se ao WebSocket em `ws://127.0.0.1:9001/domain`.
    2. Envie o token de autenticação como primeira mensagem (igual às rotas `/online` e `/sync-status`).
    3. Se o token for válido, o subdomínio será enviado como texto. Caso contrário, será enviada uma mensagem de erro e a conexão será encerrada.

- **Exemplo de uso:**

    ```javascript
    const socket = new WebSocket('ws://127.0.0.1:9001/domain');
    socket.onopen = () => {
        socket.send('seu_token_aqui'); // Envie o token como primeira mensagem
    };
    socket.onmessage = (event) => {
        console.log('Resposta:', event.data);
    };
    ```

- **Possíveis respostas:**
    - Subdomínio Cloudflare (ex: `https://algumacoisa.trycloudflare.com`)
    - `{"status":"error","message":"Token inválido"}`
    - `{"status":"error","message":"Token não fornecido"}`
    - `Subdomínio não encontrado`

## Observações sobre os campos

- O campo `"tipo"` (com valor `"v2ray"` ou `"xray"`) é **obrigatório** nas rotas de criação, edição e sincronização de usuários via WebSocket:
  - CRIAR
  - EDITAR
  - SINCRONIZAR

- O campo `"dono"` indica qual revendedor criou a conta
- O campo `"byid"` é um identificador único do usuário no banco de dados
- O campo `"conexoes_simultaneas"` indica quantas conexões ativas o usuário tem no momento
- **Não é necessário enviar os campos `tipo`, `dono` e `byid` nas rotas de exclusão** (`EXCLUIR`, `EXCLUIR_GLOBAL`), pois o backend busca essas informações diretamente no banco de dados.

## Build para aarch64 usando cross (recomendado para ARM 64 bits)

Para compilar o projeto para ARM 64 bits (aarch64) de forma fácil e portátil, use a ferramenta [cross](https://github.com/cross-rs/cross):

1. Instale o cross (apenas uma vez):

   ```bash
   cargo install cross
   ```

2. Compile para aarch64 (binário estático, compatível com a maioria dos Linux ARM 64 bits):

   ```bash
   cross build --release --target aarch64-unknown-linux-musl
   ```

3. O binário gerado estará em:

   ```
   target/aarch64-unknown-linux-musl/release/
   ```

> **Dica:** Você também pode compilar para outros targets suportados pelo Rust, basta trocar o parâmetro `--target`.

> **Importante:** Para usar o `cross`, é necessário ter o [Docker](https://www.docker.com/) instalado e rodando na sua máquina.

## Build para amd64 (x86_64) usando cross

Se quiser compilar para máquinas x86_64 (amd64), use:

```bash
cross build --release --target x86_64-unknown-linux-musl
```

O binário gerado estará em:

```
target/x86_64-unknown-linux-musl/release/
```
