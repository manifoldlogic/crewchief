#!/bin/bash
set -e

echo "🔄 Running post-start setup..."

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

# Ensure maproom-postgres is ready (optional - may not be running)
# This is external to devcontainer, so we don't block if it's not available
if pg_isready -h maproom-postgres -p 5432 -U maproom -d maproom 2>/dev/null; then
    echo "✓ maproom-postgres is available"
else
    echo "⚠️  maproom-postgres not available (start with: cd packages/maproom-mcp && docker compose up -d)"
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