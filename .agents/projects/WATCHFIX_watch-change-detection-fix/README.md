# WATCHFIX: Watch Change Detection Fix

## Problem

The maproom `watch` command has a critical bug in its file change detection logic that prevents modified files from being correctly re-indexed:

**Symptoms**:
- Modified files misclassified as NEW files instead of MODIFIED
- Indexing fails with "File not found in database" errors
- Files enter infinite retry loops
- Database never updated despite file changes
- All files detected but none successfully processed

**Impact**:
- Watch command is currently broken for file modifications
- Users must use `scan` or `upsert` instead (slow for development workflow)
- Real-time code search index becomes stale

**Root Cause**:
- Path normalization mismatch between file watcher (absolute paths) and database (relative paths)
- `get_file_id_by_path()` fails to find existing files
- Falls through to "new file" branch incorrectly
- `index_new_file()` expects file record to not exist, but it does
- Transaction fails, file re-queued forever

## Solution

Fix the watch command's `processor_task` to:

1. **Normalize paths correctly** - Create utility to convert absolute → relative paths
2. **Always use ChangeDetector** - Call `detect_change()` for Modified events with valid file_id
3. **Fix IncrementalProcessor** - Use relative paths for database queries, absolute for filesystem
4. **Add safety checks** - File size limits, path validation, error handling

**Key Changes**:
- New module: `crates/maproom/src/incremental/path_utils.rs`
- Refactor: `crates/maproom/src/indexer/mod.rs` (processor_task)
- Fix: `crates/maproom/src/incremental/processor.rs` (index_new_file, update_file)

**Result**: Watch command correctly detects and re-indexes modified files with updated database timestamps.

## Success Criteria

- [x] Bug reproduced and root cause identified ✅
- [ ] Modified files classified as `ChangeType::Modified` (not New)
- [ ] Multiple files changed simultaneously all get indexed
- [ ] Database timestamps and content updated correctly
- [ ] No infinite retry loops
- [ ] Performance: < 1s per file
- [ ] All tests pass (unit + integration + E2E)
- [ ] No regressions in scan/upsert commands

## Project Structure

```
.agents/projects/WATCHFIX_watch-change-detection-fix/
├── README.md                    # This file
├── planning/
│   ├── analysis.md              # Deep investigation of the bug
│   ├── architecture.md          # Solution design and data flow
│   ├── plan.md                  # Phases, deliverables, timeline
│   ├── quality-strategy.md      # Testing strategy
│   ├── security-review.md       # Security considerations
│   └── agent-suggestions.md     # Agent assignments
└── tickets/
    ├── WATCHFIX-1001_path-normalization-utility.md
    ├── WATCHFIX-1002_processor-task-refactor.md
    ├── WATCHFIX-1003_processor-path-handling.md
    ├── WATCHFIX-1004_security-performance.md
    ├── WATCHFIX-1005_integration-testing.md
    └── WATCHFIX-1006_documentation.md
```

## Relevant Agents

### Primary Implementation
- **rust-indexer-engineer** - Implements all fixes in crates/maproom/

### Standard Workflow
- **unit-test-runner** - Executes tests, reports results (no fixes)
- **verify-ticket** - Verifies acceptance criteria met
- **commit-ticket** - Creates Conventional Commits

## Planning Documents

### [Analysis](./planning/analysis.md)
Comprehensive investigation of the bug:
- Evidence from test scenario (3 files modified)
- Log analysis showing misclassification
- Database queries proving no indexing occurred
- Path format inconsistencies identified
- Root cause traced to processor_task lines 678-694
- Industry solutions reviewed
- Complexity assessment

**Key findings**:
- Detection works ✅
- Change detection breaks ❌
- Path mismatch causes file_id lookup failure
- Falls through to wrong code branch
- IncrementalProcessor compounds the problem

### [Architecture](./planning/architecture.md)
Solution design and implementation details:
- Path normalization strategy (absolute → relative)
- processor_task refactoring (pseudocode)
- IncrementalProcessor path handling
- Data flow diagrams
- Edge case handling
- Performance analysis
- Testing approach

**Key design decisions**:
- Single path normalization function
- Use existing ChangeDetector (don't bypass)
- Minimal API changes
- Transaction integrity maintained

### [Plan](./planning/plan.md)
Phases, deliverables, and timeline:
- **Phase 1**: Path Normalization Utility (4h)
- **Phase 2**: processor_task Refactoring (6h)
- **Phase 3**: Processor Path Handling (4h)
- **Phase 4**: Security & Performance (2h)
- **Phase 5**: Integration Testing (8h)
- **Phase 6**: Documentation & Polish (2h)

**Total effort**: 26 hours over 2 weeks

### [Quality Strategy](./planning/quality-strategy.md)
Pragmatic testing approach:
- 70% unit tests, 25% integration, 5% E2E
- 100% coverage for path normalization (critical)
- 90% coverage for change detection logic
- Integration tests for multi-file scenario
- Manual regression testing

**Test pyramid**: 10-15 unit, 3-5 integration, 1-2 E2E

### [Security Review](./planning/security-review.md)
Security assessment and mitigations:
- **Risk level**: LOW (local development tool)
- Path traversal protection (strip_prefix + validation)
- SQL injection prevention (parameterized queries)
- DoS mitigation (file size limits)
- Symlink handling (log warnings, allow)

**Verdict**: No security blockers, ready to implement

### [Agent Suggestions](./planning/agent-suggestions.md)
No new agents needed:
- rust-indexer-engineer (implementation)
- unit-test-runner (testing)
- verify-ticket (verification)
- commit-ticket (commits)

**Rationale**: Existing agents have all required capabilities

## Timeline

**Estimated**: 2 weeks (26 hours)

**Week 1**:
- Days 1-2: Path normalization utility + tests
- Days 3-4: processor_task refactoring
- Day 5: Processor path handling

**Week 2**:
- Days 1-2: Security/performance + integration tests
- Days 3-4: Integration testing
- Day 5: Documentation + manual testing

## Next Steps

1. **Create tickets**: Run `/create-project-tickets WATCHFIX`
2. **Review tickets**: Run `/review-tickets WATCHFIX`
3. **Execute project**: Run `/work-on-project WATCHFIX`

Or work tickets individually:
- `/single-ticket WATCHFIX-1001` (path normalization)
- `/single-ticket WATCHFIX-1002` (processor_task)
- ... and so on

## Evidence of Bug

### Test Scenario
Modified 3 TypeScript files simultaneously:
- `packages/cli/src/agents/discovery.ts`
- `packages/cli/src/agents/registry.ts`
- `packages/cli/src/agents/runner.ts`

### Results
```
Files detected: 3 ✅
Files enqueued: 3 ✅
Files processed: 3 ✅
Files indexed: 0 ❌

Error: "Failed to index new file" (all 3 files)
Retry loops: 3 files × 3 attempts = 9 total
Database updates: 0 (timestamps unchanged)
```

### Log Evidence
```
2025-11-05T16:12:45.913523Z DEBUG Processing update task
  path=/workspace/packages/cli/src/agents/discovery.ts
  change_type=New(Hash("c380ead..."))  # ❌ WRONG - should be Modified

2025-11-05T16:12:45.914327Z WARN Failed to process file
  path=/workspace/packages/cli/src/agents/discovery.ts
  error=Failed to index new file: /workspace/packages/cli/src/agents/discovery.ts
```

## Context

This bug was discovered during investigation of watch command behavior. The MRPROG project (Maproom Progress UX) specifically avoided touching watch command due to its architectural complexity.

Now we're addressing the actual logic bug that causes the symptoms they observed.

## Related Projects

- **MRPROG** (Maproom Progress UX) - Avoided watch command, cited complexity
- Current codebase has working ChangeDetector and IncrementalProcessor
- Bug is isolated to watch command's processor_task

## Status

📋 **Planning Complete** - Ready for ticket creation and implementation

---

**Project**: WATCHFIX (Watch Change Detection Fix)
**Slug**: WATCHFIX
**Created**: 2025-11-05
**Status**: Planning Complete
**Next Action**: Create tickets with `/create-project-tickets WATCHFIX`
