# Administrator Guide: Stale Worktree Cleanup

This guide covers administration, automation, and best practices for the Maproom stale worktree cleanup feature.

## Overview

The stale worktree cleanup feature removes database entries for git worktrees that no longer exist on disk. This improves search quality by eliminating duplicate results from deleted branches.

### Current Status

| Feature | Status | Notes |
|---------|--------|-------|
| Manual CLI cleanup | ✅ Available | `db cleanup-stale` command |
| Dry-run mode | ✅ Available | Default behavior (safe) |
| Verbose output | ✅ Available | `--verbose` flag |
| Watch integration | ⏳ Planned | Pending watch command reimplementation |
| Automatic scheduling | ⏳ Planned | Pending watch integration |

## CLI Administration

### Running Manual Cleanup

```bash
# Preview stale worktrees (safe - no changes)
maproom db cleanup-stale --verbose

# Execute cleanup
maproom db cleanup-stale --confirm --verbose
```

### Scheduling with Cron

Until automatic watch integration is available, schedule periodic cleanup with cron:

```bash
# Edit crontab
crontab -e
```

Add a weekly cleanup job:
```cron
# Run stale worktree cleanup every Sunday at 2 AM
0 2 * * 0 /path/to/maproom db cleanup-stale --confirm >> /var/log/maproom-cleanup.log 2>&1
```

For production systems, use a wrapper script:

```bash
#!/bin/bash
# /usr/local/bin/maproom-cleanup.sh

LOG_FILE="/var/log/maproom/cleanup-$(date +%Y%m%d).log"
MAPROOM_BIN="/path/to/maproom"

echo "=== Cleanup started: $(date) ===" >> "$LOG_FILE"

# Run cleanup
"$MAPROOM_BIN" db cleanup-stale --confirm --verbose >> "$LOG_FILE" 2>&1
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Cleanup successful" >> "$LOG_FILE"
elif [ $EXIT_CODE -eq 2 ]; then
    echo "✅ No stale worktrees found" >> "$LOG_FILE"
else
    echo "❌ Cleanup failed with exit code $EXIT_CODE" >> "$LOG_FILE"
    # Optionally send alert
    # mail -s "Maproom cleanup failed" admin@example.com < "$LOG_FILE"
fi

echo "=== Cleanup finished: $(date) ===" >> "$LOG_FILE"
```

### Exit Codes for Automation

| Code | Meaning | Automation Action |
|------|---------|-------------------|
| `0` | Success | Continue normally |
| `1` | Error | Alert/retry |
| `2` | No stale worktrees | Continue (not an error) |

Example in shell scripts:
```bash
maproom db cleanup-stale --confirm
case $? in
    0) echo "Cleanup completed successfully" ;;
    1) echo "ERROR: Cleanup failed" ; exit 1 ;;
    2) echo "No cleanup needed" ;;
esac
```

## Database Configuration

### Default Location

```bash
~/.maproom/maproom.db
```

### Custom Database Path

Set via environment variable:
```bash
export MAPROOM_DATABASE_URL="sqlite:///path/to/custom.db"
maproom db cleanup-stale --confirm
```

### Multi-User Environments

SQLite databases are single-writer. In multi-user environments:

1. **Shared database**: Coordinate cleanup schedules to avoid write conflicts
2. **Per-user databases**: Each user manages their own cleanup
3. **Central service**: Use a single cleanup service with exclusive write access

## Monitoring and Logging

### Log Levels

Control verbosity with `RUST_LOG`:

```bash
# Info level (default)
RUST_LOG=info maproom db cleanup-stale --verbose

# Debug level (more details)
RUST_LOG=debug maproom db cleanup-stale --verbose

# Trace level (maximum detail)
RUST_LOG=trace maproom db cleanup-stale --verbose
```

### Expected Log Output

Normal cleanup:
```
🔍 Detecting stale worktrees...
Found 3 stale worktrees
🧹 Cleaning up stale worktrees...
  Deleted: /path/to/worktree1 (245 chunks)
  Deleted: /path/to/worktree2 (1,203 chunks)
  Deleted: /path/to/worktree3 (89 chunks)
✅ Cleanup complete!
  Worktrees deleted: 3
  Chunks cleaned: 1,537
```

No stale worktrees:
```
🔍 Detecting stale worktrees...
✅ No stale worktrees found!
```

### Metrics to Monitor

1. **Cleanup frequency**: How often cleanup finds stale worktrees
2. **Worktrees deleted per run**: Growing numbers may indicate process issues
3. **Chunks cleaned per run**: Large numbers suggest significant stale data
4. **Execution time**: Should typically be < 1 second

## Performance Considerations

### Typical Performance

| Database Size | Execution Time |
|--------------|----------------|
| < 10K chunks | < 100ms |
| 10K-100K chunks | < 500ms |
| 100K+ chunks | < 2s |

### Optimizing Cleanup

1. **Run during low-activity periods**: Avoid cleanup during active indexing
2. **Use SSD storage**: Faster database operations
3. **Regular cleanup**: Smaller, more frequent cleanups are faster than large infrequent ones

## Security Considerations

### Database Permissions

Restrict database file permissions:
```bash
chmod 600 ~/.maproom/maproom.db
```

### Backup Before Cleanup

For production databases:
```bash
# Backup
cp ~/.maproom/maproom.db ~/.maproom/maproom.db.$(date +%Y%m%d)

# Run cleanup
maproom db cleanup-stale --confirm

# Verify
maproom status --repo myrepo
```

### Audit Trail

Maintain cleanup logs for audit purposes:
```bash
maproom db cleanup-stale --confirm --verbose 2>&1 | tee -a /var/log/maproom/audit.log
```

## Future: Watch Integration

When watch integration is reimplemented, the following features will be available:

### Automatic Startup Cleanup

```bash
export MAPROOM_AUTO_CLEANUP=true
maproom watch
```

Behavior:
- Runs cleanup on watch startup (background, non-blocking)
- Watch starts immediately, cleanup runs asynchronously

### Periodic Cleanup During Watch

When enabled, cleanup will run:
- Every 30 minutes during watch sessions
- Only when indexer queue is idle
- Rate-limited to prevent excessive operations

### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `MAPROOM_AUTO_CLEANUP` | `false` | Enable automatic cleanup during watch |

This documentation will be updated when watch integration is available.

## Troubleshooting

### Common Issues

**Cleanup reports error exit code 1:**
- Check database path exists
- Check database permissions
- Check disk space

**Cleanup runs but search still shows duplicates:**
- Run `maproom status` to verify cleanup worked
- May need to re-index affected repositories
- Check for new stale worktrees created since cleanup

**Cleanup takes too long:**
- Check database size
- Check disk I/O performance
- Consider more frequent cleanup schedules

### Getting Help

1. Check logs: `RUST_LOG=debug maproom db cleanup-stale --verbose`
2. Verify database: `sqlite3 ~/.maproom/maproom.db ".schema"`
3. Check documentation: `/docs/user-guide-cleanup.md`

## Related Documentation

- [User Guide: Cleanup](user-guide-cleanup.md) - End-user cleanup instructions
- [Database Architecture](architecture/DATABASE_ARCHITECTURE.md) - Schema details
- [CLAUDE.md](../crates/maproom/CLAUDE.md) - Developer reference
