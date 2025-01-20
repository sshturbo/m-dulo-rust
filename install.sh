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
print_left() {
    printf "%s\n" "$1"
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
        print_left "$package já está instalado."
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
    print_left "Baixando Docker binário para arquitetura $ARCH..."
    run_with_spinner "wget -q -O /tmp/$DOCKER_TGZ $DOCKER_URL" "Baixando Docker"

    print_left "Extraindo arquivos do Docker..."
    run_with_spinner "tar xzvf /tmp/$DOCKER_TGZ -C /tmp" "Extraindo Docker"

    print_left "Movendo binários para /usr/bin/..."
    run_with_spinner "cp /tmp/docker/* /usr/bin/" "Movendo binários"

    print_left "Iniciando o daemon do Docker..."
    run_with_spinner "dockerd &>/dev/null" "Iniciando Docker"

    print_left "Docker instalado com sucesso!"
else
    print_left "Docker já está instalado."
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
    print_left "Docker Compose já está instalado."
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
    print_left "Diretório $APP_DIR já existe. Excluindo antigo..."
    run_with_spinner "systemctl stop $SERVICE_FILE_NAME &>/dev/null" "Parando serviço"
    run_with_spinner "systemctl disable $SERVICE_FILE_NAME &>/dev/null" "Desabilitando serviço"
    rm -rf $APP_DIR
fi
mkdir -p $APP_DIR

# Baixar e configurar o módulo
print_left "Baixando $FILE_NAME..."
run_with_spinner "wget --timeout=30 -O $APP_DIR/$FILE_NAME $FILE_URL/$FILE_NAME" "Baixando módulo"

print_left "Extraindo arquivos..."
run_with_spinner "unzip $APP_DIR/$FILE_NAME -d $APP_DIR &>/dev/null && rm $APP_DIR/$FILE_NAME" "Extraindo módulo"
run_with_spinner "sleep 0.1" "Progresso" # Simulando progresso

# Configurar arquivo .env
run_with_spinner "cp $APP_DIR/.env.exemple $APP_DIR/.env" "Configurando .env"
sed -i "s|DATABASE_URL=.*|DATABASE_URL=\"postgres://postgres:$AUTHENTICATION_API_KEY@localhost:5432/postgres\"|" "$APP_DIR/.env"
chmod -R 775 $APP_DIR

# Configurar docker-compose.yml
if [ -f "$DOCKER_COMPOSE_FILE" ]; then
    sed -i "s/POSTGRES_PASSWORD:.*/POSTGRES_PASSWORD: $AUTHENTICATION_API_KEY/" $DOCKER_COMPOSE_FILE
    print_left "POSTGRES_PASSWORD atualizado em $DOCKER_COMPOSE_FILE"
else
    echo "Erro: Arquivo $DOCKER_COMPOSE_FILE não encontrado."
    exit 1
fi

# Iniciar serviços com docker-compose
print_left "Iniciando os serviços com docker-compose..."
run_with_spinner "docker-compose -f $DOCKER_COMPOSE_FILE up -d &>/dev/null" "Iniciando serviços"

# Configurar serviço systemd
if [ -f "$APP_DIR/$SERVICE_FILE_NAME" ]; then
    run_with_spinner "cp $APP_DIR/$SERVICE_FILE_NAME /etc/systemd/system/ &>/dev/null" "Configurando serviço"
    run_with_spinner "chmod 644 /etc/systemd/system/$SERVICE_FILE_NAME &>/dev/null" "Ajustando permissões"
    run_with_spinner "systemctl daemon-reload &>/dev/null" "Recarregando systemd"
    run_with_spinner "systemctl enable $SERVICE_FILE_NAME &>/dev/null" "Habilitando serviço"
    run_with_spinner "systemctl start $SERVICE_FILE_NAME &>/dev/null" "Iniciando serviço"
    print_left "Serviço $SERVICE_FILE_NAME configurado e iniciado com sucesso!"
else
    print_left "Erro: Arquivo de serviço não encontrado."
    exit 1
fi

run_with_spinner "sleep 0.1" "Finalizando configuração"
print_left "Módulo instalado e configurado com sucesso!"
