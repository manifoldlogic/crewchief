# Ticket: MD_ENHANCE-4001: Migration Script

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- integration-tester
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Create a migration script that backs up existing markdown chunk data, re-parses all markdown files using the new tree-sitter parser, updates the database with enhanced chunks, and verifies data integrity. This safely transitions from the old regex parser to the new AST-based parser.

## Background
Existing markdown files in the database were indexed with the regex parser, which lacks hierarchy information and detailed structure. We need to migrate all existing data to the new format without data loss, maintaining rollback capability in case of issues.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 80-84

## Acceptance Criteria
- [ ] Backup created of all existing markdown chunks before migration
- [ ] All markdown files re-parsed using new MarkdownParser
- [ ] Old chunks deleted (or marked deprecated) for migrated files
- [ ] New chunks inserted with full metadata (parent_path, language, etc.)
- [ ] Migration completes without data loss
- [ ] Rollback procedure documented and tested
- [ ] Migration logs all actions for audit trail

## Technical Requirements
- Create `MarkdownMigrator` struct with old and new parser instances
- Implement database backup strategy (pg_dump or snapshot table)
- Query all files with mime_type = 'text/markdown' from maproom.files
- For each file: parse content with new parser, generate chunks
- Transaction-based migration: backup → delete old → insert new → commit
- Count chunks before/after to verify no data loss
- Log migration statistics: files processed, chunks before/after, errors
- Implement rollback command to restore from backup
- Add migration status tracking to database

Architecture Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md` lines 240-266

## Implementation Notes

### Migration Strategy
```rust
pub struct MarkdownMigrator {
    old_parser: RegexParser,
    new_parser: MarkdownParser,
}

impl MarkdownMigrator {
    pub async fn migrate(&self, repo_id: i64) -> Result<MigrationStats> {
        // 1. Backup
        self.create_backup(repo_id).await?;

        // 2. Get all markdown files
        let files = self.get_markdown_files(repo_id).await?;

        let mut stats = MigrationStats::default();

        for file in files {
            // 3. Parse with new parser
            let new_chunks = self.new_parser.parse(&file.content)?;

            // 4. Transaction: delete old, insert new
            let tx = self.db.begin().await?;

            let old_count = self.delete_old_chunks(&tx, file.id).await?;
            let new_count = self.insert_new_chunks(&tx, file.id, new_chunks).await?;

            tx.commit().await?;

            stats.record(file.id, old_count, new_count);
        }

        Ok(stats)
    }
}
```

### Backup Strategy
- Create temporary table: `chunks_backup_YYYYMMDD`
- Copy all markdown-related chunks
- Store backup metadata (timestamp, file count, chunk count)
- Keep backups for 30 days

### Rollback Procedure
```bash
# Restore from backup
cargo run --bin crewchief-maproom -- migrate rollback --backup-id 20250124
```

### Migration Logging
```
[2025-01-24 10:00:00] Starting migration for repo_id=1
[2025-01-24 10:00:01] Backup created: chunks_backup_20250124
[2025-01-24 10:00:02] Processing file: README.md (18 old chunks)
[2025-01-24 10:00:02] Migrated README.md (25 new chunks, +7)
[2025-01-24 10:05:00] Migration complete: 142 files, 856 old chunks, 1203 new chunks
```

### Verification Checks
- File count unchanged
- No chunks orphaned (all have valid file_id)
- Parent paths valid for all heading chunks
- Code blocks have language metadata
- Total chunk count increased (more detailed extraction)

Reference Architecture: lines 248-264 for migration implementation

## Dependencies
- MD_ENHANCE-1001 (Parser Setup) - MUST be completed
- MD_ENHANCE-1002 (AST Walking) - MUST be completed
- MD_ENHANCE-2001 (Parent Tracking) - MUST be completed
- MD_ENHANCE-2002 (Section Boundaries) - MUST be completed
- MD_ENHANCE-3001 (Code Block Processing) - MUST be completed
- MD_ENHANCE-3002 (Link Resolution) - MUST be completed

## Risk Assessment
- **Risk**: Migration fails mid-way, leaving database in inconsistent state
  - **Mitigation**: Use database transactions, implement checkpointing, create backup before starting

- **Risk**: New parser generates more chunks, database runs out of space
  - **Mitigation**: Check disk space before migration, estimate new chunk count, monitor during migration

- **Risk**: Rollback fails to restore data correctly
  - **Mitigation**: Test rollback procedure on test database, verify backup integrity before migration

- **Risk**: Migration takes too long, blocking other operations
  - **Mitigation**: Run during maintenance window, implement progress reporting, allow pause/resume

## Files/Packages Affected
- `crates/maproom/src/migrate/markdown.rs` - New file for MarkdownMigrator
- `crates/maproom/src/migrate/mod.rs` - Module exports
- `crates/maproom/src/cli/migrate.rs` - CLI commands for migration
- `crates/maproom/migrations/` - Backup table schema
- `crates/maproom/tests/migration_test.rs` - Test migration on sample data
- `scripts/migrate-markdown.sh` - Shell wrapper for migration command
