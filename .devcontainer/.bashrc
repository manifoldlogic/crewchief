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
    alias maproom='crewchief-maproom'
    alias claude='claude --dangerous-mode'
    alias ll='ls -la'
    alias gs='git status'
    alias gd='git diff'
    alias gc='git commit'
    alias gp='git push'
    alias gl='git log --oneline --graph --decorate'

    alias dps='docker ps'
    alias dlog='docker logs -f'
    alias dexec='docker exec -it'

    alias ta='tmux attach -t'
    alias tl='tmux list-sessions'
    alias tn='tmux new -s'

    alias ..='cd ..'
    alias ...='cd ../..'
    alias ....='cd ../../..'
  ;;
esac

# End of CrewChief Bash configuration

