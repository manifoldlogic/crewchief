# .devcontainer

## Lifecycle Scripts

- **post-create.sh** — Runs once on first build (installs pnpm, deps, CrewChief CLI, Oh My Zsh)
- **post-start.sh** — Every container start (syncs configs, reinitializes firewall)
- **post-attach.sh** — Editor attach (shows git status)

## Pitfalls

- **post-create runs once only**: If you need to re-run setup, rebuild the container (`F1 → "Dev Containers: Rebuild Container"`)
- **CLAUDE_DANGEROUS_MODE + firewall**: Container has `CLAUDE_DANGEROUS_MODE=true` but internet access is restricted by `init-claude-firewall.sh`. Host machine access is blocked.
- **ZSH target**: All shell scripts must use POSIX-compatible syntax (no bash arrays, no `[[ ]]`). Shell is ZSH via Oh My Zsh.

## Troubleshooting

See `TROUBLESHOOTING.md` in this directory for build failures, port conflicts, and volume permissions.
