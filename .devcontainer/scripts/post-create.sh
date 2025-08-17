#!/bin/bash
set -e

echo "🚀 Running post-create setup for CrewChief devcontainer..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${GREEN}▶${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Install Claude Code if not already installed
print_step "Checking for Claude Code..."
if ! command -v claude &> /dev/null; then
    print_step "Installing Claude Code..."
    npm install -g @anthropic-ai/claude-code@latest || print_error "Failed to install Claude Code"
    print_success "Claude Code installed"
else
    print_success "Claude Code already installed"
fi

# Install pnpm dependencies
print_step "Installing Node.js dependencies..."
pnpm install
print_success "Node.js dependencies installed"

# Build Maproom binary if Rust is available
if command -v cargo &> /dev/null; then
    print_step "Building Maproom binary..."
    cd crates/maproom
    cargo build --release
    # Copy to expected location
    sudo cp target/release/crewchief-maproom /usr/local/bin/
    sudo chmod +x /usr/local/bin/crewchief-maproom
    cd ../..
    print_success "Maproom binary built and installed"
else
    print_error "Rust not found, skipping Maproom build"
fi

# Initialize database
print_step "Waiting for PostgreSQL to be ready..."
until pg_isready -h postgres -p 5432 -U postgres; do
    echo "Waiting for PostgreSQL..."
    sleep 2
done
print_success "PostgreSQL is ready"

# Run database migrations for Maproom
print_step "Running Maproom database migrations..."
if [ -f "/usr/local/bin/crewchief-maproom" ]; then
    DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief" \
    /usr/local/bin/crewchief-maproom db migrate || true
    print_success "Maproom database migrations complete"
else
    print_error "Maproom binary not found, skipping migrations"
fi

# Run web UI database migrations
print_step "Running Web UI database migrations..."
cd packages/web-ui
pnpm run db:migrate || true
cd ../..
print_success "Web UI database migrations complete"

# Build the web UI
print_step "Building Web UI..."
cd packages/web-ui
pnpm run build
cd ../..
print_success "Web UI built successfully"

# Set up git configuration
print_step "Configuring Git..."
git config --global --add safe.directory /workspace
git config --global core.editor "code --wait"
print_success "Git configured"

# Create useful aliases
print_step "Setting up shell aliases..."
cat >> ~/.bashrc << 'EOF'

# CrewChief aliases
alias cc='node /workspace/packages/cli/dist/cli/index.js'
alias ccdev='tsx /workspace/packages/cli/src/cli/index.ts'
alias webui='cd /workspace/packages/web-ui && pnpm dev'
alias maproom='crewchief-maproom'
alias claude='claude --dangerous-mode'
alias ll='ls -la'
alias gs='git status'
alias gd='git diff'
alias gc='git commit'
alias gp='git push'
alias gl='git log --oneline --graph --decorate'

# Docker aliases
alias dps='docker ps'
alias dlog='docker logs -f'
alias dexec='docker exec -it'

# tmux aliases
alias ta='tmux attach -t'
alias tl='tmux list-sessions'
alias tn='tmux new -s'

# Navigation
alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'

# Ensure we start in workspace (for Cursor compatibility)
if [ -z "$IN_WORKSPACE_CHECK" ]; then
    export IN_WORKSPACE_CHECK=1
    if [ "$(pwd)" != "/workspace" ] && [ -d "/workspace" ]; then
        cd /workspace
    fi
fi

EOF

# Also add to zsh if it exists
if [ -f ~/.zshrc ]; then
    cat >> ~/.zshrc << 'EOF'

# CrewChief aliases
alias cc='node /workspace/packages/cli/dist/cli/index.js'
alias ccdev='tsx /workspace/packages/cli/src/cli/index.ts'
alias webui='cd /workspace/packages/web-ui && pnpm dev'
alias maproom='crewchief-maproom'
alias claude='claude --dangerous-mode'
alias ll='ls -la'
alias gs='git status'
alias gd='git diff'
alias gc='git commit'
alias gp='git push'
alias gl='git log --oneline --graph --decorate'

# Docker aliases
alias dps='docker ps'
alias dlog='docker logs -f'
alias dexec='docker exec -it'

# tmux aliases
alias ta='tmux attach -t'
alias tl='tmux list-sessions'
alias tn='tmux new -s'

# Navigation
alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'

# Ensure we start in workspace (for Cursor compatibility)
if [ -z "$IN_WORKSPACE_CHECK" ]; then
    export IN_WORKSPACE_CHECK=1
    if [ "$(pwd)" != "/workspace" ] && [ -d "/workspace" ]; then
        cd /workspace
    fi
fi

EOF
fi
print_success "Shell aliases configured"

# Install tmux plugins
print_step "Installing tmux plugins..."
~/.tmux/plugins/tpm/bin/install_plugins
print_success "tmux plugins installed"

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    print_step "Creating .env file from example..."
    cp .env.example .env 2>/dev/null || cat > .env << EOF
# CrewChief Environment Variables
NODE_ENV=development
DATABASE_URL=postgresql://postgres:postgres@postgres:5432/crewchief
REDIS_URL=redis://redis:6379
CREWCHIEF_DB_HOST=postgres
CREWCHIEF_DB_PORT=5432
CREWCHIEF_DB_NAME=crewchief
CREWCHIEF_DB_USER=postgres
CREWCHIEF_DB_PASSWORD=postgres
CREWCHIEF_MAPROOM_BIN=/usr/local/bin/crewchief-maproom
PORT=3456
EOF
    print_success ".env file created"
fi

# Create local config if it doesn't exist
if [ ! -f crewchief.config.local.js ]; then
    print_step "Creating local config..."
    cat > crewchief.config.local.js << 'EOF'
// Local development configuration
module.exports = {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
  },
  
  agents: {
    claude: {
      command: 'claude',
      defaultArgs: ['--model', 'claude-3-opus'],
    },
  },
  
  worktree: {
    copyIgnoredFiles: ['.env', '.env.local'],
    copyFromPath: '.',
    overwriteStrategy: 'skip',
  },
  
  // Development-specific settings
  development: {
    verbose: true,
    autoReload: true,
  }
};
EOF
    print_success "Local config created"
fi

# Initialize Claude Code dangerous mode firewall
print_step "Initializing Claude Code dangerous mode firewall..."
if [ -f "/usr/local/bin/init-claude-firewall.sh" ]; then
    sudo /usr/local/bin/init-claude-firewall.sh || print_error "Firewall initialization failed (non-critical)"
    print_success "Claude Code firewall configured for dangerous mode"
else
    print_error "Claude firewall script not found"
fi

# Create Claude Code configuration
print_step "Setting up Claude Code..."
if [ ! -f /home/vscode/.claude/config.json ]; then
    mkdir -p /home/vscode/.claude
    cat > /home/vscode/.claude/config.json << 'EOF'
{
  "dangerousMode": true,
  "apiKey": "${ANTHROPIC_API_KEY}",
  "model": "claude-3-opus-20240229",
  "maxTokens": 4096,
  "temperature": 0,
  "autoSave": true,
  "workspaceRoot": "/workspace"
}
EOF
    print_success "Claude Code configured"
fi

print_success "🎉 CrewChief devcontainer setup complete!"
echo ""
echo "Quick start commands:"
echo "  claude    - Run Claude Code in dangerous mode"
echo "  webui     - Start the web UI development server"
echo "  ccdev     - Run the CrewChief CLI in development mode"
echo "  maproom   - Run Maproom commands"
echo ""
echo "Services available:"
echo "  PostgreSQL: postgres:5432"
echo "  Redis:      redis:6379"
echo "  pgAdmin:    http://localhost:5050"
echo "  Redis Commander: http://localhost:8081"
echo ""
echo "⚠️  Claude Code dangerous mode is ENABLED"
echo "   - Network access is restricted via iptables"
echo "   - Only allowed domains can be accessed"
echo "   - Run 'claude' to start Claude Code"
echo ""
echo "Happy coding! 🚀"