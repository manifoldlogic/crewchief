# Automatic Branch Switch Detection

## Overview

The automatic branch switch detection feature monitors your git repository for branch switches and automatically triggers incremental indexing. This keeps your code search index synchronized with your current branch without manual intervention.

**Key Benefits**:
- **Zero-friction workflow**: Index updates happen automatically in the background
- **Fast detection**: Branch switches detected in <1 second
- **Efficient updates**: Only processes changed files (5-10x faster than full scan)
- **Always current**: Search results always reflect your current branch state
- **Resource efficient**: <5% CPU and <20MB RAM while idle

## Prerequisites

Before using automatic branch switch detection:

1. **BRANCHX Implementation**: The branch-aware indexing system must be deployed
   - See [Branch-Aware Indexing Architecture](../architecture/branch-aware-indexing.md)

2. **PostgreSQL Database**: A configured maproom database instance
   - Database must have pgvector extension installed
   - Schema migrations applied

3. **Environment Setup**:
   ```bash
   export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"
   ```

4. **Git Repository**: Your project must be a git repository with `.git/` directory

## Quick Start

Start the branch watcher in your repository:

```bash
cd /path/to/your/project
maproom branch-watch --repo .
```

**Expected Output**:
```
[INFO] Starting branch watcher for /path/to/your/project
[INFO] Connected to database: postgresql://maproom:maproom@localhost:5432/maproom
[INFO] Watching /path/to/your/project/.git/HEAD for branch switches
[INFO] Indexing current branch: main
[INFO] Index updated in 0.1s: 0 files, 0 chunks, 0 embeddings
[INFO]   Cache hit rate: 100.0%
[INFO]   Estimated cost: $0.0000
[INFO] Waiting for changes...
```

Now when you switch branches, indexing happens automatically:

```bash
# In another terminal
git checkout feature-auth
```

**Watcher Output**:
```
[INFO] Branch switch detected: feature-auth
[INFO] Index updated in 45.2s: 150 files, 7500 chunks, 1185 embeddings
[INFO]   Cache hit rate: 84.2%
[INFO]   Estimated cost: $0.0237
[INFO] Waiting for changes...
```

**Stop the Watcher**:
Press `Ctrl+C` for graceful shutdown:
```
^C[INFO] Shutting down...
[INFO] Shutdown signal received
[INFO] Branch watcher stopped
```

## Usage

### Command Syntax

```bash
maproom branch-watch [OPTIONS]
```

**Options**:
- `--repo <REPO>` - Path to git repository (defaults to current directory)
- `--verbose` - Enable verbose logging for debugging
- `--help` - Show help information

### Basic Usage

**Watch current directory**:
```bash
maproom branch-watch
```

**Watch specific repository**:
```bash
maproom branch-watch --repo /workspace/myproject
```

**Enable verbose logging**:
```bash
maproom branch-watch --verbose
```

With verbose mode, you'll see additional debug information:
```
[DEBUG] Debouncing event (too soon after previous)
[DEBUG] Processing branch switch event
[DEBUG] Checking for file changes...
```

### Workflow Examples

#### Solo Developer Workflow

Working on a feature branch:

```bash
# Terminal 1: Start watcher
maproom branch-watch --repo .

# Terminal 2: Normal development
git checkout -b feature-auth
# ... edit files ...
git commit -m "Add authentication"

# Switch back to main
git checkout main
# Watcher automatically indexes main branch

# Switch to another feature
git checkout feature-ui
# Watcher automatically indexes feature-ui branch
```

The watcher runs continuously in the background, indexing each branch as you switch.

#### Team Collaboration

Staying synchronized with team changes:

```bash
# Terminal 1: Start watcher
maproom branch-watch

# Terminal 2: Pull latest changes
git fetch origin
git checkout main
git pull origin main
# Watcher detects branch switch, indexes updated main

git checkout feature-123
# Watcher indexes your feature branch

# Later: Rebase on latest main
git checkout main
git pull origin main
# Watcher re-indexes main with new commits

git checkout feature-123
git rebase main
# Watcher re-indexes feature-123 with rebased changes
```

#### Feature Branch Development

Working on multiple features:

```bash
# Start watcher once
maproom branch-watch --repo /workspace/myapp

# Switch between features freely
git checkout feature-auth      # Auto-indexed
git checkout feature-payments  # Auto-indexed
git checkout feature-ui        # Auto-indexed
git checkout main             # Auto-indexed

# Each branch is indexed automatically
# Search results always match your current branch
```

## How It Works

### File Watching

The watcher monitors `.git/HEAD` using OS-native file system events:
- **Linux**: inotify
- **macOS**: FSEvents
- **Windows**: ReadDirectoryChangesW

When you run `git checkout`, git updates `.git/HEAD` to point to the new branch. The watcher detects this file modification instantly (<100ms).

### Change Detection

After detecting a branch switch:

1. **Parse branch name** from `.git/HEAD`:
   - Branch reference: `ref: refs/heads/main` → `"main"`
   - Detached HEAD: `abc123def...` → `"abc123de"` (short SHA)

2. **Get or create worktree record** in database for this branch

3. **Compare git tree SHAs** between database and current commit:
   - If tree SHAs match: Skip indexing (no changes)
   - If tree SHAs differ: Proceed with incremental update

4. **Incremental update** processes only changed files:
   - Uses `git diff-tree` to find modified/added/deleted files
   - Reuses existing chunks and embeddings for unchanged content
   - Generates new embeddings only for new/modified content

See [Branch-Aware Indexing Architecture](../architecture/branch-aware-indexing.md) for technical details.

### Debouncing

To prevent rapid successive indexing operations, the watcher implements time-based debouncing:

- **Debounce window**: 2 seconds
- **Behavior**: Events within 2 seconds of the previous event are ignored

This prevents issues with:
- Multiple rapid branch switches
- Git operations that modify `.git/HEAD` multiple times
- File system noise (duplicate events)

### Error Handling and Retries

The watcher implements exponential backoff retry logic:

**Retry Strategy**:
- **Max retries**: 3 attempts
- **Backoff timing**: 2s, 4s, 8s
- **Retryable errors**: Database connection issues, temporary I/O errors
- **Non-retryable errors**: Invalid data, logic errors

**Example Retry Sequence**:
```
[WARN] Branch switch failed (attempt 1/3): connection refused
[WARN] Retrying in 2s...
[WARN] Branch switch failed (attempt 2/3): connection refused
[WARN] Retrying in 4s...
[INFO] Branch switch detected: feature-auth
[INFO] Index updated successfully
```

If all retries fail, the error is logged and the watcher continues monitoring (doesn't crash).

## Performance

### Detection Latency

**Target**: <1 second from branch switch to detection

**Measurement**: Time between `git checkout` command and watcher detecting the change

**Typical Performance**:
- Linux (inotify): ~50-100ms
- macOS (FSEvents): ~100-200ms
- Windows (ReadDirectoryChanges): ~100-300ms

### Update Time

**Depends on**:
- Number of changed files
- Cache hit rate (unchanged content)
- Embedding provider latency

**Typical Scenarios**:

| Scenario | Files Changed | Update Time | Cache Hit Rate |
|----------|--------------|-------------|----------------|
| No changes | 0 | <100ms | 100% |
| Small feature | 5-10 | 5-15s | 95% |
| Medium feature | 50-100 | 30-60s | 85% |
| Large refactor | 500+ | 3-5 min | 70% |

### Resource Usage

**While Idle** (waiting for branch switches):
- **CPU**: <5% (mostly sleeping)
- **Memory**: ~15-20 MB
- **I/O**: Minimal (OS file watching)

**During Indexing**:
- **CPU**: 50-80% (embedding generation, parsing)
- **Memory**: 50-100 MB (buffers, tree-sitter parsers)
- **Network**: Varies by embedding provider (Ollama: local, OpenAI: remote)

## Troubleshooting

### Watcher Not Detecting Branch Switches

**Symptoms**: You run `git checkout` but the watcher shows no activity

**Possible Causes**:
1. Watcher not running
2. Watching wrong repository path
3. `.git/HEAD` file watcher not initialized
4. Git worktree or submodule configuration

**Solutions**:

```bash
# 1. Verify watcher is running
ps aux | grep "maproom branch-watch"

# 2. Check you're in the right repository
pwd
ls -la .git/HEAD

# 3. Enable verbose mode to see debug logs
maproom branch-watch --verbose

# 4. Verify .git/HEAD exists and is a file (not symlink)
file .git/HEAD
cat .git/HEAD
```

**Expected `.git/HEAD` content**:
```
ref: refs/heads/main
```

### Watcher Detects Switch But Doesn't Index

**Symptoms**: Watcher logs "Branch switch detected" but no indexing happens

**Possible Causes**:
1. Database connection failure
2. No file changes (tree SHA matches)
3. Error during incremental update

**Solutions**:

```bash
# Check database connection
psql $MAPROOM_DATABASE_URL -c "SELECT 1"

# Verify MAPROOM_DATABASE_URL is set
echo $MAPROOM_DATABASE_URL

# Check watcher logs for errors
maproom branch-watch --verbose 2>&1 | grep ERROR

# Manually test incremental update
maproom upsert --repo . --worktree main
```

### High CPU Usage

**Symptoms**: `maproom branch-watch` consuming >10% CPU while idle

**Expected**: <5% CPU while waiting for changes

**Possible Causes**:
1. Indexing operation in progress (normal)
2. Rapid branch switching causing continuous indexing
3. Error retry loop

**Solutions**:

```bash
# Monitor CPU usage over time
top -p $(pgrep -f "maproom branch-watch")

# Check if indexing is in progress
# Look for "Index updated" log messages

# Check for error retry loops
maproom branch-watch --verbose 2>&1 | grep -E "ERROR|Retrying"

# If stuck in retry loop, fix the underlying issue
# (usually database connection or permissions)
```

### Database Connection Errors

**Symptoms**:
```
[ERROR] Failed to connect to database: connection refused
[WARN] Retrying in 2s...
```

**Possible Causes**:
1. MAPROOM_DATABASE_URL not set or incorrect
2. PostgreSQL not running
3. Wrong credentials or database name
4. Network/firewall issues

**Solutions**:

```bash
# 1. Verify MAPROOM_DATABASE_URL is set
echo $MAPROOM_DATABASE_URL
# Should output: postgresql://user:pass@host:port/dbname

# 2. Test database connection directly
psql $MAPROOM_DATABASE_URL -c "SELECT version()"

# 3. Check PostgreSQL is running
# Docker:
docker ps | grep postgres
# System service:
sudo systemctl status postgresql

# 4. Verify credentials
psql -U maproom -h localhost -d maproom -c "SELECT 1"

# 5. Set MAPROOM_DATABASE_URL if missing
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"

# 6. Add to shell profile for persistence
echo 'export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"' >> ~/.bashrc
source ~/.bashrc
```

### Permission Denied Reading .git/HEAD

**Symptoms**:
```
[ERROR] Permission denied: .git/HEAD
```

**Possible Causes**:
1. File permissions too restrictive
2. Parent directory not accessible
3. Running as different user than repository owner

**Solutions**:

```bash
# Check .git/HEAD permissions
ls -la .git/HEAD
# Should be readable: -rw-r--r-- or similar

# Check .git directory permissions
ls -ld .git
# Should be accessible: drwxr-xr-x or similar

# Fix permissions if needed (as repository owner)
chmod 644 .git/HEAD
chmod 755 .git

# Verify you can read the file
cat .git/HEAD
```

### Watcher Crashes or Exits Unexpectedly

**Symptoms**: Watcher process terminates without `Ctrl+C`

**Possible Causes**:
1. Unhandled error (bug)
2. System signal (SIGTERM, SIGKILL)
3. Out of memory

**Solutions**:

```bash
# Run with full logging to capture crash details
RUST_BACKTRACE=1 RUST_LOG=debug maproom branch-watch --verbose

# Check system logs for OOM killer
dmesg | grep -i "killed process"
sudo journalctl -u maproom --since "1 hour ago"

# Monitor memory usage
while true; do ps aux | grep "maproom branch-watch"; sleep 5; done

# Run in a process manager (auto-restart on crash)
# Using systemd, supervisor, or pm2
```

### Logs and Debugging

**Enable different log levels**:

```bash
# Info level (default)
maproom branch-watch

# Debug level (verbose events)
RUST_LOG=debug maproom branch-watch

# Trace level (very verbose, includes library logs)
RUST_LOG=trace maproom branch-watch

# Component-specific logging
RUST_LOG=maproom::watcher=debug maproom branch-watch
```

**Log Output Locations**:
- **stdout**: Info, warn, error messages
- **stderr**: Error messages and backtraces

**Redirect to file**:
```bash
maproom branch-watch --verbose > watcher.log 2>&1

# Watch in real-time
tail -f watcher.log
```

**Interpret log messages**:

| Log Level | Meaning | Example |
|-----------|---------|---------|
| `[INFO]` | Normal operation | `Branch switch detected: main` |
| `[WARN]` | Recoverable issue | `Retrying in 2s...` |
| `[ERROR]` | Serious problem | `Failed to connect to database` |
| `[DEBUG]` | Diagnostic info | `Debouncing event` |

## Configuration

### Environment Variables

**Required**:
- `MAPROOM_DATABASE_URL`: PostgreSQL connection string
  ```bash
  export MAPROOM_DATABASE_URL="postgresql://user:pass@host:port/dbname"
  ```

**Optional**:
- `RUST_LOG`: Logging level (`error`, `warn`, `info`, `debug`, `trace`)
  ```bash
  export RUST_LOG=debug
  ```

- `RUST_BACKTRACE`: Enable backtraces on errors
  ```bash
  export RUST_BACKTRACE=1
  ```

### Running as a Service

For long-term usage, run the watcher as a background service:

#### Using systemd (Linux)

Create `/etc/systemd/system/maproom-watcher.service`:

```ini
[Unit]
Description=Maproom Branch Watcher
After=network.target postgresql.service

[Service]
Type=simple
User=developer
WorkingDirectory=/workspace/myproject
Environment="MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/maproom branch-watch --repo /workspace/myproject
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

**Enable and start**:
```bash
sudo systemctl enable maproom-watcher
sudo systemctl start maproom-watcher
sudo systemctl status maproom-watcher
```

#### Using Docker

Run watcher in a container:

```dockerfile
FROM rust:1.75-slim
RUN cargo install crewchief-maproom
WORKDIR /workspace
CMD ["maproom", "branch-watch", "--repo", "/workspace"]
```

```bash
docker run -d \
  --name maproom-watcher \
  -v /workspace/myproject:/workspace \
  -e MAPROOM_DATABASE_URL="postgresql://maproom:maproom@postgres:5432/maproom" \
  --network maproom-net \
  maproom-watcher
```

## Related Documentation

### Architecture and Implementation

- [Branch-Aware Indexing Architecture](../architecture/branch-aware-indexing.md) - Overview of BRANCHX system
- [Database Architecture](../architecture/DATABASE_ARCHITECTURE.md) - Schema design and worktree tracking
- [Maproom Architecture](../architecture/MAPROOM_ARCHITECTURE.md) - Overall indexer architecture

### Planning Documents

For implementation details and design decisions, see:
- [BRWATCH Planning: Analysis](./.agents/projects/BRWATCH_branch-switch-detection/planning/analysis.md)
- [BRWATCH Planning: Architecture](./.agents/projects/BRWATCH_branch-switch-detection/planning/architecture.md)
- [BRWATCH Planning: Implementation Plan](./.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md)

### Related Features

- **Incremental Updates**: The watcher uses incremental update system to efficiently process only changed files
- **Content-Addressed Storage**: Unchanged content across branches shares embeddings for efficiency
- **Tree SHA Optimization**: Git tree comparison enables instant "no changes" detection

## Technical Details

### File Watcher Implementation

The watcher uses the [notify](https://crates.io/crates/notify) crate with OS-native backends:

```rust
use notify::{Watcher, RecursiveMode, recommended_watcher};

let (tx, rx) = channel();
let mut watcher = recommended_watcher(tx)?;
watcher.watch(&git_head_path, RecursiveMode::NonRecursive)?;
```

**Event Types Processed**:
- `Modify`: Content of `.git/HEAD` changed
- `Create`: `.git/HEAD` created (rare, but handled)

**Non-Recursive Mode**: Only watches `.git/HEAD` file itself, not entire `.git/` directory (more efficient).

### Graceful Shutdown

Uses `tokio::select!` to race watcher against shutdown signal:

```rust
tokio::select! {
    result = watcher.start() => {
        // Watcher exited naturally (error or channel closed)
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

### Detection Flow

```
1. User runs: git checkout feature-auth
2. Git updates: .git/HEAD → "ref: refs/heads/feature-auth"
3. OS notifies: File change event
4. Watcher receives: Event via channel
5. Debouncer checks: >2s since last event?
6. Parser extracts: "feature-auth" from HEAD
7. Database queries: Get/create worktree record
8. Git compares: Tree SHA changed?
9. Indexer runs: incremental_update() on changed files
10. Logger reports: "Index updated in 45s: 150 files..."
```

**Total latency**: Typically <1 second from step 1 to step 5.

---

**Last Updated**: 2025-11-09
**BRWATCH Version**: 1.0.0
**Related Tickets**: BRWATCH-4001
