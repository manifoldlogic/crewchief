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

# Install Oh My Zsh if not already installed
# print_step "Checking for Oh My Zsh..."
# if [ ! -d "$HOME/.oh-my-zsh" ]; then
#     print_step "Installing Oh My Zsh..."
#     sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended
#     print_success "Oh My Zsh installed"
# else
#     print_success "Oh My Zsh already installed"
# fi

# Install Claude Code if not already installed
print_step "Checking for Claude Code..."
if ! command -v claude &> /dev/null; then
    print_step "Installing Claude Code..."
    curl -fsSL https://claude.ai/install.sh | bash || print_error "Failed to install Claude Code"
    print_success "Claude Code installed"
else
    print_success "Claude Code already installed"
fi

# Install Husky globally
print_step "Installing Husky globally..."
npm install -g husky || print_error "Failed to install Husky"
print_success "Husky installed globally"

# Install CrewChief CLI globally
print_step "Installing CrewChief CLI globally..."
npm install -g @crewchief/cli@latest || print_error "Failed to install CrewChief CLI"
print_success "CrewChief CLI installed globally"

# Install TypeScript Language Server globally
print_step "Installing TypeScript Language Server globally..."
npm install -g typescript typescript-language-server || print_error "Failed to install TypeScript Language Server"
print_success "TypeScript Language Server installed globally"

# Install Go tools
if command -v go &> /dev/null; then
    print_step "Installing MCP Language Server..."
    go install github.com/isaacphi/mcp-language-server@latest || print_error "Failed to install MCP Language Server"
    print_success "MCP Language Server installed"
else
    print_error "Go not found, skipping MCP Language Server installation"
fi

# Install pnpm dependencies
print_step "Installing Node.js dependencies..."
pnpm install
print_success "Node.js dependencies installed"

# Fix ownership of volume-mounted directories (Docker creates them as root)
print_step "Fixing volume permissions..."
sudo chown -R vscode:vscode /home/vscode/.config/gh 2>/dev/null || true
print_success "Volume permissions fixed"

# Copy host .gitconfig to writable location (avoids "Device or resource busy" on macOS)
print_step "Setting up Git configuration..."
if [ -f "/home/vscode/.gitconfig-host" ]; then
    # Only copy if container .gitconfig doesn't exist or is older than host version
    if [ ! -f "/home/vscode/.gitconfig" ] || [ "/home/vscode/.gitconfig-host" -nt "/home/vscode/.gitconfig" ]; then
        cp /home/vscode/.gitconfig-host /home/vscode/.gitconfig
        print_success "Copied host .gitconfig to container"
    else
        print_success "Container .gitconfig is up to date"
    fi
else
    print_step "No host .gitconfig found, creating default..."
fi

# Set up git configuration
print_step "Configuring Git..."
git config --global --add safe.directory /workspace
git config --global core.editor "cursor --wait"
print_success "Git configured"

# Link repository-managed Bash configuration
print_step "Linking repo-managed .bashrc..."
BASHRC_SOURCE="/workspace/.devcontainer/.bashrc"
if [ -f "$BASHRC_SOURCE" ]; then
    ln -sf "$BASHRC_SOURCE" "$HOME/.bashrc"
    print_success ".bashrc linked to repository version"
else
    print_error "Missing $BASHRC_SOURCE - update or recreate this file"
fi

print_step "Note: .zshrc is still sourced from the host environment if you switch shells"

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    print_step "Creating .env file from example..."
    cp .env.example .env 2>/dev/null || cat > .env << EOF
# CrewChief Environment Variables
NODE_ENV=development
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

    // For devcontainer: use legacy path to keep worktrees in repo
    // (Easier for volume mounting and cleanup)
    worktreeBasePath: '.crewchief/worktrees',

    // Default (v2.0+) would be: '~/.crewchief/worktrees/<repo-name>'
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
# print_step "Setting up Claude Code..."
# if [ ! -f /home/vscode/.claude/config.json ]; then
#     mkdir -p /home/vscode/.claude
#     cat > /home/vscode/.claude/config.json << 'EOF'
# {
#   "dangerousMode": true,
#   "apiKey": "${ANTHROPIC_API_KEY}",
#   "model": "claude-3-opus-20240229",
#   "maxTokens": 4096,
#   "temperature": 0,
#   "autoSave": true,
#   "workspaceRoot": "/workspace"
# }
# EOF
#     print_success "Claude Code configured"
# fi

print_success "🎉 CrewChief devcontainer setup complete!"
echo ""
echo "Quick start commands:"
echo "  claude     - Run Claude Code in dangerous mode"
echo "  crewchief  - Run the CrewChief CLI (globally installed)"
echo "  ccdev      - Run the CrewChief CLI in development mode"
echo ""
echo "⚠️  Claude Code dangerous mode is ENABLED"
echo "   - Network access is restricted via iptables"
echo "   - Only allowed domains can be accessed"
echo "   - Run 'claude' to start Claude Code"
echo ""
echo "Happy coding! 🚀"