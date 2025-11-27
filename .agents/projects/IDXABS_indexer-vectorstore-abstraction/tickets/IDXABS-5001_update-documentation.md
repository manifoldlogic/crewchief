# Ticket: IDXABS-5001: Update Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation only)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Documentation ticket - no tests required
- Verify examples in documentation work as written
- Ensure no PostgreSQL references remain in user-facing docs

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Update crate documentation to reflect SQLite-only architecture, removing PostgreSQL references and updating examples.

## Background
With the migration complete, documentation needs to accurately reflect that SQLite is the only supported backend. This prevents user confusion and documents the simplified setup.

**Reference**: Phase 5, Ticket 5001 of `planning/plan.md` - "Update CLAUDE.md"

## Acceptance Criteria
- [x] `crates/maproom/CLAUDE.md` updated (no PostgreSQL references)
- [x] Environment variable documentation accurate for SQLite-only
- [x] Example commands work as documented
- [x] Default database path documented (`~/.maproom/maproom.db`)
- [x] No references to `--features sqlite` flag
- [x] No references to `--parallel` flag

## Technical Requirements
- Remove all PostgreSQL references from documentation
- Update environment variable section:
  - `MAPROOM_DATABASE_URL` - SQLite URL format (`sqlite:///path/to/db`)
  - Remove PostgreSQL connection string examples
- Update command examples to remove deprecated flags
- Add SQLite-specific notes if relevant

## Implementation Notes

### Documentation Updates for CLAUDE.md

#### Remove/Update
- PostgreSQL connection string examples
- `--features sqlite` mentions
- `--parallel` flag mentions
- Dual-backend architecture references
- Any `BackendType` mentions

#### Add/Clarify
```markdown
## Database

Maproom uses SQLite for storage. By default, the database is created at:
```
~/.maproom/maproom.db
```

### Environment Variables

- `MAPROOM_DATABASE_URL` - Override default database location
  - Example: `MAPROOM_DATABASE_URL="sqlite:///tmp/maproom.db"`
  - Default: `sqlite://~/.maproom/maproom.db`

### Quick Start

```bash
# Index a repository
cargo run --bin crewchief-maproom -- scan --path /path/to/repo

# Check status
cargo run --bin crewchief-maproom -- status

# Search
cargo run --bin crewchief-maproom -- search --query "function name"

# Generate embeddings (requires embedding provider)
cargo run --bin crewchief-maproom -- generate-embeddings
```
```

### Other Documentation Files
Check and update if needed:
- `crates/maproom/README.md` (if exists)
- Root `README.md` if it references maproom
- Any API documentation

### Verification
```bash
# Verify examples work
cargo run --bin crewchief-maproom -- --help

# Check for remaining PostgreSQL references
grep -ri "postgresql\|postgres\|tokio.postgres\|pgvector" crates/maproom/CLAUDE.md
# Should return nothing

# Check for deprecated flags
grep -ri "features sqlite\|--parallel" crates/maproom/CLAUDE.md
# Should return nothing
```

## Dependencies
- IDXABS-1001 through IDXABS-4002 (all previous tickets)

## Risk Assessment
- **Risk**: Documentation doesn't match actual behavior
  - **Mitigation**: Test all examples before committing
  - **Mitigation**: Run E2E script to verify commands work
- **Risk**: Missing edge cases in documentation
  - **Mitigation**: Document common scenarios (default path, custom path)
  - **Mitigation**: Include troubleshooting section

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/CLAUDE.md` - Main crate documentation

Files to CHECK (update if PostgreSQL references found):
- `crates/maproom/README.md` (if exists)
- `README.md` (root, maproom section if any)
- Any files in `docs/` that reference maproom

## Completion Note
This is the final ticket in the IDXABS project. After this ticket:
1. All PostgreSQL code is deleted
2. SQLite is the only backend
3. All commands work without feature flags
4. Documentation is accurate
5. Tests pass

The project can be marked complete and moved to archive.
