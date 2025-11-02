# Documentação da API do Painel Web Pro

## Índice

- [Autenticação](#autenticação)
- [Endpoints de Revenda](#endpoints-de-revenda)
  - [Criar Revenda](#criar-revenda)
  - [Renovar Revenda](#renovar-revenda)
  - [Ativar Revenda](#ativar-revenda)
  - [Suspender Revenda](#suspender-revenda)
  - [Excluir Revenda](#excluir-revenda)
  - [Listar Revendas](#listar-revendas)
  - [Listar Revendas Global](#listar-revendas-global)
- [Endpoints de Usuário](#endpoints-de-usuário)
  - [Criar Usuário](#criar-usuário)
  - [Criar Teste](#criar-teste)
  - [Renovar Usuário](#renovar-usuário)
  - [Editar Usuário](#editar-usuário)
  - [Suspender Usuário](#suspender-usuário)
  - [Reativar Usuário](#reativar-usuário)
  - [Excluir Usuário](#excluir-usuário)
  - [Listar Usuários](#listar-usuários)
  - [Listar Usuários Global](#listar-usuários-global)
- [Endpoints Online](#endpoints-online)
  - [Listar Usuários Online](#listar-usuários-online)
- [Endpoints DeviceID](#endpoints-deviceid)
  - [Excluir DeviceID por Usuário](#excluir-deviceid-por-usuário)
  - [Excluir DeviceID Global](#excluir-deviceid-global)
- [Webhooks](#webhooks)
  - [Asaas](#asaas)
  - [MercadoPago](#mercadopago)
- [Respostas de Erro](#respostas-de-erro)
- [Códigos de Status HTTP](#códigos-de-status-http)

---

## 📦 Coleção Postman

Para facilitar o teste e integração com a API, disponibilizamos uma coleção completa do Postman com todos os endpoints documentados.

**[Acessar Coleção no Postman →](https://web.postman.co/workspace/My-Workspace~a7e83321-d94b-42f4-89c0-39086e9a298a/collection/33914325-fb54e615-ea0a-4943-888e-621a896cec05?action=share&source=copy-link&creator=33914325)**

A coleção inclui:
- ✅ Todos os endpoints da API
- ✅ Exemplos de requisições pré-configuradas
- ✅ Variáveis de ambiente para fácil configuração
- ✅ Testes automatizados de resposta
- ✅ Documentação interativa

---

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

### Ativar Revenda

Ativa uma revenda suspensa e todos os seus usuários e sub-revendas. Permite ativar apenas uma categoria específica ou todas as categorias.

**Endpoint:** POST `/api/revenda/ativarevenda.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string", // Login da revenda (obrigatório)
  "categoria": "string" // Nome da categoria (opcional)
}
```

**Funcionamento:**

- Se `categoria` não for fornecida: ativa TODAS as atribuições da revenda
- Se `categoria` for fornecida: ativa APENAS a atribuição daquela categoria específica
- Processa recursivamente todas as sub-revendas
- Sincroniza todos os usuários nos servidores via endpoint `/sincronizar`

**Validações:**

- Revenda precisa existir
- Categoria precisa existir (se especificada)
- Usuário precisa ter permissão (nível >= 2)
- Servidores precisam estar disponíveis

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Revenda(s) ativada(s) com sucesso!",
  "data": {
    "login": "revenda123",
    "categoria": "Premium",
    "revendas_afetadas": ["revenda123", "subrevenda1", "subrevenda2"],
    "total_atribuicoes_ativadas": 3
  }
}
```

**Exemplos de Uso:**

Ativar todas as categorias:
```json
{
  "login": "revenda123"
}
```

Ativar apenas uma categoria:
```json
{
  "login": "revenda123",
  "categoria": "Premium"
}
```

### Suspender Revenda

Suspende uma revenda ativa, removendo todos os seus usuários e sub-revendas dos servidores. Permite suspender apenas uma categoria específica ou todas as categorias.

**Endpoint:** POST `/api/revenda/suspender.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string", // Login da revenda (obrigatório)
  "categoria": "string" // Nome da categoria (opcional)
}
```

**Funcionamento:**

- Se `categoria` não for fornecida: suspende TODAS as atribuições da revenda
- Se `categoria` for fornecida: suspende APENAS a atribuição daquela categoria específica
- Processa recursivamente todas as sub-revendas
- Remove todos os usuários dos servidores via endpoint `/excluir_global`

**Validações:**

- Revenda precisa existir
- Categoria precisa existir (se especificada)
- Usuário precisa ter permissão (nível >= 2)
- Servidores precisam estar disponíveis

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Revenda(s) suspensa(s) com sucesso!",
  "data": {
    "login": "revenda123",
    "categoria": "Premium",
    "revendas_afetadas": ["revenda123", "subrevenda1", "subrevenda2"],
    "total_usuarios_removidos": 150,
    "total_atribuicoes_suspensas": 3
  }
}
```

**Exemplos de Uso:**

Suspender todas as categorias:
```json
{
  "login": "revenda123"
}
```

Suspender apenas uma categoria:
```json
{
  "login": "revenda123",
  "categoria": "Premium"
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

### Listar Revendas

Lista todas as revendas que pertencem ao usuário logado (revendedor).

**Endpoint:** GET `/api/revenda/listarrevendas.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros de Query (opcionais):**

```
page=number          // Número da página (padrão: 1)
resultsPerPage=number // Registros por página (padrão: 10)
searchName=string    // Busca por login ou nome
status=string        // Filtro: 'proximo', 'ativo', 'expirado', 'suspenso'
todos=boolean        // Se 'true', retorna todos sem paginação
```

**Validações:**

- Usuário precisa ter nível de acesso de revenda
- Token válido obrigatório

**Resposta de Sucesso:**

```json
{
  "success": true,
  "data": {
    "revendas": [
      {
        "id": 1,
        "login": "revenda1",
        "nome": "Revenda Teste",
        "contato": "11999999999",
        "email": "revenda@teste.com",
        "limite": 100,
        "limitetest": 10,
        "tipo": "Validade",
        "expira": "2024-01-15 10:30:00",
        "expira_formatada": "15/01/2024 10:30:00",
        "status": "ativo",
        "dias_restantes": 30,
        "suspenso": false,
        "total_sub_revendas": 5,
        "total_usuarios": 25
      }
    ],
    "paginacao": {
      "pagina_atual": 1,
      "total_paginas": 5,
      "total_registros": 50,
      "registros_por_pagina": 10,
      "inicio": 1,
      "fim": 10
    },
    "filtros": {
      "searchName": "",
      "status": "ativo"
    }
  }
}
```

**Status possíveis:**

- `ativo`: Revenda com mais de 5 dias para expirar
- `proximo_expiracao`: Revenda com 5 dias ou menos para expirar
- `expira_hoje`: Revenda que expira hoje
- `expirado`: Revenda já expirada
- `suspenso`: Revenda suspensa

### Listar Revendas Global

Lista todas as revendas do sistema (apenas para administradores).

**Endpoint:** GET `/api/revenda/listarrevendas_global.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros de Query (opcionais):**

```
page=number          // Número da página (padrão: 1)
resultsPerPage=number // Registros por página (padrão: 10)
searchName=string    // Busca por login ou nome
status=string        // Filtro: 'vencido', 'proximo', 'ativo', 'suspenso', 'sem_atribuicao'
byid=number          // Filtro por revendedor pai específico
todos=boolean        // Se 'true', retorna todos sem paginação
estatisticas=boolean // Se 'true', retorna estatísticas do sistema
```

**Validações:**

- Apenas administradores (nível admin) podem acessar
- Token válido obrigatório

**Resposta de Sucesso (Listagem):**

```json
{
  "success": true,
  "data": {
    "revendas": [
      {
        "id": 1,
        "login": "revenda1",
        "nome": "Revenda Teste",
        "contato": "11999999999",
        "email": "revenda@teste.com",
        "limite": 100,
        "limitetest": 10,
        "tipo": "Validade",
        "expira": "2024-01-15 10:30:00",
        "expira_formatada": "15/01/2024 10:30:00",
        "status": "ativo",
        "dias_restantes": 30,
        "suspenso": false,
        "total_sub_revendas": 5,
        "total_usuarios": 25,
        "revendedor_pai": {
          "id": 2,
          "login": "admin",
          "nome": "Administrador"
        }
      }
    ],
    "paginacao": {
      "pagina_atual": 1,
      "total_paginas": 5,
      "total_registros": 100,
      "registros_por_pagina": 10,
      "inicio": 1,
      "fim": 10
    },
    "filtros": {
      "searchName": "",
      "status": "ativo",
      "byid": null
    }
  }
}
```

**Resposta de Sucesso (Estatísticas):**

```json
{
  "success": true,
  "data": {
    "total_revendas": 1000,
    "revendas_ativas": 750,
    "revendas_expiradas": 200,
    "revendas_proximo_expiracao": 50,
    "revendas_expira_hoje": 5,
    "revendas_suspensas": 25,
    "top_revendedores": [
      {
        "id": 2,
        "login": "revendedor1",
        "nome": "Maria Santos",
        "total_sub_revendas": 150
      }
    ],
    "distribuicao_tipos": [
      {
        "tipo": "Validade",
        "total": 600
      },
      {
        "tipo": "Credito",
        "total": 400
      }
    ]
  }
}
```

## Endpoints de Usuário

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
  "nome": "string", // Nome do usuário (obrigatório)
  "tipo": "string", // Tipo do usuário: v2ray, xray ou ssh (opcional, default: v2ray)
  "uuid": "string", // UUID para usuários v2ray/xray (opcional)
  "contato": "string" // Contato do usuário (opcional)
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
  "data": {
    "login": "string",
    "senha": "string",
    "limite": number,
    "expira": "string",
    "categoria": "string",
    "mensagem_personalizada": "string"
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
  "nome": "string", // Nome do usuário (opcional)
  "tipo": "string", // Tipo do usuário: v2ray, xray, ssh (opcional, default: ssh)
  "contato": "string", // Contato do usuário (opcional)
  "uuid": "string" // UUID (opcional)
}
```

**Validações:**

- Limite de teste por categoria e créditos disponíveis
- Suspensão e validade da atribuição

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

Renova a validade de um usuário existente por 31 dias, utilizando automaticamente o limite já cadastrado no banco de dados.

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

- O sistema irá renovar o usuário por 31 dias a partir da data atual
- O limite será mantido conforme já cadastrado para o usuário no banco de dados
- Não é necessário informar dias ou limite na requisição

**Validações:**

- Usuário precisa existir
- Usuário precisa ter permissão para renovar o usuário
- Para tipo "Credito": usuário precisa ter créditos disponíveis
- Para tipo "Validade": não pode reduzir limite abaixo dos usuários já criados

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário renovado com sucesso.",
  "data": {
    "login": "string",
    "nova_expira": "2025-06-17 19:42:00",
    "limite": 10
  }
}
```

### Editar Usuário

Edita um usuário existente, permitindo alterar todos os campos de forma opcional.

**Endpoint:** POST `/api/usuario/editar.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string", // Login do usuário a ser editado (obrigatório)
  "login_novo": "string", // Novo login (opcional)
  "senha": "string", // Nova senha (opcional)
  "limite": number, // Novo limite de conexões (opcional)
  "dias": number, // Dias a adicionar à validade (opcional)
  "nome": "string", // Novo nome (opcional)
  "contato": "string", // Novo contato (opcional)
  "valor": "string", // Novo valor (opcional)
  "categoriaid": number // Nova categoria (opcional)
}
```

**Funcionamento:**

- Apenas os campos fornecidos serão alterados
- Campos não fornecidos mantêm os valores atuais do banco de dados
- O parâmetro `dias` adiciona dias à validade atual (não substitui)
- Se `dias` não for fornecido, a data de expiração permanece inalterada

**Validações:**

- Usuário precisa existir
- Usuário precisa ter permissão para editar o usuário
- Para tipo "Credito": valida créditos disponíveis ao alterar limite
- Para tipo "Validade": valida limites da categoria
- Categoria precisa estar atribuída ao usuário
- Atribuição não pode estar suspensa ou vencida

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário editado com sucesso.",
  "data": {
    "login": "string",
    "senha": "string",
    "limite": 5,
    "expira": "2025-07-15 10:30:00",
    "nome": "João Silva",
    "contato": "62998612492",
    "valor": "29.90",
    "categoria": "Premium",
    "mensagem_personalizada": "string"
  }
}
```

**Exemplos de Uso:**

Editar apenas a senha:
```json
{
  "login": "usuario123",
  "senha": "novasenha123"
}
```

Editar limite e adicionar 30 dias:
```json
{
  "login": "usuario123",
  "limite": 3,
  "dias": 30
}
```

Editar múltiplos campos:
```json
{
  "login": "usuario123",
  "login_novo": "novologin123",
  "senha": "novasenha",
  "limite": 2,
  "dias": 30,
  "nome": "João Silva",
  "contato": "62998612492",
  "valor": "29.90"
}
```

### Suspender Usuário

Suspende um usuário ativo, removendo-o de todos os servidores.

**Endpoint:** POST `/api/usuario/suspender.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string" // Login do usuário (obrigatório)
}
```

**Funcionamento:**

- Verifica se o usuário está ativo (status = 1)
- Remove o usuário de todos os servidores (envia comando de exclusão)
- Atualiza o status no banco para suspenso (status = 0)
- Envia comandos em paralelo para todos os servidores cadastrados

**Validações:**

- Usuário precisa existir
- Usuário precisa estar ativo (não pode suspender usuário já suspenso)
- Usuário precisa ter permissão para gerenciar o usuário
- Atribuição não pode estar suspensa ou vencida
- Servidores precisam estar disponíveis

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário suspenso com sucesso.",
  "data": {
    "login": "usuario123",
    "status_anterior": "ativo",
    "status_atual": "suspenso"
  }
}
```

### Reativar Usuário

Reativa um usuário suspenso, criando-o novamente em todos os servidores.

**Endpoint:** POST `/api/usuario/reativar.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string" // Login do usuário (obrigatório)
}
```

**Funcionamento:**

- Verifica se o usuário está suspenso (status = 0)
- Calcula os dias restantes baseado na data de expiração
- Cria o usuário novamente em todos os servidores
- Atualiza o status no banco para ativo (status = 1)
- Envia comandos em paralelo para todos os servidores cadastrados

**Validações:**

- Usuário precisa existir
- Usuário precisa estar suspenso (não pode reativar usuário já ativo)
- Usuário precisa ter permissão para gerenciar o usuário
- Atribuição não pode estar suspensa ou vencida
- Servidores precisam estar disponíveis

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário reativado com sucesso.",
  "data": {
    "login": "usuario123",
    "status_anterior": "suspenso",
    "status_atual": "ativo"
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

### Listar Usuários

Lista todos os usuários que pertencem ao usuário logado (revendedor).

**Endpoint:** GET `/api/usuario/listarusuarios.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros de Query (opcionais):**

```
page=number          // Número da página (padrão: 1)
resultsPerPage=number // Registros por página (padrão: 10)
searchName=string    // Busca por login ou nome
status=string        // Filtro: 'proximo', 'ativo', 'expirado'
todos=boolean        // Se 'true', retorna todos sem paginação
```

**Validações:**

- Usuário precisa ter nível de acesso de revenda
- Token válido obrigatório

**Resposta de Sucesso:**

```json
{
  "success": true,
  "data": {
    "usuarios": [
      {
        "id": 1,
        "login": "usuario1",
        "nome": "João Silva",
        "limite": 5,
        "expira": "2024-01-15 10:30:00",
        "expira_formatada": "15/01/2024 10:30:00",
        "status": "ativo",
        "dias_restantes": 30,
        "categoria": {
          "id": 1,
          "nome": "Premium"
        },
        "plano": {
          "nome": "Plano Mensal",
          "duracao_dias": 30,
          "valor": "29.90"
        },
        "valor": "29.90"
      }
    ],
    "paginacao": {
      "pagina_atual": 1,
      "total_paginas": 5,
      "total_registros": 50,
      "registros_por_pagina": 10,
      "inicio": 1,
      "fim": 10
    },
    "filtros": {
      "searchName": "",
      "status": "ativo"
    }
  }
}
```

**Status possíveis:**

- `ativo`: Usuário com mais de 5 dias para expirar
- `proximo_expiracao`: Usuário com 5 dias ou menos para expirar
- `expira_hoje`: Usuário que expira hoje
- `expirado`: Usuário já expirado

### Listar Usuários Global

Lista todos os usuários do sistema (apenas para administradores).

**Endpoint:** GET `/api/usuario/listarusuarios_global.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros de Query (opcionais):**

```
page=number          // Número da página (padrão: 1)
resultsPerPage=number // Registros por página (padrão: 10)
searchName=string    // Busca por login ou nome
status=string        // Filtro: 'vencido', 'proximo', 'ativo'
byid=number          // Filtro por revendedor específico
todos=boolean        // Se 'true', retorna todos sem paginação
estatisticas=boolean // Se 'true', retorna estatísticas do sistema
```

**Validações:**

- Apenas administradores (nível admin) podem acessar
- Token válido obrigatório

**Resposta de Sucesso (Listagem):**

```json
{
  "success": true,
  "data": {
    "usuarios": [
      {
        "id": 1,
        "login": "usuario1",
        "nome": "João Silva",
        "limite": 5,
        "expira": "2024-01-15 10:30:00",
        "expira_formatada": "15/01/2024 10:30:00",
        "status": "ativo",
        "dias_restantes": 30,
        "categoria": {
          "id": 1,
          "nome": "Premium"
        },
        "plano": {
          "nome": "Plano Mensal",
          "duracao_dias": 30,
          "valor": "29.90"
        },
        "valor": "29.90",
        "revendedor": {
          "id": 2,
          "login": "revendedor1",
          "nome": "Maria Santos"
        }
      }
    ],
    "paginacao": {
      "pagina_atual": 1,
      "total_paginas": 5,
      "total_registros": 100,
      "registros_por_pagina": 10,
      "inicio": 1,
      "fim": 10
    },
    "filtros": {
      "searchName": "",
      "status": "ativo",
      "byid": null
    }
  }
}
```

**Resposta de Sucesso (Estatísticas):**

```json
{
  "success": true,
  "data": {
    "total_usuarios": 1000,
    "usuarios_ativos": 750,
    "usuarios_expirados": 200,
    "usuarios_proximo_expiracao": 50,
    "usuarios_expira_hoje": 5,
    "top_revendedores": [
      {
        "id": 2,
        "login": "revendedor1",
        "nome": "Maria Santos",
        "total_usuarios": 150
      }
    ],
    "distribuicao_categorias": [
      {
        "nome": "Premium",
        "total_usuarios": 500
      }
    ]
  }
}
```

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

## Endpoints DeviceID

### Excluir DeviceID por Usuário

Exclui todos os DeviceIDs associados a um usuário específico.

**Endpoint:** POST `/api/devaiceid/excluir.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

```json
{
  "login": "string" // Login do usuário (obrigatório)
}
```

**Validações:**

- Usuário precisa existir
- Token precisa ter permissão para gerenciar o usuário
- Apenas o dono do usuário ou administrador pode excluir

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "DeviceID(s) excluído(s) com sucesso.",
  "data": {
    "login": "usuario123",
    "userid": 42,
    "total_excluidos": 3
  }
}
```

**Resposta de Erro:**

```json
{
  "error": "Nenhum deviceid encontrado para este usuário."
}
```

### Excluir DeviceID Global

Exclui DeviceIDs em massa:
- **Administradores (nível 3)**: Exclui TODOS os DeviceIDs do sistema
- **Revendedores (nível 2)**: Exclui TODOS os DeviceIDs dos seus usuários

**Endpoint:** POST `/api/devaiceid/excluir_global.php`

**Headers:**

```
Content-Type: application/json
Authorization: Bearer {token}
```

**Parâmetros:**

Nenhum parâmetro necessário.

**Validações:**

- Administradores (nível = 3) podem excluir todos os DeviceIDs do sistema
- Revendedores (nível = 2) podem excluir apenas DeviceIDs dos seus usuários
- Sistema precisa ter DeviceIDs cadastrados

**Resposta de Sucesso (Administrador):**

```json
{
  "success": true,
  "message": "Todos os DeviceIDs foram excluídos com sucesso.",
  "data": {
    "total_excluidos": 150,
    "tipo": "global"
  }
}
```

**Resposta de Sucesso (Revendedor):**

```json
{
  "success": true,
  "message": "DeviceIDs dos seus usuários foram excluídos com sucesso.",
  "data": {
    "total_excluidos": 25,
    "total_usuarios": 10,
    "tipo": "revenda"
  }
}
```

**Resposta de Erro:**

```json
{
  "error": "Nenhum deviceid encontrado para seus usuários."
}
```

## Webhooks

### Asaas

Processa webhooks de pagamento do Asaas para renovações e compras de usuários e revendas.

**Endpoint:** POST `/api/webhooks/asaas.php`

**Headers:**

```
Content-Type: application/json
```

**Parâmetros (enviados pelo Asaas):**

```json
{
  "event": "string", // Tipo do evento (PAYMENT_RECEIVED, PAYMENT_OVERDUE, etc.)
  "payment": {
    "id": "string",
    "customer": "string",
    "value": number,
    "netValue": number,
    "status": "string",
    "billingType": "string",
    "dueDate": "string",
    "description": "string",
    "externalReference": "string"
  }
}
```

**Eventos suportados:**

- `PAYMENT_RECEIVED`: Pagamento confirmado
- `PAYMENT_OVERDUE`: Pagamento em atraso
- `PAYMENT_DELETED`: Pagamento cancelado

**Funcionalidades:**

- Renovação automática de usuários
- Renovação automática de revendas
- Atualização de limites e planos
- Logs detalhados de processamento

### MercadoPago

Processa webhooks de pagamento do MercadoPago para renovações e compras.

**Endpoint:** POST `/api/webhooks/mercadopago.php`

**Headers:**

```
Content-Type: application/json
```

**Parâmetros (enviados pelo MercadoPago):**

```json
{
  "action": "string",
  "data": {
    "id": "string"
  }
}
```

**Ações suportadas:**

- `payment.created`: Pagamento criado
- `payment.updated`: Pagamento atualizado
- `payment.cancelled`: Pagamento cancelado

**Funcionalidades:**

- Renovação automática de usuários
- Atualização de limites e planos
- Logs detalhados de processamento

## Respostas de Erro

Em caso de erro, a API retorna um status code apropriado junto com uma mensagem descritiva:

```json
{
  "error": "Descrição do erro"
}
```

**Exemplos de respostas de erro:**

```json
{
  "error": "Campo obrigatório não fornecido: login"
}
```

```json
{
  "error": "Usuário não tem permissão para excluir este usuário"
}
```

```json
{
  "error": "Token inválido"
}
```

## Códigos de Status HTTP

A API utiliza os seguintes códigos de status HTTP:

| Código | Descrição             | Uso                               |
| ------ | --------------------- | --------------------------------- |
| 200    | OK                    | Requisição processada com sucesso |
| 201    | Created               | Recurso criado com sucesso        |
| 400    | Bad Request           | Parâmetros inválidos ou faltando  |
| 401    | Unauthorized          | Token inválido/expirado           |
| 403    | Forbidden             | Sem permissão para a ação         |
| 404    | Not Found             | Recurso não encontrado            |
| 405    | Method Not Allowed    | Método HTTP não permitido         |
| 500    | Internal Server Error | Erro interno do servidor          |

**Códigos de Erro Comuns:**

- **400**: Parâmetros inválidos ou faltando
- **401**: Token inválido/expirado
- **403**: Sem permissão para a ação
- **404**: Recurso não encontrado
- **405**: Método HTTP não permitido
- **500**: Erro interno do servidor

## Exemplos de Uso

### Exemplo 1: Criar um usuário

```bash
curl -X POST https://seudominio.com/api/usuario/criar.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui" \
  -d '{
    "login": "usuario123",
    "senha": "senha123",
    "dias": 30,
    "limite": 5,
    "nome": "João Silva",
    "tipo": "ssh"
  }'
```

### Exemplo 2: Criar um teste

```bash
curl -X POST https://seudominio.com/api/usuario/criar_teste.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui" \
  -d '{
    "login": "teste123",
    "senha": "senha123",
    "minutos": 120,
    "nome": "Teste Usuário",
    "tipo": "ssh"
  }'
```

### Exemplo 3: Editar um usuário (apenas senha)

```bash
curl -X POST https://seudominio.com/api/usuario/editar.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui" \
  -d '{
    "login": "usuario123",
    "senha": "novasenha123"
  }'
```

### Exemplo 4: Editar múltiplos campos do usuário

```bash
curl -X POST https://seudominio.com/api/usuario/editar.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui" \
  -d '{
    "login": "usuario123",
    "login_novo": "novologin123",
    "senha": "novasenha",
    "limite": 2,
    "dias": 30,
    "nome": "João Silva",
    "contato": "62998612492",
    "valor": "29.90"
  }'
```

### Exemplo 5: Suspender um usuário

```bash
curl -X POST https://seudominio.com/api/usuario/suspender.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui" \
  -d '{
    "login": "usuario123"
  }'
```

### Exemplo 6: Reativar um usuário

```bash
curl -X POST https://seudominio.com/api/usuario/reativar.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui" \
  -d '{
    "login": "usuario123"
  }'
```

### Exemplo 7: Listar usuários com paginação

```bash
curl -X GET "https://seudominio.com/api/usuario/listarusuarios.php?page=1&resultsPerPage=10&status=ativo" \
  -H "Authorization: Bearer seu-token-aqui"
```

### Exemplo 8: Obter estatísticas globais

```bash
curl -X GET "https://seudominio.com/api/usuario/listarusuarios_global.php?estatisticas=true" \
  -H "Authorization: Bearer seu-token-aqui"
```

### Exemplo 9: Listar revendas com paginação

```bash
curl -X GET "https://seudominio.com/api/revenda/listarrevendas.php?page=1&resultsPerPage=10&status=ativo" \
  -H "Authorization: Bearer seu-token-aqui"
```

### Exemplo 10: Obter estatísticas de revendas

```bash
curl -X GET "https://seudominio.com/api/revenda/listarrevendas_global.php?estatisticas=true" \
  -H "Authorization: Bearer seu-token-aqui"
```

### Exemplo 11: Excluir DeviceID de um usuário

```bash
curl -X POST https://seudominio.com/api/devaiceid/excluir.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui" \
  -d '{
    "login": "usuario123"
  }'
```

### Exemplo 12: Excluir todos os DeviceIDs (Admin)

```bash
curl -X POST https://seudominio.com/api/devaiceid/excluir_global.php \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer seu-token-aqui"
```

## Notas Importantes

1. **Autenticação**: Todos os endpoints (exceto webhooks) requerem autenticação via token Bearer
2. **Timezone**: A API utiliza o timezone `America/Sao_Paulo`
3. **Encoding**: Todas as requisições e respostas devem usar UTF-8
4. **Rate Limiting**: Considere implementar rate limiting em produção
5. **Logs**: A API gera logs detalhados para auditoria e debugging
6. **Segurança**: Sempre use HTTPS em produção
7. **Categoria**: A categoria do usuário é automaticamente definida baseada na atribuição do token
8. **Limites**: Os limites de caracteres para login e senha são definidos na tabela `config.maxtext`
