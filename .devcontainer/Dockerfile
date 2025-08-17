FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

# Install Node.js first (required for npm commands)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

# Install system dependencies including network tools for Claude Code dangerous mode
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    tmux \
    postgresql-client \
    redis-tools \
    jq \
    ripgrep \
    fd-find \
    bat \
    exa \
    httpie \
    # Network tools for Claude Code dangerous mode
    iptables \
    ipset \
    iproute2 \
    dnsutils \
    net-tools \
    curl \
    wget \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install pnpm globally (Claude Code will be installed later if needed)
RUN npm install -g pnpm@latest

# Rust will be installed by devcontainer features
# Skip manual Rust installation to speed up builds

# Create workspace directory and set as default for shells
RUN mkdir -p /workspace && chown -R vscode:vscode /workspace && \
    echo 'cd /workspace 2>/dev/null || true' >> /etc/bash.bashrc && \
    echo 'cd /workspace 2>/dev/null || true' >> /etc/zsh/zshrc

# Install tmux plugins (TPM)
USER vscode
RUN git clone https://github.com/tmux-plugins/tpm /home/vscode/.tmux/plugins/tpm

# Copy tmux configuration
COPY --chown=vscode:vscode tmux.conf /home/vscode/.tmux.conf

# Set up pnpm and workspace defaults
RUN SHELL=/bin/bash pnpm setup && \
    echo 'export PNPM_HOME="/home/vscode/.local/share/pnpm"' >> /home/vscode/.bashrc && \
    echo 'export PATH="$PNPM_HOME:$PATH"' >> /home/vscode/.bashrc && \
    echo 'export WORKSPACE_DIR="/workspace"' >> /home/vscode/.bashrc && \
    echo 'export PNPM_HOME="/home/vscode/.local/share/pnpm"' >> /home/vscode/.zshrc && \
    echo 'export PATH="$PNPM_HOME:$PATH"' >> /home/vscode/.zshrc && \
    echo 'export WORKSPACE_DIR="/workspace"' >> /home/vscode/.zshrc

# Pre-create directories for better caching (including Cursor support)
RUN mkdir -p /home/vscode/.cargo \
    /home/vscode/.pnpm-store \
    /home/vscode/.cache \
    /home/vscode/.local/share \
    /home/vscode/.claude \
    /home/vscode/.cursor-server \
    /home/vscode/.vscode-server

# Copy and set up firewall script for Claude Code dangerous mode
COPY --chown=root:root scripts/init-claude-firewall.sh /usr/local/bin/
USER root
RUN chmod +x /usr/local/bin/init-claude-firewall.sh && \
    echo "vscode ALL=(root) NOPASSWD: /usr/local/bin/init-claude-firewall.sh" > /etc/sudoers.d/vscode-firewall && \
    chmod 0440 /etc/sudoers.d/vscode-firewall

# Create Claude Code configuration directory
RUN mkdir -p /home/vscode/.claude && chown -R vscode:vscode /home/vscode/.claude

# Set working directory
USER vscode
WORKDIR /workspace

# Keep container running
CMD ["sleep", "infinity"]