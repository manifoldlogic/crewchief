# Database Connection Fallback

**Project Slug**: `DBFALLBK`
**Status**: Planning Complete
**Estimated Effort**: 1-2 days

## Problem

CrewChief's Maproom component has inconsistent database connection behavior:

1. **Devcontainer Confusion**: Two PostgreSQL databases (devcontainer postgres + maproom-postgres) cause confusion about which is being used
2. **CLI Overrides Explicit Config**: Node.js CLI always overrides `DATABASE_URL` environment variable, ignoring explicit configuration
3. **Rust Requires Explicit Config**: Rust binary fails without `DATABASE_URL` set, no auto-detection
4. **Inconsistent Behavior**: Different components behave differently, confusing developers

**Example Problem**:
```bash
# In devcontainer with DATABASE_URL set to postgres:5432/crewchief
node packages/maproom-mcp/bin/cli.cjs scan /workspace
# Unexpectedly uses maproom-postgres:5432/maproom instead!

# Direct Rust binary usage
cargo run --bin crewchief-maproom -- scan --path /workspace
# Fails: "DATABASE_URL env var is required"
```

## Solution

Implement **consistent fallback logic** across Node.js CLI and Rust binary, and **remove devcontainer postgres** to eliminate confusion.

### Unified Fallback Hierarchy

Both components will use this priority:

1. **DATABASE_URL** (if set) → Respect explicit configuration
2. **MAPROOM_DB_HOST** (if set) → Build connection string from components
3. **maproom-postgres** hostname (if resolves) → Auto-detect container
4. **localhost:5433** → Development fallback

### Key Changes

**Remove devcontainer postgres**:
- Single database: maproom-postgres
- No more dual database confusion
- Simpler mental model

**Node.js CLI**:
- Respect existing `DATABASE_URL` instead of always overriding
- Only use fallback logic when `DATABASE_URL` not set
- Log which method was used

**Rust Binary**:
- Add fallback logic (currently requires explicit config)
- Match Node.js CLI behavior exactly
- Auto-detect maproom-postgres for typical users

## Benefits

- ✅ **Consistency**: Node.js and Rust behave identically
- ✅ **Simplicity**: One database instead of two
- ✅ **Predictability**: Explicit config always respected
- ✅ **Usability**: Auto-detection works for typical users
- ✅ **Backward Compatible**: Existing usage patterns still work

## Agents

**Primary**: `general-purpose` (all phases)
- Rust development (fallback logic)
- Node.js development (CLI updates)
- Docker Compose changes
- Documentation updates
- Testing

**Optional**: `test-runner` (Phase 4 - end-to-end scenarios)

## Planning Documents

- [Analysis](planning/analysis.md) - Problem space, research, current state
- [Architecture](planning/architecture.md) - Solution design, implementation details
- [Quality Strategy](planning/quality-strategy.md) - Testing approach, coverage
- [Security Review](planning/security-review.md) - Security considerations
- [Plan](planning/plan.md) - Phases, deliverables, timeline

## Success Criteria

1. **Devcontainer uses single database** (maproom-postgres)
2. **Explicit DATABASE_URL respected** in all scenarios
3. **Auto-detection works** when DATABASE_URL not set
4. **All tests passing** (15+ tests across Rust and Node.js)
5. **Documentation updated** to reflect single database architecture
6. **No regressions** in existing functionality

## Quick Start

After implementation, connection behavior will be:

```bash
# Scenario 1: Explicit DATABASE_URL (highest priority)
export DATABASE_URL="postgresql://user:pass@host:5432/db"
cargo run --bin crewchief-maproom -- db status
# Uses: postgresql://user:***@host:5432/db

# Scenario 2: No DATABASE_URL (auto-detect)
unset DATABASE_URL
cargo run --bin crewchief-maproom -- db status
# Uses: postgresql://maproom:***@maproom-postgres:5432/maproom

# Scenario 3: MAPROOM_DB_HOST override
export MAPROOM_DB_HOST=custom-host
export MAPROOM_DB_PORT=5555
cargo run --bin crewchief-maproom -- db status
# Uses: postgresql://maproom:***@custom-host:5555/maproom

# Scenario 4: Devcontainer (DATABASE_URL set to maproom-postgres)
# Uses DATABASE_URL from docker-compose.yml
node packages/maproom-mcp/bin/cli.cjs scan /workspace
# Uses: postgresql://maproom:***@maproom-postgres:5432/maproom
```

## Timeline

**Phase 1**: Remove devcontainer postgres (2 hours)
**Phase 2**: Rust fallback logic (4 hours)
**Phase 3**: Node.js CLI updates (2 hours)
**Phase 4**: End-to-end testing (3 hours)
**Phase 5**: Documentation (2 hours)

**Total**: 1-2 days

## Files Changed

**Configuration**:
- `.devcontainer/docker-compose.yml` - Remove postgres service
- `crates/maproom/src/db/connection.rs` - New fallback module
- `crates/maproom/src/db/pool.rs` - Use fallback
- `crates/maproom/src/db/queries.rs` - Use fallback
- `packages/maproom-mcp/bin/cli.cjs` - Respect DATABASE_URL

**Documentation**:
- `CLAUDE.md` - Update database architecture section
- `docs/architecture/DATABASE_ARCHITECTURE.md` - Remove dual DB info
- `packages/maproom-mcp/README.md` - Connection behavior

**Tests**:
- `crates/maproom/src/db/connection.rs` - Unit tests
- `crates/maproom/tests/connection_fallback_test.rs` - Integration test
- `packages/maproom-mcp/tests/connection-fallback.test.js` - Node.js tests
