FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

# Install system dependencies
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
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install pnpm globally
RUN npm install -g pnpm@latest

# Install Rust toolchain (will be overridden by devcontainer features, but good as backup)
USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/home/vscode/.cargo/bin:${PATH}"

# Install useful Rust tools
RUN cargo install cargo-watch cargo-edit cargo-expand cargo-nextest

# Switch back to root for remaining setup
USER root

# Create workspace directory
RUN mkdir -p /workspace && chown -R vscode:vscode /workspace

# Install tmux plugins (TPM)
USER vscode
RUN git clone https://github.com/tmux-plugins/tpm /home/vscode/.tmux/plugins/tpm

# Copy tmux configuration
COPY --chown=vscode:vscode tmux.conf /home/vscode/.tmux.conf

# Set up pnpm
RUN pnpm setup && \
    echo 'export PNPM_HOME="/home/vscode/.local/share/pnpm"' >> /home/vscode/.bashrc && \
    echo 'export PATH="$PNPM_HOME:$PATH"' >> /home/vscode/.bashrc && \
    echo 'export PNPM_HOME="/home/vscode/.local/share/pnpm"' >> /home/vscode/.zshrc && \
    echo 'export PATH="$PNPM_HOME:$PATH"' >> /home/vscode/.zshrc

# Pre-create directories for better caching
RUN mkdir -p /home/vscode/.cargo \
    /home/vscode/.pnpm-store \
    /home/vscode/.cache \
    /home/vscode/.local/share

# Set working directory
WORKDIR /workspace

# Keep container running
CMD ["sleep", "infinity"]