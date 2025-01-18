#!/bin/bash

# Verifica se o script está sendo executado como root
if [[ $EUID -ne 0 ]]; then
    echo "Este script deve ser executado como root"
    exit 1
fi

# Função para centralizar texto
print_centered() {
    printf "%*s\n" $(((${#1} + $(tput cols)) / 2)) "$1"
}

# Função para simular uma barra de progresso
progress_bar() {
    local total_steps=$1
    local current_step=0

    echo -n "Progresso: ["
    while [ $current_step -lt $total_steps ]; do
        echo -n "#"
        ((current_step++))
        sleep 0.1
    done
    echo "] Completo!"
}

# Definir função run_with_spinner
run_with_spinner() {
    local command=$1
    local message=$2

    echo -n "$message"
    $command &
    pid=$!

    while kill -0 $pid 2>/dev/null; do
        echo -n "."
        sleep 1
    done

    wait $pid
    echo " Done!"
}

# Verifica se o Docker está instalado
if ! command -v docker &>/dev/null; then
    run_with_spinner "sudo apt update >/dev/null 2>&1" "Atualizando o apt"
    run_with_spinner "sudo apt install -y docker.io >/dev/null 2>&1" "Instalando Docker"
    run_with_spinner "sudo systemctl start docker >/dev/null 2>&1" "Iniciando Docker"
    run_with_spinner "sudo systemctl enable docker >/dev/null 2>&1" "Habilitando inicialização automática do Docker"
else
    print_centered "Docker já está instalado."
fi

# Verifica se o Docker Compose está instalado
if ! command -v docker-compose &>/dev/null; then
    run_with_spinner "sudo apt install -y curl >/dev/null 2>&1" "Instalando curl (necessário para o Docker Compose)"
    run_with_spinner "sudo curl -L 'https://github.com/docker/compose/releases/download/$(curl -s https://api.github.com/repos/docker/compose/releases/latest | grep -oP '(?<=\"tag_name\": \").*?(?=\")')/docker-compose-$(uname -s)-$(uname -m)' -o /usr/local/bin/docker-compose" "Baixando Docker Compose"
    run_with_spinner "sudo chmod +x /usr/local/bin/docker-compose" "Aplicando permissões ao Docker Compose"
else
    print_centered "Docker Compose já está instalado."
fi

# Gera uma chave de autenticação para o Postgres
AUTHENTICATION_API_KEY=$(openssl rand -hex 16)
print_centered "Chave de autenticação gerada: $AUTHENTICATION_API_KEY"

# Modifica o arquivo docker-compose.yml
if [ -f "docker-compose.yml" ]; then
    print_centered "Adicionando POSTGRES_PASSWORD no arquivo docker-compose.yml..."
    sed -i "s|POSTGRES_PASSWORD:.*|POSTGRES_PASSWORD: $AUTHENTICATION_API_KEY|" docker-compose.yml || {
        echo "Erro ao modificar o arquivo docker-compose.yml."
        exit 1
    }
else
    echo "Erro: Arquivo docker-compose.yml não encontrado."
    exit 1
fi

# Inicia o serviço com docker-compose
print_centered "Iniciando os serviços com docker-compose..."
docker-compose up -d &>/dev/null

if [ $? -eq 0 ]; then
    print_centered "Serviços iniciados com sucesso!"
else
    echo "Erro ao iniciar os serviços."
    exit 1
fi

DEPENDENCIES=("unzip" "wget")
NEED_INSTALL=()

# Verificar dependências
for dep in "${DEPENDENCIES[@]}"; do
    if ! command -v $dep &>/dev/null; then
        NEED_INSTALL+=($dep)
    else
        print_centered "$dep já está instalado."
    fi
done

# Instalar dependências necessárias
for dep in "${NEED_INSTALL[@]}"; do
    print_centered "Instalando $dep..."
    apt install -y $dep
done

# Detectar arquitetura
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        FILE_NAME="m-dulo-rust-x86_64-unknown-linux-musl.zip"
        FILE_URL="https://github.com/sshturbo/m-dulo-rust/releases/download/1.0.1/$FILE_NAME"
        ;;
    aarch64)
        FILE_NAME="m-dulo-rust-aarch64-unknown-linux-musl.zip"
        FILE_URL="https://github.com/sshturbo/m-dulo-rust/releases/download/1.0.1/$FILE_NAME"
        ;;
    *)
        echo "Arquitetura $ARCH não suportada."
        exit 1
        ;;
esac

# Configuração do diretório /opt/myapp/
if [ -d "/opt/myapp/" ]; then
    print_centered "Diretório /opt/myapp/ já existe. Excluindo antigo..."
    systemctl stop m-dulo.service &>/dev/null
    systemctl disable m-dulo.service &>/dev/null
    systemctl daemon-reload &>/dev/null
    rm -rf /opt/myapp/
fi

mkdir -p /opt/myapp/

# Baixar e configurar o repositório
print_centered "Baixando $FILE_NAME..."
wget --timeout=30 -O /opt/myapp/$FILE_NAME "$FILE_URL" &>/dev/null

if [ $? -ne 0 ]; then
    echo "Erro ao baixar o arquivo. Verifique a URL ou a conexão."
    exit 1
fi

print_centered "Extraindo arquivos..."
unzip /opt/myapp/$FILE_NAME -d /opt/myapp/ &>/dev/null && rm /opt/myapp/$FILE_NAME
progress_bar 5

# Atualizar permissões do diretório /opt/myapp/
print_centered "Atualizando permissões..."
chmod -R 775 /opt/myapp/

# Configurar serviço systemd
if [ -f "/opt/myapp/m-dulo.service" ]; then
    print_centered "Configurando serviço systemd..."
    cp /opt/myapp/m-dulo.service /etc/systemd/system/
    chown root:root /etc/systemd/system/m-dulo.service
    chmod 644 /etc/systemd/system/m-dulo.service
    systemctl daemon-reload
    systemctl enable m-dulo.service
    systemctl start m-dulo.service
else
    print_centered "Erro: Arquivo m-dulo.service não encontrado."
    exit 1
fi