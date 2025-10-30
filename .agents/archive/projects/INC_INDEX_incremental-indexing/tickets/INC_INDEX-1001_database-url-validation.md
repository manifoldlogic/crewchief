# Ticket: INC_INDEX-1001: Fix DATABASE_URL Configuration and Validation in Maproom Watch

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Improve database connection handling in the maproom watch command to provide clear error messages when DATABASE_URL is misconfigured, particularly in Docker/devcontainer environments where PostgreSQL runs in a separate container.

## Background
During troubleshooting of the watch functionality after implementing MCP_CORE-2003, it was discovered that the watch command was failing silently when DATABASE_URL pointed to the wrong hostname. The watch command would start but fail with "Connection refused (os error 111)" when trying to connect to the database.

In Docker/devcontainer environments, PostgreSQL runs in a separate container with hostname `postgres:5432`, but DATABASE_URL often defaults to or is configured as `localhost:5432`, causing connection failures. The watch command provides no clear indication of this misconfiguration, making it difficult to diagnose.

**Symptoms observed:**
- Watch command starts but shows no "Started incremental watch" message
- Status shows `total_processed=0`
- No file changes are detected
- Error only visible with RUST_LOG=debug: "error connecting to server: Connection refused (os error 111)"

**Working configuration (Docker):**
```bash
export DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief"
```

**Non-working configuration:**
```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/crewchief"
```

## Acceptance Criteria
- [x] Watch command validates database connection on startup before starting file watcher
- [x] Clear error message is displayed if database is unreachable, including the DATABASE_URL being used
- [x] Error message provides actionable guidance (e.g., "Check that DATABASE_URL points to correct hostname")
- [x] README.md includes a DATABASE_URL configuration section explaining requirements for different environments
- [x] Example .env file created with commented variants for both localhost and Docker configurations
- [x] Watch command fails fast with helpful message rather than appearing to run while silently failing

## Technical Requirements
- Modify watch command to test database connection on startup (before starting watcher)
- Add error handling for database connection failures with descriptive messages
- Include the DATABASE_URL value in error messages (sanitize password if present)
- Update README.md with DATABASE_URL configuration section
- Create `.env.example` file with commented variants for different deployment scenarios
- Consider auto-detecting Docker environment and suggesting correct URL in error messages
- Ensure connection validation doesn't significantly slow down watch startup

## Implementation Notes

### Connection Validation Strategy
1. In `watch_worktree` function, test database connection before starting the file watcher
2. Use existing pool creation logic but catch connection errors explicitly
3. Parse DATABASE_URL to extract hostname for display in error messages
4. Provide specific guidance based on detected environment (if in Docker, suggest `postgres:5432`)

### Error Message Format
```
Error: Failed to connect to database
  DATABASE_URL: postgresql://postgres:***@localhost:5432/crewchief
  Error: Connection refused (os error 111)

  Troubleshooting:
  - Verify PostgreSQL is running
  - In Docker/devcontainer, use hostname 'postgres' instead of 'localhost'
  - Example: postgresql://postgres:postgres@postgres:5432/crewchief
```

### Documentation Updates
- Add "Database Configuration" section to README.md
- Explain DATABASE_URL format and requirements
- Provide examples for local development vs Docker environments
- Add troubleshooting subsection for common connection issues

### Example .env File Structure
```bash
# Local development (PostgreSQL running on host)
# DATABASE_URL="postgresql://postgres:postgres@localhost:5432/crewchief"

# Docker/devcontainer (PostgreSQL in separate container)
DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief"

# Custom configuration
# DATABASE_URL="postgresql://username:password@hostname:port/database"
```

## Dependencies
- None - this is a standalone improvement

## Risk Assessment
- **Risk**: Connection validation might add startup latency to watch command
  - **Mitigation**: Use a short timeout for validation check (e.g., 5 seconds). Connection test should be quick if database is reachable.

- **Risk**: Exposing DATABASE_URL in error messages could leak credentials
  - **Mitigation**: Sanitize password from URL before displaying (replace with `***`)

- **Risk**: Auto-detection of Docker environment might not work in all cases
  - **Mitigation**: Make suggestion conditional and always show the actual DATABASE_URL being used so users can verify

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` - Add connection validation in `watch_worktree` function
- `crates/maproom/src/db/pool.rs` - Improve error messages in `create_pool` function
- `crates/maproom/README.md` - Add DATABASE_URL configuration and troubleshooting documentation
- `crates/maproom/.env.example` - Create new file with database URL examples

## Implementation Summary

### Changes Made

1. **Enhanced `create_pool()` in `src/db/pool.rs`**:
   - Added comprehensive error handling with `.map_err()` when getting initial connection
   - Error message displays sanitized DATABASE_URL (password replaced with `***`)
   - Provides context-specific troubleshooting guidance
   - Detects localhost usage and suggests `postgres` hostname for Docker/devcontainer
   - Added `sanitize_database_url()` helper function to safely display URLs

2. **Updated `watch_worktree()` in `src/indexer/mod.rs`**:
   - Added database connection validation before starting file watcher (fast-fail)
   - Validates connection by checking for `maproom` schema in database
   - Provides clear error messages if schema is missing
   - Shows success message when validation passes: "✅ Database connection validated successfully"
   - Adds informative message: "🔌 Validating database connection..."

3. **Documentation in `README.md`**:
   - Added comprehensive "Database Configuration" section
   - Documented DATABASE_URL format with examples
   - Provided separate examples for local development vs Docker/devcontainer
   - Added troubleshooting section covering common issues:
     - Connection refused errors
     - Authentication failures
     - Missing database/schema
   - Explained validation behavior of watch command

4. **Enhanced `.env.example`**:
   - Restructured with clear sections and headers
   - Added both localhost and Docker/devcontainer configurations
   - Included helpful comments explaining when to use each variant
   - Added note about common "Connection refused" issue

### Key Features

- **Fast-fail behavior**: Watch command validates database connection before starting watcher
- **Password sanitization**: DATABASE_URL displayed in errors with password replaced by `***`
- **Context-aware guidance**: Suggests Docker-specific solutions when localhost is detected
- **Clear success/failure feedback**: Users see immediate validation results
- **Comprehensive documentation**: README covers all common configuration scenarios

### Testing

The implementation compiles successfully with:
```bash
cargo build --release
```

No new clippy warnings introduced (pre-existing warnings are unrelated to this change).
