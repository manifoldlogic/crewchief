# Automatic Branch Detection and Indexing

## Overview

The maproom `watch` command monitors your git repository for file changes and automatically keeps your code search index synchronized. The watch command auto-detects your current branch at startup and indexes files to the appropriate worktree.

**Key Benefits**:
- **Zero-friction workflow**: Just run `maproom watch` - branch is auto-detected
- **Efficient updates**: Only processes changed files (incremental indexing)
- **Always current**: Search results reflect your current branch state
- **Resource efficient**: <5% CPU and <20MB RAM while idle

> **Note**: Runtime branch switch detection (detecting when you `git checkout` while watch is running) is planned in the UNIWATCH project but not yet implemented. Currently, if you switch branches, you need to restart the watch command.

## Prerequisites

Before using automatic indexing:

1. **SQLite Database**: Maproom uses SQLite by default (no setup required)
   - Database auto-created at `~/.maproom/maproom.db`
   - Override with: `MAPROOM_DATABASE_URL="sqlite:///path/to/db"`

2. **Git Repository**: Your project must be a git repository with `.git/` directory

## Quick Start

Start watching your repository:

```bash
cd /path/to/your/project
maproom watch
```

**Expected Output**:
```
[INFO] Auto-detected branch: main
[INFO] Starting watch for /path/to/your/project
[INFO] Watching for file changes...
```

When you edit files, indexing happens automatically:

```bash
# Edit a file
vim src/auth.rs
```

**Watcher Output**:
```
[INFO] File changed: src/auth.rs
[INFO] Index updated: 1 file, 15 chunks
```

**Stop the Watcher**:
Press `Ctrl+C` for graceful shutdown:
```
^C[INFO] Shutting down...
[INFO] Watch stopped
```

## Usage

### Command Syntax

```bash
maproom watch [OPTIONS]
```

**Options**:
- `--repo <REPO>` - Repository name (auto-detected if not provided)
- `--path <PATH>` - Path to watch (defaults to current directory)
- `--throttle <MS>` - Throttle duration in milliseconds (default: 100)
- `--help` - Show help information

### Basic Usage

**Watch current directory**:
```bash
maproom watch
```

**Watch specific path**:
```bash
maproom watch --path /workspace/myproject
```

**Specify repository name**:
```bash
maproom watch --repo myproject
```

### Workflow Examples

#### Solo Developer Workflow

Working on a feature branch:

```bash
# Start watcher on your current branch
maproom watch

# In another terminal: Normal development
# ... edit files ...
git commit -m "Add authentication"

# File changes are automatically indexed
```

> **Note**: When you need to switch branches, stop the watcher (`Ctrl+C`) and restart it on the new branch. Runtime branch switch detection is planned for future release.

#### Team Collaboration

Staying synchronized with team changes:

```bash
# Start watcher
maproom watch

# Pull latest changes (file changes auto-indexed)
git pull origin main

# Edit files as usual
# Changes indexed automatically
```

#### Feature Branch Development

Working on a feature branch:

```bash
# Switch to your feature branch
git checkout feature-auth

# Start watcher (auto-detects current branch)
maproom watch

# Edit files - all changes indexed to feature-auth worktree
vim src/auth.rs
vim tests/auth_test.rs

# Search results reflect your feature branch
```

## How It Works

### File Watching

The watcher monitors your repository for file changes using OS-native file system events:
- **Linux**: inotify
- **macOS**: FSEvents
- **Windows**: ReadDirectoryChangesW

When files change, the watcher detects the modification and queues the file for incremental indexing.

### Branch Detection at Startup

When you start the watch command:

1. **Parse branch name** from `.git/HEAD`:
   - Branch reference: `ref: refs/heads/main` → `"main"`
   - Detached HEAD: `abc123def...` → `"abc123de"` (short SHA)

2. **Get or create worktree record** in database for this branch

3. **Start watching** for file changes in the repository

### Incremental Updates

When files change:

1. **Detect file change** via OS file system events
2. **Queue for processing** with throttling to batch rapid changes
3. **Incremental update** processes only changed files:
   - Parses changed files with tree-sitter
   - Generates chunks from code structures
   - Creates embeddings for new/modified content
   - Stores to SQLite database

### Debouncing

To prevent rapid successive indexing operations, the watcher implements time-based debouncing:

- **Throttle window**: Configurable (default 100ms)
- **Behavior**: Rapid file changes are batched together

This prevents issues with:
- Multiple rapid file saves
- Build processes that modify many files
- File system noise (duplicate events)

## Performance

### Detection Latency

**Target**: <500ms from file save to detection

**Typical Performance**:
- Linux (inotify): ~50-100ms
- macOS (FSEvents): ~100-200ms
- Windows (ReadDirectoryChanges): ~100-300ms

### Update Time

**Depends on**:
- Size of changed files
- Number of chunks generated
- Embedding provider latency

**Typical Scenarios**:

| Scenario | Files Changed | Update Time |
|----------|--------------|-------------|
| Single file edit | 1 | <5s |
| Small feature | 5-10 | 10-30s |
| Medium feature | 50-100 | 1-2 min |

### Resource Usage

**While Idle** (waiting for file changes):
- **CPU**: <5% (mostly sleeping)
- **Memory**: ~15-20 MB
- **I/O**: Minimal (OS file watching)

**During Indexing**:
- **CPU**: 50-80% (embedding generation, parsing)
- **Memory**: 50-100 MB (buffers, tree-sitter parsers)
- **Network**: Varies by embedding provider (Ollama: local, OpenAI: remote)

## Troubleshooting

### Watcher Not Detecting File Changes

**Symptoms**: You edit files but the watcher shows no activity

**Possible Causes**:
1. Watcher not running
2. Watching wrong repository path
3. File not in watched directory

**Solutions**:

```bash
# 1. Verify watcher is running
ps aux | grep "maproom watch"

# 2. Check you're in the right repository
pwd
ls -la .git

# 3. Restart the watcher
maproom watch
```

### High CPU Usage

**Symptoms**: `maproom watch` consuming >10% CPU while idle

**Expected**: <5% CPU while waiting for changes

**Possible Causes**:
1. Indexing operation in progress (normal)
2. Many files changing rapidly
3. Error causing retry loop

**Solutions**:

```bash
# Monitor CPU usage over time
top -p $(pgrep -f "maproom watch")

# Check if indexing is in progress
# Look for indexing log messages

# If needed, increase throttle
maproom watch --throttle 500
```

### Database Errors

**Symptoms**:
```
[ERROR] Database error: unable to open database file
```

**Solutions**:

```bash
# 1. Check database location (default: ~/.maproom/maproom.db)
ls -la ~/.maproom/

# 2. Override database location if needed
export MAPROOM_DATABASE_URL="sqlite:///path/to/maproom.db"

# 3. Ensure directory exists and is writable
mkdir -p ~/.maproom
```

### Watcher Crashes or Exits Unexpectedly

**Symptoms**: Watcher process terminates without `Ctrl+C`

**Solutions**:

```bash
# Run with full logging to capture crash details
RUST_BACKTRACE=1 RUST_LOG=debug maproom watch

# Check system logs for OOM killer
dmesg | grep -i "killed process"
```

### Logs and Debugging

**Enable different log levels**:

```bash
# Info level (default)
maproom watch

# Debug level (verbose events)
RUST_LOG=debug maproom watch

# Trace level (very verbose, includes library logs)
RUST_LOG=trace maproom watch
```

**Interpret log messages**:

| Log Level | Meaning | Example |
|-----------|---------|---------|
| `[INFO]` | Normal operation | `File changed: src/main.rs` |
| `[WARN]` | Recoverable issue | `Skipping binary file` |
| `[ERROR]` | Serious problem | `Failed to parse file` |
| `[DEBUG]` | Diagnostic info | `Debouncing event` |

## Configuration

### Environment Variables

**Optional**:
- `MAPROOM_DATABASE_URL`: Database URL (default: SQLite at `~/.maproom/maproom.db`). Accepts a `sqlite://` path or, in a `--features postgres` build, a `postgres://`/`postgresql://` URL to use the PostgreSQL/pgvector backend.
  ```bash
  export MAPROOM_DATABASE_URL="sqlite:///custom/path/maproom.db"
  ```

- `RUST_LOG`: Logging level (`error`, `warn`, `info`, `debug`, `trace`)
  ```bash
  export RUST_LOG=debug
  ```

- `RUST_BACKTRACE`: Enable backtraces on errors
  ```bash
  export RUST_BACKTRACE=1
  ```

## Related Documentation

### Architecture and Implementation

- [Database Architecture](../architecture/DATABASE_ARCHITECTURE.md) - Schema design and worktree tracking
- [Maproom Architecture](../architecture/MAPROOM_ARCHITECTURE.md) - Overall indexer architecture

### Related Features

- **Incremental Updates**: The watcher uses incremental update system to efficiently process only changed files
- **Content-Addressed Storage**: Unchanged content across branches shares embeddings for efficiency

## Technical Details

### File Watcher Implementation

The watcher uses the [notify](https://crates.io/crates/notify) crate with OS-native backends:

```rust
use notify::{Watcher, RecursiveMode, recommended_watcher};

let (tx, rx) = channel();
let mut watcher = recommended_watcher(tx)?;
watcher.watch(&path, RecursiveMode::Recursive)?;
```

**Event Types Processed**:
- `Modify`: File content changed
- `Create`: New file created
- `Remove`: File deleted

### Graceful Shutdown

Uses `tokio::select!` to race watcher against shutdown signal:

```rust
tokio::select! {
    event = event_rx.recv() => {
        // Process file change event
    }
    _ = shutdown_rx => {
        // Ctrl+C received, shutdown gracefully
    }
}
```

Ensures:
- Current indexing operation completes
- Database connections closed properly
- File watcher resources released
- Exit code 0 for normal shutdown

---

**Last Updated**: 2025-11-28
**Related Project**: UNIWATCH (Unified Watch Command)
