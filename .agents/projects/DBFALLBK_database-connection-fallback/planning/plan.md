# Implementation Plan: Database Connection Fallback

## Project Overview

Implement consistent database connection fallback logic across Node.js CLI and Rust binary, and remove the confusing devcontainer postgres to use only maproom-postgres.

**Total Estimated Effort**: 1-2 days
**Complexity**: Low-Medium (infrastructure change, high test coverage)

## Phase 1: Remove Devcontainer Postgres

**Goal**: Eliminate dual database setup to use only maproom-postgres

**Deliverables**:
1. Remove postgres service from `.devcontainer/docker-compose.yml`
2. Update DATABASE_URL to point to maproom-postgres
3. Update documentation references
4. Remove unused CREWCHIEF_DB_* environment variables

**Agent**: general-purpose

**Tasks**:
- Remove postgres service definition
- Remove postgres-data volume
- Update depends_on references
- Update DATABASE_URL default
- Remove CREWCHIEF_DB_HOST, CREWCHIEF_DB_PORT, CREWCHIEF_DB_NAME, CREWCHIEF_DB_USER, CREWCHIEF_DB_PASSWORD
- Update CLAUDE.md database architecture section
- Update DATABASE_ARCHITECTURE.md

**Testing**:
- Devcontainer rebuilds successfully
- No references to old postgres service
- maproom-postgres is accessible from devcontainer

**Acceptance Criteria**:
- [ ] postgres service removed from docker-compose.yml
- [ ] DATABASE_URL points to maproom-postgres
- [ ] Documentation updated
- [ ] Unused env vars removed
- [ ] Devcontainer builds and connects to maproom-postgres

## Phase 2: Implement Rust Fallback Logic

**Goal**: Add connection fallback logic to Rust binary

**Deliverables**:
1. New module: `crates/maproom/src/db/connection.rs`
2. Updated `pool.rs` to use fallback
3. Updated `queries.rs` to use fallback
4. Unit tests for fallback logic
5. Integration tests for connection

**Agent**: general-purpose (Rust expertise)

**Tasks**:
- Create `connection.rs` module with `get_database_url()` function
- Implement fallback hierarchy (DATABASE_URL → MAPROOM_DB_HOST → maproom-postgres → localhost)
- Implement hostname resolution check
- Add debug logging
- Update `pool::create_pool()` to call `get_database_url()`
- Update `queries::connect()` to call `get_database_url()`
- Export new module in `db/mod.rs`
- Write unit tests (4 tests)
- Write integration test

**Testing**:
- `cargo test --lib db::connection` passes
- `cargo test connection_fallback_test` passes
- All existing database tests still pass

**Acceptance Criteria**:
- [ ] `connection.rs` implements fallback logic
- [ ] `pool.rs` uses new fallback
- [ ] `queries.rs` uses new fallback
- [ ] 4 unit tests pass
- [ ] 1 integration test passes
- [ ] No regressions in existing tests

## Phase 3: Update Node.js CLI Logic

**Goal**: Make CLI respect existing DATABASE_URL instead of always overriding

**Deliverables**:
1. Updated scan command to check DATABASE_URL first
2. Updated watch command to check DATABASE_URL first
3. Logging to indicate auto-detection vs explicit
4. Node.js tests

**Agent**: general-purpose

**Tasks**:
- Update scan command (line 1524): Only set DATABASE_URL if not already set
- Update watch command (similar location): Same logic
- Add logging: "Using explicit DATABASE_URL" vs "Auto-detected database"
- Write Node.js tests (2 tests)
- Update scan/watch debug output

**Testing**:
- `npm test` passes in maproom-mcp package
- Manual test: DATABASE_URL respected when set
- Manual test: Auto-detection works when not set

**Acceptance Criteria**:
- [ ] scan command respects DATABASE_URL
- [ ] watch command respects DATABASE_URL
- [ ] Logging shows which method used
- [ ] 2 Node.js tests pass
- [ ] Manual tests verify behavior

## Phase 4: End-to-End Testing

**Goal**: Verify all scenarios work correctly

**Deliverables**:
1. Test all 4 user scenarios
2. Document results
3. Fix any issues found

**Agent**: test-runner (manual scenarios) or general-purpose

**Scenarios to Test**:

### Scenario 1: Devcontainer (now using maproom-postgres)
```bash
# DATABASE_URL set to maproom-postgres in docker-compose.yml
cargo run --bin crewchief-maproom -- db status
# Should show: postgresql://maproom:***@maproom-postgres:5432/maproom

node packages/maproom-mcp/bin/cli.cjs scan /workspace
# Should show: Using explicit DATABASE_URL from environment
```

### Scenario 2: MCP User (no DATABASE_URL)
```bash
unset DATABASE_URL
node packages/maproom-mcp/bin/cli.cjs scan /workspace
# Should show: Auto-detected database connection
```

### Scenario 3: Direct Rust Binary (no DATABASE_URL)
```bash
unset DATABASE_URL
cargo run --bin crewchief-maproom -- db status
# Should show: Auto-detected maproom-postgres hostname
```

### Scenario 4: MAPROOM_DB_HOST Override
```bash
export MAPROOM_DB_HOST=maproom-postgres
cargo run --bin crewchief-maproom -- db status
# Should show: Using MAPROOM_DB_HOST: maproom-postgres
```

**Acceptance Criteria**:
- [ ] All 4 scenarios work as expected
- [ ] No database connection errors
- [ ] Logging is clear and helpful
- [ ] Backward compatibility maintained

## Phase 5: Documentation & Cleanup

**Goal**: Update all documentation and remove obsolete references

**Deliverables**:
1. Updated CLAUDE.md
2. Updated DATABASE_ARCHITECTURE.md
3. Updated README for maproom-mcp
4. Migration notes if needed

**Agent**: general-purpose

**Tasks**:
- Update CLAUDE.md database architecture section (remove dual DB info)
- Update DATABASE_ARCHITECTURE.md (remove devcontainer postgres)
- Update maproom-mcp README (connection fallback behavior)
- Add troubleshooting guide for connection issues
- Remove any references to unused env vars

**Acceptance Criteria**:
- [ ] CLAUDE.md updated
- [ ] DATABASE_ARCHITECTURE.md updated
- [ ] README.md updated
- [ ] No references to old devcontainer postgres

## Rollback Plan

If issues arise:

1. **Rollback Phase 1** (devcontainer):
   - Restore postgres service to docker-compose.yml
   - Restore old DATABASE_URL
   - Rebuild devcontainer

2. **Rollback Phases 2-3** (code changes):
   - Revert Rust changes: `git revert <commits>`
   - Revert Node.js changes: `git revert <commits>`
   - Database connection falls back to requiring explicit DATABASE_URL

3. **Data Recovery**:
   - No data loss risk - this is configuration only
   - maproom-postgres data persists in volumes

## Dependencies

**Before Starting**:
- maproom-postgres must be running and accessible
- Existing tests must be passing
- No pending database migrations

**External Dependencies**:
- Docker Compose (for maproom-postgres)
- PostgreSQL client libraries (already present)

## Risk Mitigation

**Risk**: Breaking existing developer workflows
- **Mitigation**: Comprehensive testing, clear communication
- **Fallback**: Rollback plan ready

**Risk**: Connection issues in different environments
- **Mitigation**: Test in devcontainer, local, CI/CD
- **Fallback**: Fallback to localhost still available

**Risk**: Performance degradation from hostname checks
- **Mitigation**: 1-second timeout, only on startup
- **Measurement**: Monitor connection time

## Success Metrics

1. **Functionality**:
   - ✅ All database connections work in all scenarios
   - ✅ Fallback logic consistent across Node.js and Rust
   - ✅ Devcontainer uses single database

2. **Testing**:
   - ✅ 15+ tests passing (4 Rust unit, 1 Rust integration, 2 Node.js, 4 scenarios)
   - ✅ No regressions in existing tests
   - ✅ CI/CD pipeline green

3. **Documentation**:
   - ✅ Clear explanation of fallback behavior
   - ✅ Troubleshooting guide for connection issues
   - ✅ No references to removed devcontainer postgres

4. **User Experience**:
   - ✅ MCP users don't notice any change (still auto-detects)
   - ✅ Developers have simpler mental model (one database)
   - ✅ Explicit DATABASE_URL works correctly

## Timeline

**Day 1**:
- Morning: Phase 1 (Remove devcontainer postgres) - 2 hours
- Afternoon: Phase 2 (Rust fallback logic) - 4 hours
- Evening: Phase 3 (Node.js CLI updates) - 2 hours

**Day 2**:
- Morning: Phase 4 (End-to-end testing) - 3 hours
- Afternoon: Phase 5 (Documentation) - 2 hours
- Final: Code review and merge - 1 hour

**Total**: ~14 hours spread over 1-2 days

## Post-Deployment

**Monitoring** (first week after merge):
- Watch for connection errors in logs
- Monitor MCP server uptime
- Track developer feedback in Slack/issues

**Support**:
- Update onboarding docs for new developers
- Announce change in team channels
- Provide migration guide if needed

## Conclusion

This is a straightforward infrastructure improvement with high test coverage. The main complexity is ensuring backward compatibility and comprehensive testing across environments.

Removing the devcontainer postgres simplifies the architecture significantly and eliminates a major source of confusion.
