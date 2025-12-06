#!/bin/bash

# CrewChief devcontainer Bash configuration
# This file is sourced by interactive shells inside the devcontainer.
# Edit this file within the repository (.devcontainer/.bashrc) to change
# the shell experience for future sessions.

# Environment setup ---------------------------------------------------------
export PNPM_HOME="/home/vscode/.local/share/pnpm"
export PATH="$PNPM_HOME:$PATH"
export WORKSPACE_DIR="/workspace"

# Ensure pnpm home exists to avoid PATH entries for missing directories.
if [ ! -d "$PNPM_HOME" ]; then
  mkdir -p "$PNPM_HOME"
fi

# Shell customisation -------------------------------------------------------
# Only run interactive customisations for interactive shells.
case $- in
  *i*)
    # Always start interactive shells in the workspace directory.
    if [ -d "$WORKSPACE_DIR" ] && [ "$PWD" != "$WORKSPACE_DIR" ]; then
      cd "$WORKSPACE_DIR"
    fi

    # Helpful aliases for the CrewChief project.
    alias cc='node /workspace/packages/cli/dist/cli/index.js'
    alias ccdev='tsx /workspace/packages/cli/src/cli/index.ts'
    alias ll='ls -la'
    alias gs='git status'
    alias gd='git diff'
    alias gcm='git commit -m'
    alias gco='git checkout'
    alias gcob='git checkout -b'
    alias gp='git push'
    alias gl='git log --oneline --graph --decorate'
    alias ccwt='crewchief worktree'
    alias ccmp='crewchief maproom'

    alias dps='docker ps'
    alias dlog='docker logs -f'
    alias dexec='docker exec -it'

    alias ..='cd ..'
    alias ...='cd ../..'
    alias ....='cd ../../..'
  ;;
esac

# Docker-in-Docker workspace path (auto-detected on container start)
# This is set by post-start.sh and used by maproom-mcp docker-compose volume mount
if [ -z "$WORKSPACE_HOST_PATH" ]; then
  export WORKSPACE_HOST_PATH=$(docker inspect $(hostname) --format '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}' 2>/dev/null || echo "/workspace")
fi

# End of CrewChief Bash configuration
