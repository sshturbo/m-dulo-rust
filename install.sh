#!/bin/bash

# Variáveis
APP_DIR="/opt/myapp"
DEPENDENCIES=("unzip" "wget")
VERSION="1.0.2"
AUTHENTICATION_API_KEY=$(openssl rand -hex 16)
FILE_URL="https://github.com/sshturbo/m-dulo-rust/releases/download/$VERSION"
ARCH=$(uname -m)
case $ARCH in
x86_64) FILE_NAME="m-dulo-x86_64-unknown-linux-musl.zip" ;;
aarch64) FILE_NAME="m-dulo-aarch64-unknown-linux-musl.zip" ;;
*)
    echo "Arquitetura $ARCH não suportada."
    exit 1
    ;;
esac

# Verifica se o script está sendo executado como root
if [[ $EUID -ne 0 ]]; then
    echo "Este script deve ser executado como root"
    exit 1
fi

# Função para centralizar texto
print_centered() {
    printf "%*s\n" $(((${#1} + $(tput cols)) / 2)) "$1"
}

# Função para exibir barra de progresso
progress_bar() {
    local total_steps=$1
    for ((i = 0; i < total_steps; i++)); do
        echo -n "#"
        sleep 0.1
    done
    echo "] Completo!"
}

# Executar comandos com spinner
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
    echo " Feito!"
}

# Instalação de pacotes com verificação
install_if_missing() {
    local package=$1
    if ! command -v $package &>/dev/null; then
        run_with_spinner "sudo apt install -y $package >/dev/null 2>&1" "Instalando $package"
    else
        print_centered "$package já está instalado."
    fi
}

# Verificar e instalar Docker
if ! command -v docker &>/dev/null; then
    run_with_spinner "sudo apt update >/dev/null 2>&1" "Atualizando apt"
    run_with_spinner "sudo apt install -y docker.io >/dev/null 2>&1" "Instalando Docker"
    run_with_spinner "sudo systemctl start docker >/dev/null 2>&1" "Iniciando Docker"
    run_with_spinner "sudo systemctl enable docker >/dev/null 2>&1" "Habilitando inicialização automática do Docker"
else
    print_centered "Docker já está instalado."
fi

# Verificar e instalar Docker Compose
if ! command -v docker-compose &>/dev/null; then
    install_if_missing "curl"
    run_with_spinner "sudo curl -L 'https://github.com/docker/compose/releases/download/$(curl -s https://api.github.com/repos/docker/compose/releases/latest | grep -oP '(?<=\"tag_name\": \").*?(?=\")')/docker-compose-$(uname -s)-$(uname -m)' -o /usr/local/bin/docker-compose" "Baixando Docker Compose"
    run_with_spinner "sudo chmod +x /usr/local/bin/docker-compose" "Configurando Docker Compose"
else
    print_centered "Docker Compose já está instalado."
fi

# Gera uma chave de autenticação
print_centered "Chave de autenticação gerada: $AUTHENTICATION_API_KEY"

# Verificar e criar diretório de aplicação
mkdir -p $APP_DIR

# Instalar dependências
for dep in "${DEPENDENCIES[@]}"; do
    install_if_missing $dep
done

# Baixar e configurar o módulo
print_centered "Baixando $FILE_NAME..."
wget --timeout=30 -O "$APP_DIR/$FILE_NAME" "$FILE_URL/$FILE_NAME" &>/dev/null || {
    echo "Erro ao baixar o arquivo."
    exit 1
}

print_centered "Extraindo arquivos..."
unzip "$APP_DIR/$FILE_NAME" -d "$APP_DIR" &>/dev/null && rm "$APP_DIR/$FILE_NAME"
progress_bar 5

# Copiar .env.example para .env
cp "$APP_DIR/.env.example" "$APP_DIR/.env"

# Atualizar URL do banco de dados no arquivo .env
sed -i "s|DATABASE_URL=.*|DATABASE_URL=\"postgres://postgres:$AUTHENTICATION_API_KEY@localhost:5432/postgres\"|" "$APP_DIR/.env"

# Atualizar permissões
chmod -R 775 $APP_DIR

# Configurar docker-compose.yml
DOCKER_COMPOSE_FILE="$APP_DIR/docker-compose.yml"
if [ -f "$DOCKER_COMPOSE_FILE" ]; then
    sed -i "s/POSTGRES_PASSWORD:.*/POSTGRES_PASSWORD: $AUTHENTICATION_API_KEY/" $DOCKER_COMPOSE_FILE
    print_centered "POSTGRES_PASSWORD atualizado em $DOCKER_COMPOSE_FILE"
else
    echo "Erro: Arquivo $DOCKER_COMPOSE_FILE não encontrado."
    exit 1
fi

# Iniciar serviços com docker-compose
print_centered "Iniciando os serviços com docker-compose..."
docker-compose -f $DOCKER_COMPOSE_FILE up -d &>/dev/null &&
    print_centered "Serviços iniciados com sucesso!" ||
    {
        echo "Erro ao iniciar os serviços."
        exit 1
    }

# Configurar serviço systemd
SERVICE_FILE="$APP_DIR/m-dulo.service"
if [ -f "$SERVICE_FILE" ]; then
    cp "$SERVICE_FILE" /etc/systemd/system/
    chmod 644 /etc/systemd/system/m-dulo.service
    systemctl daemon-reload
    systemctl enable m-dulo.service
    systemctl start m-dulo.service
    print_centered "Serviço m-dulo configurado e iniciado com sucesso!"
else
    print_centered "Erro: Arquivo de serviço não encontrado."
    exit 1
fi

progress_bar 10
print_centered "Modulos instalado e configurado com sucesso!"
