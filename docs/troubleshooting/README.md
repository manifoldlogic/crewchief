# Troubleshooting

Guides for diagnosing and resolving Maproom issues.

## Quick Links

- **[Common Errors](common-errors.md)** - Solutions for frequent issues
  - Ollama connection errors
  - SQLite database errors
  - Search problems
  - Daemon crashes

- **[Debugging Guide](debugging.md)** - Diagnostic procedures
  - Enable debug logging
  - Database inspection
  - Process management
  - Performance profiling

## Quick Diagnostic

When something doesn't work, check in this order:

### 1. Is Ollama running?
```bash
curl http://localhost:11434/api/tags
```
If not: `ollama serve`

### 2. Is the model available?
```bash
ollama list | grep mxbai
```
If not: `ollama pull mxbai-embed-large`

### 3. Does the database exist?
```bash
ls -la ~/.maproom/maproom.db
```
If not: `maproom scan /path/to/repo`

### 4. Is anything indexed?
Use the `status` MCP tool to check repos and chunk counts.

### 5. Enable debug logging
```bash
RUST_LOG=debug maproom serve
```

## Common Quick Fixes

| Problem | Quick Fix |
|---------|-----------|
| Search returns nothing | Run `scan` first |
| Ollama errors | `ollama serve && ollama pull mxbai-embed-large` |
| Database locked | Wait or `pkill -f maproom` |
| Stale results | `scan` to re-index |
| Daemon won't start | Check `RUST_LOG=debug` output |

## Getting Help

If issues persist:
1. Enable `RUST_LOG=debug`
2. Collect relevant logs
3. Check database integrity: `sqlite3 ~/.maproom/maproom.db "PRAGMA integrity_check"`
4. Report at: https://github.com/anthropics/claude-code/issues
