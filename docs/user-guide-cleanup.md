# User Guide: Stale Worktree Cleanup

This guide explains how to clean up stale worktrees from your Maproom index database, improving search quality and reducing duplicate results.

## What is a Stale Worktree?

A **stale worktree** is a database entry for a git worktree directory that no longer exists on disk. This commonly happens when:

- You delete a git worktree with `git worktree remove`
- You delete a branch's local checkout
- You move or rename a repository directory
- A temporary worktree was created for testing and later removed

Stale worktrees cause **duplicate search results** because Maproom returns matches from both current and deleted worktrees.

## Quick Start

```bash
# Preview what will be deleted (safe - no changes made)
maproom db cleanup-stale

# Actually delete stale worktrees
maproom db cleanup-stale --confirm
```

## Step-by-Step Workflow

### Step 1: Preview (Dry-Run)

Always start with a dry-run to see what will be deleted:

```bash
maproom db cleanup-stale --verbose
```

Example output:
```
🔍 Detecting stale worktrees...
Found 3 stale worktrees:

  Worktree: /home/user/projects/myrepo-feature-branch
  Repository: myrepo
  Last indexed: 2025-11-20 14:30:00

  Worktree: /home/user/projects/old-project
  Repository: old-project
  Last indexed: 2025-11-15 09:15:00

  Worktree: /tmp/test-worktree
  Repository: myrepo
  Last indexed: 2025-11-25 11:00:00

Total: 3 stale worktrees
Run with --confirm to delete.
```

### Step 2: Review the List

Before confirming deletion, verify:

1. **Check the paths**: Do these directories actually not exist?
2. **Check the repository names**: Are these worktrees you no longer need?
3. **Check the timestamps**: Old timestamps often indicate abandoned worktrees

You can manually verify a path doesn't exist:
```bash
ls -la /home/user/projects/myrepo-feature-branch
# Should show "No such file or directory"
```

### Step 3: Confirm Deletion

Once you've reviewed and are satisfied, run with `--confirm`:

```bash
maproom db cleanup-stale --confirm
```

Example output:
```
🔍 Detecting stale worktrees...
Found 3 stale worktrees

🧹 Cleaning up stale worktrees...
  Deleted: /home/user/projects/myrepo-feature-branch (245 chunks)
  Deleted: /home/user/projects/old-project (1,203 chunks)
  Deleted: /tmp/test-worktree (89 chunks)

✅ Cleanup complete!
  Worktrees deleted: 3
  Chunks cleaned: 1,537
```

### Step 4: Verify Search Quality

After cleanup, run a search to verify improved results:

```bash
maproom search --query "your search term" --repo myrepo
```

You should see fewer duplicate results from deleted worktrees.

## Understanding Exit Codes

| Exit Code | Meaning | Action |
|-----------|---------|--------|
| `0` | Success | Cleanup completed (or dry-run preview shown) |
| `1` | Error | Check error message (database connection, permissions) |
| `2` | No stale worktrees | Nothing to clean - your database is already clean |

## Command Reference

### Basic Commands

```bash
# Dry-run (default, safe)
maproom db cleanup-stale

# Confirm deletion
maproom db cleanup-stale --confirm

# Verbose output (shows each worktree)
maproom db cleanup-stale --verbose

# Combine flags
maproom db cleanup-stale --confirm --verbose
```

### Building from Source

If running from the codebase:

```bash
# Dry-run
cargo run --bin maproom -- db cleanup-stale

# Confirm
cargo run --bin maproom -- db cleanup-stale --confirm

# Verbose
cargo run --bin maproom -- db cleanup-stale --verbose
```

## Safety Features

### Dry-Run by Default

The cleanup command **never deletes data without explicit confirmation**. Running without `--confirm` only shows what would be deleted.

### Multi-Worktree Chunk Safety

When a code chunk is shared between multiple worktrees (e.g., common code in main and feature branches), Maproom only removes the **association** between the chunk and the stale worktree. The chunk data is preserved if other worktrees still reference it.

### Verbose Progress Reporting

Use `--verbose` to see exactly what's happening during cleanup:
- Each worktree being processed
- Number of chunks being cleaned
- Any errors encountered

## Troubleshooting

### "No stale worktrees found" (Exit Code 2)

This is normal - your database is clean. This happens when:
- All indexed worktrees still exist on disk
- You've already run cleanup recently
- You're using a fresh database

### "Database connection failed" (Exit Code 1)

Check your database configuration:

```bash
# Verify database path
echo $MAPROOM_DATABASE_URL

# Default location
ls -la ~/.maproom/maproom.db
```

### Cleanup Takes Too Long

For large databases, cleanup may take longer. Use `--verbose` to monitor progress. Typical performance:
- Small database (< 10K chunks): < 100ms
- Medium database (10K-100K chunks): < 500ms
- Large database (100K+ chunks): < 2s

### False Positives (Valid Worktree Flagged as Stale)

If a valid worktree is incorrectly flagged:

1. Check if the path exists: `ls -la /path/to/worktree`
2. Check file permissions: Maproom needs read access to verify directory existence
3. Check for symbolic links: Follow symlinks to verify the actual path

If you find a bug, do NOT run with `--confirm`. Report the issue instead.

## Recovery Procedures

### If You Accidentally Delete Valid Data

Maproom cleanup only removes **metadata** (worktree entries and chunk associations) from the database. Your actual code files are never touched.

To restore deleted worktree data:

1. **Re-index the worktree**:
   ```bash
   maproom scan --path /path/to/worktree --repo myrepo --worktree main
   ```

2. **Generate embeddings** (if using semantic search):
   ```bash
   maproom generate-embeddings --repo myrepo
   ```

### Backup Recommendations

Before running cleanup on important databases:

```bash
# Backup the database
cp ~/.maproom/maproom.db ~/.maproom/maproom.db.backup

# Or specify a custom database
cp /path/to/custom.db /path/to/custom.db.backup
```

To restore from backup:
```bash
cp ~/.maproom/maproom.db.backup ~/.maproom/maproom.db
```

## Best Practices

1. **Run periodically**: Clean up every few weeks to maintain search quality
2. **Always preview first**: Use dry-run before `--confirm`
3. **Use verbose for large cleanups**: Monitor progress with `--verbose`
4. **Backup before major cleanups**: Especially on production databases
5. **Re-index after cleanup**: If you notice missing search results

## Related Documentation

- [CLAUDE.md](../crates/maproom/CLAUDE.md) - Maproom development and commands
- [Database Architecture](architecture/DATABASE_ARCHITECTURE.md) - SQLite schema details
