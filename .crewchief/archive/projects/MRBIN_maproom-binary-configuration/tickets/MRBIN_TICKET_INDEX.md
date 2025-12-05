# MRBIN Ticket Index

Project: Maproom Binary Configuration
Total Tickets: 8 (4 Phase 1, 2 Phase 2, 2 Phase 3)

## Phase 1: Configuration Foundation (4 tickets)

Foundation work to add config schema, shared utility, tests, and async conversion without changing existing behavior.

### MRBIN-1001: Add maproomBinaryPath to Config Schema
- **File**: `MRBIN-1001_config-schema-extension.md`
- **Agent**: typescript-engineer
- **Scope**: 2 hours
- **Summary**: Extend RepositorySchema with optional maproomBinaryPath field
- **Deliverable**: Config schema accepts maproomBinaryPath, backwards compatible
- **Dependencies**: None

### MRBIN-1002: Implement Shared Binary Resolution Utility
- **File**: `MRBIN-1002_shared-binary-utility.md`
- **Agent**: typescript-engineer
- **Scope**: 3-4 hours
- **Summary**: Create packages/cli/src/utils/maproom-binary.ts with resolution logic
- **Deliverable**: Utility implements env > config > global > packaged priority
- **Dependencies**: MRBIN-1001

### MRBIN-1003: Unit Tests for Binary Resolution
- **File**: `MRBIN-1003_unit-tests.md`
- **Agent**: typescript-engineer
- **Scope**: 2-3 hours
- **Summary**: Comprehensive unit tests for findMaproomBinary() utility
- **Deliverable**: 90%+ coverage, all precedence paths tested
- **Dependencies**: MRBIN-1002

### MRBIN-1004: Convert Maproom Action Handlers to Async
- **File**: `MRBIN-1004_async-conversion.md`
- **Agent**: typescript-engineer
- **Scope**: 1-2 hours
- **Summary**: Make maproom.ts action handlers async for config loading
- **Deliverable**: All action handlers async, existing commands work
- **Dependencies**: None (can run parallel with 1001-1003)

## Phase 2: CLI Integration (2 tickets)

Integrate shared utility into CLI commands and remove duplicated code.

### MRBIN-2001: Refactor maproom.ts to Use Shared Utility
- **File**: `MRBIN-2001_refactor-maproom-ts.md`
- **Agent**: typescript-engineer
- **Scope**: 2-3 hours
- **Summary**: Replace resolvePackagedMaproomBin() with shared utility
- **Deliverable**: Config-based resolution, improved error messages, ~60 lines removed
- **Dependencies**: MRBIN-1001, MRBIN-1002, MRBIN-1003, MRBIN-1004

### MRBIN-2002: Refactor worktrees.ts to Use Shared Utility
- **File**: `MRBIN-2002_refactor-worktrees-ts.md`
- **Agent**: typescript-engineer
- **Scope**: 1-2 hours
- **Summary**: Replace inline resolution in runMaproomScan() with shared utility
- **Deliverable**: Consistent resolution logic, ~40 lines removed
- **Dependencies**: MRBIN-2001

## Phase 3: Documentation and Validation (2 tickets)

Complete documentation and validate full implementation.

### MRBIN-3001: Update Documentation
- **File**: `MRBIN-3001_documentation.md`
- **Agent**: typescript-engineer
- **Scope**: 2-3 hours
- **Summary**: Document config option, priority order, examples, migration guide
- **Deliverable**: README.md, local-development.md updated with examples
- **Dependencies**: MRBIN-2001, MRBIN-2002

### MRBIN-3002: Integration Validation
- **File**: `MRBIN-3002_integration-validation.md`
- **Agent**: typescript-engineer
- **Scope**: 2-3 hours
- **Summary**: Manual testing with real configs across all scenarios
- **Deliverable**: All 6 manual test scenarios validated, Windows tested
- **Dependencies**: MRBIN-3001

## Execution Order

### Critical Path
1. MRBIN-1001 (Schema) - Foundation
2. MRBIN-1004 (Async) - Can run parallel with 1002
3. MRBIN-1002 (Utility) - Core implementation
4. MRBIN-1003 (Tests) - Validation of utility
5. MRBIN-2001 (maproom.ts) - First consumer
6. MRBIN-2002 (worktrees.ts) - Second consumer
7. MRBIN-3001 (Docs) - User-facing
8. MRBIN-3002 (Validation) - Final verification

### Parallel Opportunities
- MRBIN-1004 can run parallel with MRBIN-1001/1002
- MRBIN-2001 and MRBIN-2002 must run sequentially (validate pattern with first, then apply to second)

## Summary Statistics

**Total Estimated Time**: 16-22 hours (2 days with buffer)

**By Phase**:
- Phase 1: 8-11 hours (foundation)
- Phase 2: 3-5 hours (integration)
- Phase 3: 4-6 hours (docs + validation)

**By Activity**:
- Implementation: 11-16 hours
- Testing: 4-6 hours
- Documentation: 2-3 hours
- Validation: 2-3 hours

**Files Modified**:
- New files: 2 (utility + tests)
- Modified files: 3 (schema, maproom.ts, worktrees.ts)
- Documentation: 3 (README, local-dev, migration notes)

**Code Changes**:
- Lines added: ~150 (utility + tests + schema)
- Lines removed: ~100 (duplicated resolution logic)
- Net change: +50 lines with significantly improved maintainability
