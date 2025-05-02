#!/bin/bash

# Nome do serviço
SERVICE_NAME="m-dulo"

# Caminho da aplicação
APP_PATH="/opt/myapp"

echo "Parando o serviço $SERVICE_NAME..."
systemctl stop "$SERVICE_NAME"

echo "Desabilitando o serviço $SERVICE_NAME..."
systemctl disable "$SERVICE_NAME"

echo "Removendo arquivos do serviço..."
rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
rm -f "/lib/systemd/system/${SERVICE_NAME}.service"

# Atualiza o systemctl
systemctl daemon-reload

echo "Removendo diretório da aplicação em $APP_PATH..."
rm -rf "$APP_PATH"

# --- REMOÇÃO DE REDIS ---
if systemctl list-units --type=service | grep -q "redis"; then
    echo "Parando e desabilitando o Redis..."
    systemctl stop redis
    systemctl disable redis

    echo "Removendo Redis..."
    apt-get purge -y redis-server
    apt-get autoremove -y
    rm -rf /etc/redis /var/lib/redis
else
    echo "Redis não encontrado ou já removido."
fi

# --- REMOÇÃO DE POSTGRESQL ---
if systemctl list-units --type=service | grep -q "postgresql"; then
    echo "Parando e desabilitando o PostgreSQL..."
    systemctl stop postgresql
    systemctl disable postgresql

    echo "Removendo PostgreSQL..."
    apt-get purge -y postgresql*
    apt-get autoremove -y
    rm -rf /etc/postgresql /var/lib/postgresql
else
    echo "PostgreSQL não encontrado ou já removido."
fi

# --- REMOÇÃO DO DIRETÓRIO .cloudflared ---
CLOUDFLARED_DIR="$HOME/.cloudflared"

if [ -d "$CLOUDFLARED_DIR" ]; then
    echo "Removendo diretório .cloudflared em $CLOUDFLARED_DIR..."
    rm -rf "$CLOUDFLARED_DIR"
    echo "Diretório .cloudflared removido com sucesso."
else
    echo "Diretório .cloudflared não encontrado ou já removido."
fi

echo "Parando e removendo todos os contêineres Docker..."
docker stop $(docker ps -aq)
docker rm $(docker ps -aq)

echo "Removendo todas as imagens Docker..."
docker rmi -f $(docker images -q)

echo "Pronto! Tudo foi removido e as portas liberadas."

