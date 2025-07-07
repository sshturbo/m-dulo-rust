# 🚀 API do Painel Web Pro

**Versão:** 1.0  
**Data de Atualização:** Julho 2025  
**URL Base:** `https://seu-dominio.com`

## 📋 Índice

- [🔐 Introdução e Autenticação](#-introdução-e-autenticação)
  - [Obtenção do Token](#obtenção-do-token)
  - [Formato de Autenticação](#formato-de-autenticação)
- [🏢 Endpoints de Revenda](#-endpoints-de-revenda)
  - [Criar Revenda](#criar-revenda)
  - [Renovar Revenda](#renovar-revenda)
  - [Excluir Revenda](#excluir-revenda)
  - [Listar Revendas](#listar-revendas)
- [👤 Endpoints de Usuário](#-endpoints-de-usuário)
  - [Criar Usuário](#criar-usuário)
  - [Criar Teste](#criar-teste)
  - [Renovar Usuário](#renovar-usuário)
  - [Excluir Usuário](#excluir-usuário)
  - [Listar Usuários](#listar-usuários)
- [🌐 Endpoints Online](#-endpoints-online)
  - [Listar Usuários Online](#listar-usuários-online)
- [⚠️ Tratamento de Erros](#️-tratamento-de-erros)
- [📖 Exemplos de Integração](#-exemplos-de-integração)
- [📝 Changelog](#-changelog)

## 🔐 Introdução e Autenticação

A API do Painel Web Pro permite gerenciar revendas, usuários e monitorar conexões online de forma programática. Esta documentação fornece todas as informações necessárias para integrar com nossa API RESTful.

### Características Principais

- ✅ **RESTful API** com JSON
- ✅ **Autenticação via Bearer Token**
- ✅ **Rate Limiting** incorporado
- ✅ **Validação de dados** robusta
- ✅ **Códigos de erro** padronizados
- ✅ **Timezone** São Paulo/Brasil

### Obtenção do Token

1. Acesse o painel administrativo
2. Navegue até **Configurações** → **API**
3. Gere um novo token ou copie o token existente
4. Mantenha o token seguro e não o compartilhe

### Formato de Autenticação

Todas as requisições devem incluir o header de autorização:

```http
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```

### Exemplo de Requisição Básica

```bash
curl -X GET "https://seu-dominio.com/api/online/listall.php" \
  -H "Authorization: Bearer seu-token-aqui" \
  -H "Content-Type: application/json"
```

## 🏢 Endpoints de Revenda

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
      "id": 123,
      "login": "revenda01",
      "nome": "Revenda Premium LTDA"
    },
    "atribuicao": {
      "tipo": "Credito",
      "limite": 100,
      "limitetest": 10,
      "expira": "2025-08-07 23:59:59"
    }
  }
}
```

**Exemplo Completo:**

```bash
curl -X POST "https://seu-dominio.com/api/revenda/criar.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "login": "revenda01",
    "senha": "senha123forte",
    "nome": "Revenda Premium LTDA",
    "contato": "+55 11 99999-9999",
    "email": "contato@revenda01.com",
    "limite": 100,
    "limitetest": 10,
    "dias": 30
  }'
```

**Resposta de Erro:**

```json
{
  "error": "Login já existe no sistema"
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
    "login": "revenda01",
    "tipo": "Credito",
    "dias": 30,
    "limite": 150,
    "limitetest": 15
  }
}
```

**Exemplo Completo:**

```bash
curl -X POST "https://seu-dominio.com/api/revenda/renovar.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "login": "revenda01",
    "dias": 30,
    "limite": 150,
    "limitetest": 15
  }'
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
    "revendas_excluidas": 1,
    "arquivo": "revenda01_backup.sql"
  }
}
```

**Exemplo Completo:**

```bash
curl -X POST "https://seu-dominio.com/api/revenda/excluir.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "login": "revenda01"
  }'
```

### Listar Revendas

Lista todas as revendas com suas respectivas informações e status.

**Endpoint:** GET `/api/revenda/listar.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros de Consulta (Opcionais):**

- `page` (number): Página da listagem (default: 1)
- `limit` (number): Itens por página (default: 50, max: 100)
- `status` (string): Filtrar por status (ativo, vencido, suspenso)
- `tipo` (string): Filtrar por tipo (Credito, Validade)

**Exemplo de URL:**

```
GET /api/revenda/listar.php?page=1&limit=25&status=ativo
```

**Resposta de Sucesso:**

```json
{
  "success": true,
  "data": [
    {
      "id": 123,
      "login": "revenda01",
      "nome": "Revenda Premium LTDA",
      "email": "contato@revenda01.com",
      "contato": "+55 11 99999-9999",
      "tipo": "Credito",
      "limite": 100,
      "limitetest": 10,
      "usado": 25,
      "usado_test": 3,
      "expira": "2025-08-07 23:59:59",
      "status": "ativo",
      "criado": "2025-07-07 10:30:00"
    }
  ],
  "pagination": {
    "current_page": 1,
    "total_pages": 5,
    "total_items": 125,
    "items_per_page": 25
  }
}
```

**Exemplo Completo:**

```bash
curl -X GET "https://seu-dominio.com/api/revenda/listar.php?page=1&limit=25" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
```

## 👤 Endpoints de Usuário

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
  "userData": {
    "login": "usuario01",
    "senha": "senha123",
    "expira": "2025-08-07 23:59:59",
    "limite": 5,
    "tipo": "v2ray"
  }
}
```

**Exemplo Completo:**

```bash
curl -X POST "https://seu-dominio.com/api/usuario/criar.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "login": "usuario01",
    "senha": "senha123forte",
    "dias": 30,
    "limite": 5,
    "tipo": "v2ray",
    "nome": "João Silva",
    "contato": "+55 11 88888-8888"
  }'
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
    "login": "teste01",
    "senha": "teste123",
    "limite": 1,
    "expira": "2025-07-07 18:30:00",
    "categoria": "ssh"
  }
}
```

**Exemplo Completo:**

```bash
curl -X POST "https://seu-dominio.com/api/usuario/criar_teste.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "login": "teste01",
    "senha": "teste123",
    "minutos": 30,
    "tipo": "ssh",
    "nome": "Teste 30 min"
  }'
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

**Resposta de Sucesso:**

```json
{
  "success": true,
  "message": "Usuário renovado com sucesso.",
  "data": {
    "login": "usuario01",
    "nova_expira": "2025-08-07 19:42:00",
    "limite": 5
  }
}
```

**Exemplo Completo:**

```bash
curl -X POST "https://seu-dominio.com/api/usuario/renovar.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "login": "usuario01"
  }'
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
    "login": "usuario01"
  }
}
```

**Exemplo Completo:**

```bash
curl -X POST "https://seu-dominio.com/api/usuario/excluir.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "login": "usuario01"
  }'
```

### Listar Usuários

Lista todos os usuários com informações detalhadas e paginação.

**Endpoint:** GET `/api/usuario/listar.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros de Consulta (Opcionais):**

- `page` (number): Página da listagem (default: 1)
- `limit` (number): Itens por página (default: 50, max: 100)
- `status` (string): Filtrar por status (ativo, vencido, suspenso)
- `tipo` (string): Filtrar por tipo (v2ray, xray, ssh)
- `dono` (string): Filtrar por proprietário

**Exemplo de URL:**

```
GET /api/usuario/listar.php?page=1&limit=25&status=ativo&tipo=v2ray
```

**Resposta de Sucesso:**

```json
{
  "success": true,
  "data": [
    {
      "id": 456,
      "login": "usuario01",
      "nome": "João Silva",
      "contato": "+55 11 88888-8888",
      "tipo": "v2ray",
      "limite": 5,
      "expira": "2025-08-07 23:59:59",
      "status": "ativo",
      "dono": "revenda01",
      "criado": "2025-07-07 10:30:00",
      "ultimo_acesso": "2025-07-07 15:45:30"
    }
  ],
  "pagination": {
    "current_page": 1,
    "total_pages": 8,
    "total_items": 200,
    "items_per_page": 25
  }
}
```

**Exemplo Completo:**

```bash
curl -X GET "https://seu-dominio.com/api/usuario/listar.php?page=1&limit=25" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
```

## 🌐 Endpoints Online

### Listar Usuários Online

Retorna todos os usuários online atualmente cadastrados na tabela `api_online`, com informações do tempo online calculado.

**Endpoint:** GET `/api/online/listall.php`

**Headers:**

```
Authorization: Bearer {token}
```

**Parâmetros:**

Nenhum parâmetro necessário.

**Resposta de Sucesso:**

```json
[
  {
    "login": "usuario1",
    "limite": 10,
    "tipo": "premium",
    "ip": "192.168.0.1",
    "start_time": "2025-07-07 20:00:00",
    "tempo_online": "03:15:42",
    "dono": "admin"
  },
  {
    "login": "usuario2",
    "limite": 5,
    "tipo": "normal",
    "ip": "192.168.0.2",
    "start_time": "2025-07-07 21:30:00",
    "tempo_online": "01:45:10",
    "dono": "revenda01"
  }
]
```

**Exemplo Completo:**

```bash
curl -X GET "https://seu-dominio.com/api/online/listall.php" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
```

**Observações:**
- O campo `tempo_online` é calculado em tempo real pela diferença entre o campo `start_time` e a data/hora atual de São Paulo
- O campo `dono` corresponde ao login do proprietário da conta

## ⚠️ Tratamento de Erros

### Códigos de Status HTTP

| Código | Descrição | Situação |
|--------|-----------|----------|
| `200` | Sucesso | Requisição processada com sucesso |
| `400` | Bad Request | Parâmetros inválidos ou faltando |
| `401` | Unauthorized | Token inválido ou expirado |
| `403` | Forbidden | Sem permissão para executar a ação |
| `404` | Not Found | Recurso não encontrado |
| `429` | Too Many Requests | Rate limit excedido |
| `500` | Internal Server Error | Erro interno do servidor |

### Formato de Erro

Todas as respostas de erro seguem o formato padrão:

```json
{
  "error": "Descrição detalhada do erro",
  "code": "ERROR_CODE_INTERNAL",
  "timestamp": "2025-07-07T18:30:00-03:00"
}
```

### Exemplos de Erros Comuns

**Token Inválido (401):**
```json
{
  "error": "Token de acesso inválido ou expirado",
  "code": "INVALID_TOKEN",
  "timestamp": "2025-07-07T18:30:00-03:00"
}
```

**Login já existe (400):**
```json
{
  "error": "Login já existe no sistema",
  "code": "LOGIN_EXISTS",
  "timestamp": "2025-07-07T18:30:00-03:00"
}
```

**Sem permissão (403):**
```json
{
  "error": "Usuário sem permissão para executar esta ação",
  "code": "INSUFFICIENT_PERMISSIONS",
  "timestamp": "2025-07-07T18:30:00-03:00"
}
```

## 📖 Exemplos de Integração

### Integração em PHP

```php
<?php
class PainelWebProAPI {
    private $baseUrl;
    private $token;
    
    public function __construct($baseUrl, $token) {
        $this->baseUrl = rtrim($baseUrl, '/');
        $this->token = $token;
    }
    
    private function makeRequest($endpoint, $data = null, $method = 'GET') {
        $curl = curl_init();
        
        curl_setopt_array($curl, [
            CURLOPT_URL => $this->baseUrl . $endpoint,
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_HTTPHEADER => [
                'Authorization: Bearer ' . $this->token,
                'Content-Type: application/json'
            ]
        ]);
        
        if ($method === 'POST' && $data) {
            curl_setopt($curl, CURLOPT_POST, true);
            curl_setopt($curl, CURLOPT_POSTFIELDS, json_encode($data));
        }
        
        $response = curl_exec($curl);
        $httpCode = curl_getinfo($curl, CURLINFO_HTTP_CODE);
        curl_close($curl);
        
        if ($httpCode !== 200) {
            throw new Exception("API Error: HTTP $httpCode");
        }
        
        return json_decode($response, true);
    }
    
    public function criarUsuario($dados) {
        return $this->makeRequest('/api/usuario/criar.php', $dados, 'POST');
    }
    
    public function listarOnline() {
        return $this->makeRequest('/api/online/listall.php');
    }
    
    public function criarRevenda($dados) {
        return $this->makeRequest('/api/revenda/criar.php', $dados, 'POST');
    }
}

// Exemplo de uso
$api = new PainelWebProAPI('https://seu-dominio.com', 'seu-token');

try {
    $usuario = $api->criarUsuario([
        'login' => 'teste123',
        'senha' => 'senha123',
        'dias' => 30,
        'limite' => 5,
        'tipo' => 'v2ray',
        'nome' => 'Usuário Teste'
    ]);
    
    echo "Usuário criado: " . $usuario['userData']['login'];
} catch (Exception $e) {
    echo "Erro: " . $e->getMessage();
}
?>
```

### Integração em Python

```python
import requests
import json
from datetime import datetime

class PainelWebProAPI:
    def __init__(self, base_url, token):
        self.base_url = base_url.rstrip('/')
        self.token = token
        self.headers = {
            'Authorization': f'Bearer {token}',
            'Content-Type': 'application/json'
        }
    
    def _make_request(self, endpoint, data=None, method='GET'):
        url = f"{self.base_url}{endpoint}"
        
        if method == 'GET':
            response = requests.get(url, headers=self.headers)
        elif method == 'POST':
            response = requests.post(url, headers=self.headers, json=data)
        
        response.raise_for_status()
        return response.json()
    
    def criar_usuario(self, dados):
        return self._make_request('/api/usuario/criar.php', dados, 'POST')
    
    def listar_online(self):
        return self._make_request('/api/online/listall.php')
    
    def criar_revenda(self, dados):
        return self._make_request('/api/revenda/criar.php', dados, 'POST')

# Exemplo de uso
api = PainelWebProAPI('https://seu-dominio.com', 'seu-token')

try:
    usuario = api.criar_usuario({
        'login': 'teste123',
        'senha': 'senha123',
        'dias': 30,
        'limite': 5,
        'tipo': 'v2ray',
        'nome': 'Usuário Teste'
    })
    
    print(f"Usuário criado: {usuario['userData']['login']}")
except requests.RequestException as e:
    print(f"Erro: {e}")
```

### Integração em JavaScript (Node.js)

```javascript
const axios = require('axios');

class PainelWebProAPI {
    constructor(baseUrl, token) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.token = token;
        this.headers = {
            'Authorization': `Bearer ${token}`,
            'Content-Type': 'application/json'
        };
    }
    
    async makeRequest(endpoint, data = null, method = 'GET') {
        const url = `${this.baseUrl}${endpoint}`;
        
        try {
            let response;
            if (method === 'GET') {
                response = await axios.get(url, { headers: this.headers });
            } else if (method === 'POST') {
                response = await axios.post(url, data, { headers: this.headers });
            }
            
            return response.data;
        } catch (error) {
            throw new Error(`API Error: ${error.response?.status} - ${error.response?.data?.error}`);
        }
    }
    
    async criarUsuario(dados) {
        return await this.makeRequest('/api/usuario/criar.php', dados, 'POST');
    }
    
    async listarOnline() {
        return await this.makeRequest('/api/online/listall.php');
    }
    
    async criarRevenda(dados) {
        return await this.makeRequest('/api/revenda/criar.php', dados, 'POST');
    }
}

// Exemplo de uso
const api = new PainelWebProAPI('https://seu-dominio.com', 'seu-token');

(async () => {
    try {
        const usuario = await api.criarUsuario({
            login: 'teste123',
            senha: 'senha123',
            dias: 30,
            limite: 5,
            tipo: 'v2ray',
            nome: 'Usuário Teste'
        });
        
        console.log(`Usuário criado: ${usuario.userData.login}`);
    } catch (error) {
        console.error(`Erro: ${error.message}`);
    }
})();
```

## 📝 Changelog

### Versão 1.0 (Julho 2025)
- ✅ Implementação inicial da API
- ✅ Endpoints de revenda (criar, renovar, excluir, listar)
- ✅ Endpoints de usuário (criar, renovar, excluir, listar)
- ✅ Endpoint de usuários online
- ✅ Sistema de autenticação via Bearer Token
- ✅ Validação robusta de dados
- ✅ Tratamento de erros padronizado
- ✅ Rate limiting implementado
- ✅ Timezone configurado para São Paulo/Brasil

---

**Suporte Técnico:** Para dúvidas ou problemas com a API, entre em contato através do painel administrativo.

**Documentação Atualizada:** Esta documentação é atualizada regularmente. Verifique a versão e data no topo do documento.
