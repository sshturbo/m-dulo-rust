#!/bin/bash

# ===============================
# Configurações e Variáveis Globais
# ===============================
APP_DIR="/opt/myapp"
DEPENDENCIES=("unzip" "iptables")
VERSION="1.0.2"
AUTHENTICATION_API_KEY=$(openssl rand -hex 16)
FILE_URL="https://github.com/sshturbo/m-dulo-rust/releases/download/$VERSION"
DOCKER_VERSION="27.5.0"
DOCKER_BASE_URL="https://download.docker.com/linux/static/stable"
DOCKER_COMPOSE_RELEASE_URL="https://github.com/docker/compose/releases/latest"
ARCH=$(uname -m)
DOCKER_TGZ="docker-$DOCKER_VERSION.tgz"
SERVICE_FILE_NAME="m-dulo.service"
DOCKER_COMPOSE_BIN="/usr/local/bin/docker-compose"
DOCKER_COMPOSE_FILE="$APP_DIR/docker-compose.yml"

# Determinar arquitetura e nome do arquivo para download
case $ARCH in
x86_64)
    FILE_NAME="m-dulo-x86_64-unknown-linux-musl.zip"
    DOCKER_ARCH="x86_64"
    ;;
aarch64)
    FILE_NAME="m-dulo-aarch64-unknown-linux-musl.zip"
    DOCKER_ARCH="aarch64"
    ;;
*)
    echo "Arquitetura $ARCH não suportada."
    exit 1
    ;;
esac

DOCKER_URL="$DOCKER_BASE_URL/$DOCKER_ARCH/$DOCKER_TGZ"

# ===============================
# Funções Utilitárias
# ===============================
print_centered() {
    printf "%*s\n" $(((${#1} + $(tput cols)) / 2)) "$1"
}

progress_bar() {
    local total_steps=$1
    for ((i = 0; i < total_steps; i++)); do
        echo -n "#"
        sleep 0.1
    done
    echo " Completo!"
}

run_with_spinner() {
    local command="$1"
    local message="$2"
    echo -n "$message"
    $command &>/dev/null &
    local pid=$!
    while kill -0 $pid 2>/dev/null; do
        echo -n "."
        sleep 1
    done
    wait $pid
    echo " Feito!"
}

install_if_missing() {
    local package=$1
    if ! command -v $package &>/dev/null; then
        run_with_spinner "sudo apt install -y $package" "Instalando $package"
    else
        print_centered "$package já está instalado."
    fi
}

# ===============================
# Validações Iniciais
# ===============================
if [[ $EUID -ne 0 ]]; then
    echo "Este script deve ser executado como root."
    exit 1
fi

# ===============================
# Instalação do Docker
# ===============================
if ! command -v docker &>/dev/null; then
    print_centered "Baixando Docker binário para arquitetura $ARCH..."
    wget -q -O "/tmp/$DOCKER_TGZ" "$DOCKER_URL" || {
        echo "Erro ao baixar o Docker binário."
        exit 1
    }

    print_centered "Extraindo arquivos do Docker..."
    tar xzvf "/tmp/$DOCKER_TGZ" -C /tmp &>/dev/null

    print_centered "Movendo binários para /usr/bin/..."
    cp /tmp/docker/* /usr/bin/
    chmod +x /usr/bin/docker*

    print_centered "Iniciando o daemon do Docker..."
    dockerd &
    print_centered "Docker instalado com sucesso!"
else
    print_centered "Docker já está instalado."
fi

# Limpeza temporária
rm -rf /tmp/docker /tmp/$DOCKER_TGZ

# ===============================
# Instalação do Docker Compose
# ===============================
if ! command -v docker-compose &>/dev/null; then
    install_if_missing "wget"
    COMPOSE_VERSION=$(curl -s "$DOCKER_COMPOSE_RELEASE_URL" | grep -oP '(?<="tag_name": ").*?(?=")')
    COMPOSE_URL="https://github.com/docker/compose/releases/download/$COMPOSE_VERSION/docker-compose-$(uname -s)-$(uname -m)"

    run_with_spinner "sudo wget -q -O $DOCKER_COMPOSE_BIN $COMPOSE_URL" "Baixando Docker Compose"
    run_with_spinner "sudo chmod +x $DOCKER_COMPOSE_BIN" "Configurando Docker Compose"
else
    print_centered "Docker Compose já está instalado."
fi

# ===============================
# Configuração da Aplicação
# ===============================
# Instalar dependências
for dep in "${DEPENDENCIES[@]}"; do
    install_if_missing $dep
done

# Configurar diretório da aplicação
if [ -d "$APP_DIR" ]; then
    print_centered "Diretório $APP_DIR já existe. Excluindo antigo..."
    systemctl stop $SERVICE_FILE_NAME &>/dev/null
    systemctl disable $SERVICE_FILE_NAME &>/dev/null
    rm -rf $APP_DIR
fi
mkdir -p $APP_DIR

# Baixar e configurar o módulo
print_centered "Baixando $FILE_NAME..."
wget --timeout=30 -O "$APP_DIR/$FILE_NAME" "$FILE_URL/$FILE_NAME" &>/dev/null || {
    echo "Erro ao baixar o arquivo."
    exit 1
}

print_centered "Extraindo arquivos..."
unzip "$APP_DIR/$FILE_NAME" -d "$APP_DIR" &>/dev/null && rm "$APP_DIR/$FILE_NAME"
progress_bar 5

# Configurar arquivo .env
cp "$APP_DIR/.env.exemple" "$APP_DIR/.env"
sed -i "s|DATABASE_URL=.*|DATABASE_URL=\"postgres://postgres:$AUTHENTICATION_API_KEY@localhost:5432/postgres\"|" "$APP_DIR/.env"
chmod -R 775 $APP_DIR

# Configurar docker-compose.yml
if [ -f "$DOCKER_COMPOSE_FILE" ]; then
    sed -i "s/POSTGRES_PASSWORD:.*/POSTGRES_PASSWORD: $AUTHENTICATION_API_KEY/" $DOCKER_COMPOSE_FILE
    print_centered "POSTGRES_PASSWORD atualizado em $DOCKER_COMPOSE_FILE"
else
    echo "Erro: Arquivo $DOCKER_COMPOSE_FILE não encontrado."
    exit 1
fi

# Iniciar serviços com docker-compose
print_centered "Iniciando os serviços com docker-compose..."
if docker-compose -f $DOCKER_COMPOSE_FILE up -d &>/dev/null; then
    print_centered "Serviços iniciados com sucesso!"
else
    echo "Erro ao iniciar os serviços. Verifique os logs do Docker para mais detalhes."
    docker-compose -f $DOCKER_COMPOSE_FILE logs
    exit 1
fi

# Configurar serviço systemd
if [ -f "$APP_DIR/$SERVICE_FILE_NAME" ]; then
    cp "$APP_DIR/$SERVICE_FILE_NAME" /etc/systemd/system/
    chmod 644 /etc/systemd/system/$SERVICE_FILE_NAME
    systemctl daemon-reload
    systemctl enable $SERVICE_FILE_NAME
    systemctl start $SERVICE_FILE_NAME
    print_centered "Serviço $SERVICE_FILE_NAME configurado e iniciado com sucesso!"
else
    print_centered "Erro: Arquivo de serviço não encontrado."
    exit 1
fi

progress_bar 10
print_centered "Módulo instalado e configurado com sucesso!"
