# Ticket: CICLEAN-1003: Update workflow header documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only change)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- "Tests pass - N/A" for documentation-only changes

## Agents
- code-editor
- verify-ticket
- commit-ticket

## Summary
Update the header documentation in `.github/workflows/test.yml` to accurately reflect SQLite-only architecture and remove references to PostgreSQL as a dual-backend option.

## Background
The CI workflow header comments still describe a "SQLite-First Testing Strategy" with PostgreSQL as an "Integration" option. This is misleading because:

1. PostgreSQL support was completely removed
2. Only SQLite backend exists in the codebase
3. Comments suggest PostgreSQL is available for "team sharing/production validation"

These outdated comments can confuse future contributors and lead to attempts to re-add PostgreSQL support.

**Planning Reference**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/architecture.md`

## Acceptance Criteria
- [x] Workflow title updated from "SQLite-First" to "SQLite-Only"
- [x] DATABASE BACKENDS section updated to show SQLite only
- [x] PostgreSQL references removed from header comments
- [x] Job organization section updated to remove PostgreSQL jobs
- [x] Section divider comments simplified (no "Primary" vs "Integration" distinction)
- [x] Comments accurately reflect current codebase architecture

## Technical Requirements

### 1. Update workflow title and overview
**File**: `.github/workflows/test.yml`
**Lines**: 1-10

```yaml
# Before
# =============================================================================
# CI Workflow: SQLite-First Testing Strategy
# =============================================================================
#
# Tests CrewChief packages with focus on SQLite as primary backend.
# PostgreSQL tests included for integration validation.

# After
# =============================================================================
# CI Workflow: SQLite-Only Testing Strategy
# =============================================================================
#
# Tests CrewChief packages using SQLite as the only database backend.
# PostgreSQL support was intentionally removed for simplicity.
```

### 2. Update DATABASE BACKENDS section
**File**: `.github/workflows/test.yml`
**Lines**: 11-14

```yaml
# Before
# DATABASE BACKENDS:
#   - SQLite (Default): Zero-configuration, runs without external services
#   - PostgreSQL (Integration): For team sharing/production validation

# After
# DATABASE BACKEND:
#   - SQLite (Only): Zero-configuration, runs without external services
#   - No PostgreSQL support (intentionally removed for simplicity)
```

### 3. Update JOB ORGANIZATION section
**File**: `.github/workflows/test.yml`
**Lines**: 15-23

```yaml
# Before
# JOB ORGANIZATION:
#   1. SQLite Tests (Primary) - Fast, no dependencies
#      - test-sqlite-e2e: CLI end-to-end tests
#      - test-mcp-sqlite: TypeScript MCP server tests
#      - test-rust-sqlite: Rust library tests
#
#   2. PostgreSQL Tests (Integration) - Requires service container
#      - test-postgres: TypeScript PostgreSQL integration
#      - test-rust-postgres: Rust PostgreSQL feature tests

# After
# JOB ORGANIZATION:
#   - test-sqlite-e2e: CLI end-to-end tests
#   - test-mcp-sqlite: TypeScript MCP server tests
#   - test-rust: Rust library tests
#   - test-typescript: TypeScript package tests (CLI, VSCode, daemon-client)
```

### 4. Simplify section divider
**File**: `.github/workflows/test.yml`
**Lines**: 63-65

```yaml
# Before
# =============================================================================
# SQLITE TESTS (Primary)
# =============================================================================

# After
# =============================================================================
# TEST JOBS
# =============================================================================
```

## Implementation Notes

**Documentation principles**:
1. **Be explicit**: "SQLite-Only" not "SQLite-First"
2. **Explain rationale**: Why PostgreSQL was removed (simplicity)
3. **Prevent confusion**: Clear that PostgreSQL is intentionally not supported
4. **Match reality**: Job list matches actual jobs in workflow

**What the comments should communicate**:
- SQLite is the only supported backend
- PostgreSQL was removed intentionally (not coming back)
- No service containers or external dependencies needed
- Simple, fast CI runs

## Dependencies
- Depends on: CICLEAN-1001 (PostgreSQL jobs removed)
- Depends on: CICLEAN-1002 (Rust job renamed)

## Risk Assessment

- **Risk**: Documentation out of sync with code
  - **Mitigation**: Update docs in same commit as code changes
  - **Impact**: Low (comments only, no functional change)
  - **Probability**: Very low (part of atomic change set)

- **Risk**: Misleading future contributors
  - **Mitigation**: Explicit "intentionally removed" language prevents confusion
  - **Impact**: Low (saves time investigating why PostgreSQL doesn't work)
  - **Probability**: Medium without this change (likely to cause confusion)

## Files/Packages Affected
- `.github/workflows/test.yml` - Update header comments (lines 1-23, 63-65)
