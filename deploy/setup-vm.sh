#!/usr/bin/env bash
set -euo pipefail

echo "===================================================="
echo "      Battler Server VM Automated Setup"
echo "===================================================="

# 1. Configure 2 GB Swap file to ensure stability on 1 GB e2-micro
SWAPFILE="/swapfile"
if ! grep -q "$SWAPFILE" /proc/swaps; then
    echo "=> Setting up 2 GB swapfile for memory safety..."
    sudo fallocate -l 2G "$SWAPFILE" || sudo dd if=/dev/zero of="$SWAPFILE" bs=1M count=2048
    sudo chmod 600 "$SWAPFILE"
    sudo mkswap "$SWAPFILE"
    sudo swapon "$SWAPFILE"
    if ! grep -q "$SWAPFILE" /etc/fstab; then
        echo "$SWAPFILE none swap sw 0 0" | sudo tee -a /etc/fstab
    fi
    # Set swappiness to 10 (prefer RAM, use swap only under pressure)
    sudo sysctl vm.swappiness=10
    echo "vm.swappiness=10" | sudo tee -a /etc/sysctl.d/99-swappiness.conf
    echo "   ✅ 2 GB swapfile active."
else
    echo "   ℹ️ Swapfile already active."
fi

# 2. Install Docker & Docker Compose if not already installed
if ! command -v docker &>/dev/null; then
    echo "=> Installing Docker..."
    sudo apt-get update
    sudo apt-get install -y ca-certificates curl gnupg lsb-release

    sudo install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    sudo chmod a+r /etc/apt/keyrings/docker.gpg

    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
      $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

    sudo apt-get update
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
    sudo systemctl enable docker
    sudo systemctl start docker
    sudo usermod -aG docker "$USER" || true
    echo "   ✅ Docker installed successfully."
else
    echo "   ℹ️ Docker is already installed."
fi

# 3. Create deploy directory
DEPLOY_DIR="/opt/battler"
sudo mkdir -p "$DEPLOY_DIR"
sudo chown -R "$USER:$USER" "$DEPLOY_DIR"

echo "=> Copying deployment configuration files to $DEPLOY_DIR..."
# If run from repository clone, copy files; otherwise download from raw github
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/Caddyfile" && -f "$SCRIPT_DIR/docker-compose.prod.yml" ]]; then
    cp "$SCRIPT_DIR/Caddyfile" "$DEPLOY_DIR/Caddyfile"
    cp "$SCRIPT_DIR/docker-compose.prod.yml" "$DEPLOY_DIR/docker-compose.prod.yml"
else
    BRANCH="${BRANCH:-main}"
    curl -fsSL "https://raw.githubusercontent.com/jackson-nestelroad/battler/${BRANCH}/deploy/Caddyfile" -o "$DEPLOY_DIR/Caddyfile"
    curl -fsSL "https://raw.githubusercontent.com/jackson-nestelroad/battler/${BRANCH}/deploy/docker-compose.prod.yml" -o "$DEPLOY_DIR/docker-compose.prod.yml"
fi

# 4. Pull pre-built image and start containers
echo "=> Pulling latest battler-server and caddy containers..."
cd "$DEPLOY_DIR"
docker compose -f docker-compose.prod.yml pull || true
docker compose -f docker-compose.prod.yml up -d

echo ""
echo "===================================================="
echo "🎉 Battler services launched successfully!"
echo "===================================================="
docker compose -f docker-compose.prod.yml ps
echo ""
echo "Check logs anytime with:"
echo "   docker compose -f $DEPLOY_DIR/docker-compose.prod.yml logs -f"
