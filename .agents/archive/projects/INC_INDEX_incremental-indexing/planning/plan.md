# INC_INDEX Plan: Incremental Indexing Pipeline

## Project Overview
Implement real-time incremental indexing with file watching, enabling instant updates as code changes occur across multiple worktrees.

## Phase 1: Change Detection (Week 1)

**Agent: rust-indexer-engineer**

### Tasks
1. **File Hashing System**
   - Implement blake3 content hashing
   - Create hash cache management
   - Add database hash storage
   - Build comparison logic

2. **Change Detection API**
   - Detect new files
   - Identify modified files
   - Handle deleted files
   - Track file moves/renames

**Acceptance Criteria:**
- [ ] Hash generation <10ms per file
- [ ] Accurate change detection
- [ ] Hash cache working
- [ ] Database integration complete

## Phase 2: File Watching (Week 2)

**Agent: rust-indexer-engineer**

### Tasks
1. **Watcher Implementation**
   - Integrate notify crate
   - Set up event handling
   - Add debouncing logic
   - Implement ignore patterns

2. **Multi-Worktree Support**
   - Watch multiple paths
   - Isolate worktree events
   - Handle concurrent watchers
   - Manage watcher lifecycle

**Acceptance Criteria:**
- [ ] File changes detected <2s
- [ ] CPU usage <1% idle
- [ ] Multiple watchers work
- [ ] Ignore patterns respected

## Phase 3: Update Processing (Week 3)

**Agent: rust-indexer-engineer + database-engineer**

### Tasks
1. **Update Queue**
   - Priority queue implementation
   - Task deduplication
   - Batch processing
   - Error handling

2. **Incremental Processing**
   - Update single files
   - Maintain transaction integrity
   - Update edges correctly
   - Handle failures gracefully

**Acceptance Criteria:**
- [ ] Queue processes correctly
- [ ] Updates atomic
- [ ] Edges maintained
- [ ] Failures don't corrupt data

## Phase 4: Integration (Week 4)

**Agent: rust-indexer-engineer + integration-tester**

### Tasks
1. **Watch Command**
   - CLI command implementation
   - Configuration loading
   - Status reporting
   - Graceful shutdown

2. **Testing & Validation**
   - File change scenarios
   - Concurrent updates
   - Large batch updates
   - Failure recovery

**Acceptance Criteria:**
- [ ] Watch command works
- [ ] All tests passing
- [ ] Performance targets met
- [ ] Documentation complete

## Success Metrics
- Change detection <2s
- Update processing <5s
- CPU usage <1% idle
- Memory <50MB overhead
- Zero data corruption

## Risk Mitigation
- Fallback to full scan
- Periodic reconciliation
- Transaction rollback
- Dead letter queue