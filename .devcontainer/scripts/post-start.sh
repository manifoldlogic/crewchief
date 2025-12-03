#!/bin/bash
set -e

echo "🔄 Running post-start setup..."

# Sync .gitconfig from host if it's newer (avoids "Device or resource busy" on macOS)
if [ -f "/home/vscode/.gitconfig-host" ]; then
    if [ ! -f "/home/vscode/.gitconfig" ] || [ "/home/vscode/.gitconfig-host" -nt "/home/vscode/.gitconfig" ]; then
        cp /home/vscode/.gitconfig-host /home/vscode/.gitconfig
        echo "✓ Updated .gitconfig from host"
    fi
fi

# Detect and export the host path for /workspace (needed for Docker-in-Docker volume mounts)
WORKSPACE_HOST_PATH=$(docker inspect $(hostname) --format '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}' 2>/dev/null || echo "/workspace")
export WORKSPACE_HOST_PATH
echo "✓ WORKSPACE_HOST_PATH: $WORKSPACE_HOST_PATH"

# Ensure .bashrc points to the repository-managed configuration
BASHRC_SOURCE="/workspace/.devcontainer/.bashrc"
if [ -f "$BASHRC_SOURCE" ]; then
    if [ ! -L "$HOME/.bashrc" ] || [ "$(readlink -f "$HOME/.bashrc" 2>/dev/null)" != "$BASHRC_SOURCE" ]; then
        ln -sf "$BASHRC_SOURCE" "$HOME/.bashrc"
        echo "✓ Updated ~/.bashrc to use repository configuration"
    fi
else
    echo "⚠️  Repository .bashrc not found at $BASHRC_SOURCE"
fi

# Fix Docker socket permissions for docker-in-docker
if [ -S /var/run/docker.sock ]; then
    sudo chown root:docker /var/run/docker.sock 2>/dev/null || true
    echo "✓ Fixed Docker socket permissions"
fi

# Fix gh CLI config permissions (Docker volumes are created as root)
if [ -d "/home/vscode/.config/gh" ]; then
    sudo chown -R vscode:vscode /home/vscode/.config/gh 2>/dev/null || true
fi

# Update dependencies if package.json has changed
if [ -f /workspace/.devcontainer/.last-package-json-hash ]; then
    CURRENT_HASH=$(sha256sum package.json | cut -d' ' -f1)
    LAST_HASH=$(cat /workspace/.devcontainer/.last-package-json-hash)
    
    if [ "$CURRENT_HASH" != "$LAST_HASH" ]; then
        echo "📦 package.json has changed, updating dependencies..."
        pnpm install
    fi
else
    sha256sum package.json | cut -d' ' -f1 > /workspace/.devcontainer/.last-package-json-hash
fi

echo "✅ Post-start setup complete"