#!/bin/bash
set -e

echo "🔄 Running post-start setup..."

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

# Start tmux session if not already running
if ! tmux has-session -t crewchief 2>/dev/null; then
    echo "🖥️  Creating tmux session 'crewchief'..."
    tmux new-session -d -s crewchief -n main
    tmux send-keys -t crewchief:main "cd /workspace" C-m
    tmux send-keys -t crewchief:main "clear" C-m
    
    # Create additional windows
    tmux new-window -t crewchief -n cli
    tmux send-keys -t crewchief:cli "cd /workspace/packages/cli" C-m
    
    tmux new-window -t crewchief -n maproom
    tmux send-keys -t crewchief:maproom "cd /workspace/crates/maproom" C-m
    
    echo "✓ tmux session 'crewchief' created"
    echo "  Use 'tmux attach -t crewchief' to attach"
fi

echo "✅ Post-start setup complete"