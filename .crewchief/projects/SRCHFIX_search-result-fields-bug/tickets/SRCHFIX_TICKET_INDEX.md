# SRCHFIX Ticket Index

## Project Overview
Search Result Fields Bug Fix - Complete data plumbing to expose chunk_id, symbol_name, and kind fields from Rust daemon to TypeScript clients.

**Project Path**: `.crewchief/projects/SRCHFIX_search-result-fields-bug`

## Ticket Summary

### Phase 1: Fix Data Serialization and Types (4 tickets)

Objective: Add missing fields to daemon JSON response and update TypeScript interfaces.

| Ticket ID | Title | Agent | Status | Estimated Time |
|-----------|-------|-------|--------|----------------|
| SRCHFIX-1001 | Update Rust Daemon Serialization | rust-expert | Not Started | 30 min |
| SRCHFIX-1002 | Update TypeScript Daemon Client Interface | typescript-expert | Not Started | 45 min |
| SRCHFIX-1003 | Update Maproom MCP Mapping Code | typescript-expert | Not Started | 1 hour |
| SRCHFIX-1004 | Search for Old Field Name Usage | typescript-expert | Not Started | 30 min |

**Phase 1 Total Estimated Time**: 2.75 hours

**Dependencies**:
- All Phase 1 tickets can run in parallel
- SRCHFIX-1002 should complete before SRCHFIX-1003 (type safety)
- SRCHFIX-1004 should complete before SRCHFIX-1002 (verify rename is safe)

### Phase 2: Validation and Testing (3 tickets)

Objective: Verify all three fields are correctly populated in search results.

| Ticket ID | Title | Agent | Status | Estimated Time |
|-----------|-------|-------|--------|----------------|
| SRCHFIX-2001 | Run Existing Tests | test-runner | Not Started | 15 min |
| SRCHFIX-2002 | Create Integration Test | typescript-expert | Not Started | 1.5 hours |
| SRCHFIX-2003 | Manual Validation | typescript-expert | Not Started | 30 min |

**Phase 2 Total Estimated Time**: 2.25 hours

**Dependencies**:
- All Phase 2 tickets require all Phase 1 tickets complete
- SRCHFIX-2002 requires SRCHFIX-2001 (verify no regressions)
- SRCHFIX-2003 requires SRCHFIX-2002 (automated tests pass first)

## Project Total

- **Total Tickets**: 7
- **Total Estimated Time**: 5 hours
- **Phase 1**: 4 tickets (2.75 hours)
- **Phase 2**: 3 tickets (2.25 hours)

## Completion Criteria

### Phase 1 Complete When:
- [ ] Rust daemon serializes chunk_id in JSON response
- [ ] TypeScript SearchResult interface has chunk_id, symbol_name, kind fields
- [ ] Mapping code uses actual values from daemon (not hardcoded defaults)
- [ ] All obsolete fallback code removed
- [ ] TypeScript and Rust compilation succeed with no errors

### Phase 2 Complete When:
- [ ] Integration test passes (all fields populated correctly)
- [ ] Manual test confirms fields are present and valid
- [ ] Context retrieval works using chunk_id from search results
- [ ] No regressions in existing tests

### Project Complete When:
- [ ] All Phase 1 and Phase 2 criteria met
- [ ] Code committed to main branch
- [ ] Documentation updated (if needed)

## Ticket Details

### SRCHFIX-1001: Update Rust Daemon Serialization
Add `chunk_id` field to daemon JSON response serialization.

**Key Changes**:
- File: `crates/maproom/src/daemon/mod.rs` (line 332-340)
- Add: `"chunk_id": hit.chunk_id` to serde_json::json! macro

**Acceptance Criteria**:
- Daemon serializes chunk_id field in search hit JSON
- cargo build and clippy succeed

### SRCHFIX-1002: Update TypeScript Daemon Client Interface
Sync TypeScript interfaces with Rust daemon response structure.

**Key Changes**:
- Files: `packages/daemon-client/src/client.ts`, `packages/maproom-mcp/src/daemon-client/client.ts`
- Rename: `chunk_index` → `chunk_id`
- Add: `symbol_name: string | null`, `kind: string`
- Add sync comment pointing to Rust struct

**Acceptance Criteria**:
- Both daemon-client interfaces match exactly
- TypeScript compilation succeeds
- Sync comment added

### SRCHFIX-1003: Update Maproom MCP Mapping Code
Use actual field values from daemon instead of hardcoded empty strings.

**Key Changes**:
- File: `packages/maproom-mcp/src/tools/search.ts`
- Update rustOutput mapping to use daemon values
- Remove obsolete chunkIdMap fallback code
- Get chunk_id directly from daemonHit

**Acceptance Criteria**:
- Mapping uses daemon values (not hardcoded)
- chunkIdMap code removed
- TypeScript compilation succeeds

### SRCHFIX-1004: Search for Old Field Name Usage
Verify renaming chunk_index to chunk_id is safe.

**Key Changes**:
- Search for `chunk_index` and `chunkIndex` in codebase
- Document findings
- Replace any usage with `chunk_id`

**Acceptance Criteria**:
- Searched all TypeScript packages
- Documented all findings
- Replaced any usage or confirmed only interface definitions found

### SRCHFIX-2001: Run Existing Tests
Verify no regressions from Phase 1 changes.

**Key Changes**:
- Run cargo test (Rust)
- Run pnpm test (daemon-client, maproom-mcp)
- Document results

**Acceptance Criteria**:
- All existing tests pass
- Test output captured
- No regressions detected

### SRCHFIX-2002: Create Integration Test
End-to-end validation of field population.

**Key Changes**:
- New file: `packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts`
- Test chunk_id, symbol_name, kind population
- Test context retrieval with chunk_id
- Graceful skip if database unavailable

**Acceptance Criteria**:
- Integration test file created
- All 5 test cases implemented
- Tests pass when database available
- Tests skip gracefully when database unavailable

### SRCHFIX-2003: Manual Validation
Human verification of bug fix in live environment.

**Key Changes**:
- Build and run MCP server
- Perform search and inspect JSON
- Test context retrieval
- Document results with examples

**Acceptance Criteria**:
- MCP server runs successfully
- All three fields present in responses
- Context retrieval works
- Validation results documented with JSON examples

## Quick Reference

**Files Modified in Phase 1**:
- `crates/maproom/src/daemon/mod.rs`
- `packages/daemon-client/src/client.ts`
- `packages/maproom-mcp/src/daemon-client/client.ts`
- `packages/maproom-mcp/src/tools/search.ts`

**Files Created in Phase 2**:
- `packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts`

**Key Fields Being Fixed**:
- `chunk_id`: Database ID for chunk (enables context retrieval)
- `symbol_name`: Name of function/class/method (or null for anonymous)
- `kind`: Type of symbol (function, class, method, etc.)

## Planning References

- **Plan**: `.crewchief/projects/SRCHFIX_search-result-fields-bug/planning/plan.md`
- **Architecture**: `.crewchief/projects/SRCHFIX_search-result-fields-bug/planning/architecture.md`
- **Quality Strategy**: `.crewchief/projects/SRCHFIX_search-result-fields-bug/planning/quality-strategy.md`
