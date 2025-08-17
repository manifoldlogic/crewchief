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
alias cc='node packages/cli/dist/cli/index.js'
alias ccdev='tsx packages/cli/src/cli/index.ts'
alias webui='cd packages/web-ui && pnpm dev'
alias maproom='crewchief-maproom'
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

EOF

# Also add to zsh if it exists
if [ -f ~/.zshrc ]; then
    cat >> ~/.zshrc << 'EOF'

# CrewChief aliases
alias cc='node packages/cli/dist/cli/index.js'
alias ccdev='tsx packages/cli/src/cli/index.ts'
alias webui='cd packages/web-ui && pnpm dev'
alias maproom='crewchief-maproom'
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

print_success "🎉 CrewChief devcontainer setup complete!"
echo ""
echo "Quick start commands:"
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
echo "Happy coding! 🚀"