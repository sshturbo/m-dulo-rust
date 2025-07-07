# Documentação da API do Painel Web Pro

## Índice

- [Autenticação](#autenticação)
- [Endpoints de Revenda](#endpoints-de-revenda)
  - [Criar Revenda](#criar-revenda)
  - [Renovar Revenda](#renovar-revenda)
  - [Excluir Revenda](#excluir-revenda)
- [Endpoints de Usuário](#endpoints-de-usuário)
  - [Criar Usuário](#criar-usuário)
  - [Excluir Usuário](#excluir-usuário)
- [Endpoints Online](#endpoints-online)
  - [Listar Usuários Online](#listar-usuários-online)
- [Respostas de Erro](#respostas-de-erro)

## Autenticação

Todas as requisições precisam incluir um token Bearer no header Authorization:

```
Authorization: Bearer {seu-token}
```

O token pode ser obtido nas configurações da API do painel admin.

## Endpoints de Revenda

### Criar Revenda

Cria uma nova revenda com atribuição de recursos.

**Endpoint:** POST `/api/revenda/criar.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string", // Login da revenda (obrigatório)
  "senha": "string", // Senha da revenda (obrigatório)
  "nome": "string", // Nome da revenda (obrigatório)
  "contato": "string", // Contato da revenda (opcional)
  "email": "string", // Email da revenda (opcional)
  "limite": number, // Limite de usuários (obrigatório)
  "limitetest": number, // Limite de testes (obrigatório)
  "dias": number // Duração em dias (obrigatório)
}
```

**Validações:**

- Login não pode já existir
- Usuário precisa ter nível de acesso >= 2
- Para tipo "Credito": usuário precisa ter créditos disponíveis
- Para tipo "Validade": usuário não pode estar vencido ou suspenso

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Revenda e atribuição criadas com sucesso",
  "data": {
    "revenda": {
      "id": number,
      "login": "string",
      "nome": "string"
    },
    "atribuicao": {
      "tipo": "string",
      "limite": number,
      "limitetest": number,
      "expira": "string"
    }
  }
}
```

### Renovar Revenda

Renova a atribuição de uma revenda existente.

**Endpoint:** POST `/api/revenda/renovar.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string", // Login da revenda (obrigatório)
  "dias": number, // Duração em dias (obrigatório)
  "limite": number, // Novo limite de usuários (obrigatório)
  "limitetest": number // Novo limite de testes (obrigatório)
}
```

**Validações:**

- Revenda precisa existir
- Tipo "Credito": usuário precisa ter créditos disponíveis
- Tipo "Validade": não pode reduzir limite abaixo dos usuários já criados
- Revenda não pode estar suspensa ou vencida

**Resposta de Sucesso:**

```json
{
  "message": "Atribuição renovada com sucesso",
  "data": {
    "login": "string",
    "tipo": "string",
    "dias": number,
    "limite": number,
    "limitetest": number
  }
}
```

### Excluir Revenda

Exclui uma revenda e todos seus usuários associados.

**Endpoint:** POST `/api/revenda/excluir.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string" // Login da revenda (obrigatório)
}
```

**Validações:**

- Revenda precisa existir
- Usuário precisa ter permissão para excluir a revenda

**Resposta de Sucesso:**

```json
{
  "message": "Revenda, atribuição e usuários associados excluídos com sucesso",
  "data": {
    "revendas_excluidas": number,
    "arquivo": "string"
  }
}
```

## Endpoints de Usuário

## Endpoints Online

### Listar Usuários Online

Retorna todos os usuários online atualmente cadastrados na tabela `api_online`, com informações do tempo online calculado.

**Endpoint:** GET `/api/online/listall.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros:**

Nenhum parâmetro necessário.

**Exemplo de resposta:**

```json
[
  {
    "login": "usuario1",
    "limite": 10,
    "tipo": "premium",
    "ip": "192.168.0.1",
    "start_time": "2025-05-17 20:00:00",
    "tempo_online": "03:15:42",
    "dono": "admin"
  },
  {
    "login": "usuario2",
    "limite": 5,
    "tipo": "normal",
    "ip": "192.168.0.2",
    "start_time": "2025-05-17 21:30:00",
    "tempo_online": "01:45:10",
    "dono": "revenda01"
  }
]
```

**Observações:**
- O campo `tempo_online` é calculado em tempo real pela diferença entre o campo `start_time` e a data/hora atual de São Paulo.
- O campo `dono` corresponde ao login do proprietário da conta.

### Criar Usuário

Cria um novo usuário no sistema.

**Endpoint:** POST `/api/usuario/criar.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string", // Login do usuário (obrigatório)
  "senha": "string", // Senha do usuário (obrigatório)
  "dias": number, // Duração em dias (obrigatório)
  "limite": number, // Limite de conexões (obrigatório)
  "tipo": "string", // Tipo do usuário: v2ray, xray ou ssh (opcional, default: v2ray)
  "uuid": "string", // UUID para usuários v2ray/xray (opcional)
  "nome": "string", // Nome do usuário (obrigatório)
  "contato": "string", // Contato do usuário (opcional)
}
```

**Validações:**

- Login não pode já existir
- Usuário precisa ter atribuição válida na categoria
- Para tipo "Credito": usuário precisa ter créditos disponíveis
- Para tipo "Validade": não pode exceder limite da categoria
- Revenda não pode estar suspensa ou vencida

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário criado com sucesso",
  "userData": {
    "login": "string",
    "senha": "string",
    "expira": "string",
    "limite": number,
    "tipo": "string"
  }
}
```

### Criar Teste

Cria um usuário de teste com limite 1 e tempo de expiração em minutos.

**Endpoint:** POST `/api/usuario/criar_teste.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string", // Login do teste (obrigatório)
  "senha": "string", // Senha do teste (obrigatório)
  "minutos": number, // Duração em minutos (obrigatório)
  "tipo": "string", // Tipo do usuário: v2ray, xray, ssh (opcional, default: ssh)
  "nome": "string", // Nome do usuário (opcional)
  "contato": "string", // Contato do usuário (opcional)
  "uuid": "string" // UUID (opcional)
}
```

**Comportamento:**

- O campo `limite` é sempre 1
- O campo `dias` enviado para o servidor é sempre 1
- O campo de expiração salvo no banco é calculado em minutos

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Teste criado com sucesso",
  "data": {
    "login": "string",
    "senha": "string",
    "limite": 1,
    "expira": "2025-05-17 18:00:00",
    "categoria": "string"
  }
}
```

### Renovar Usuário

Renova a validade de um usuário existente por 31 dias, utilizando automaticamente o limite já cadastrados no banco de dados.

**Endpoint:** POST `/api/usuario/renovar.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string" // Login do usuário a ser renovado (obrigatório)
}
```

**Funcionamento:**
- O sistema irá renovar o usuário por 31 dias a partir da data atual.
- O limite serão mantidos conforme já cadastrados para o usuário no banco de dados.
- Não é necessário informar dias, limite na requisição.

**Validações:**
- Usuário precisa existir
- Usuário precisa ter permissão para renovar o usuário
- Para tipo "Credito": usuário precisa ter créditos disponíveis

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário renovado com sucesso.",
  "data": {
    "login": "string",
    "nova_expira": "2025-06-17 19:42:00",
    "limite": 10,
  }
}
```

### Excluir Usuário

Exclui um usuário do sistema e de todos os servidores.

**Endpoint:** POST `/api/usuario/excluir.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string" // Login do usuário a ser excluído (obrigatório)
}
```

**Validações:**

- Usuário precisa existir
- Servidores precisam estar disponíveis para exclusão

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário excluído com sucesso",
  "data": {
    "login": "string"
  }
}
```

## Respostas de Erro

Em caso de erro, a API retorna um status code apropriado junto com uma mensagem descritiva:

```json
{
  "error": "Descrição do erro"
}
```

**Códigos de Erro Comuns:**

- 400: Parâmetros inválidos ou faltando
- 401: Token inválido/expirado
- 403: Sem permissão para a ação
- 404: Revenda não encontrada
- 500: Erro interno do servidor
