# INC_INDEX Analysis: Incremental Indexing Pipeline

## Problem Space

### Current Limitations
Maproom currently only supports full repository scanning, requiring complete re-indexing even for single file changes. This leads to:
- **Slow feedback loops**: Minutes to index after each change
- **Resource waste**: Re-processing unchanged files
- **Agent delays**: Waiting for index updates
- **Stale search results**: Gap between changes and searchability

### Industry Context

Leading solutions achieve near-instant incremental updates:
- **GitHub Copilot**: 60-second max indexing (previously 5 minutes)
- **Cursor**: Merkle tree sync with 10-minute cycles
- **IntelliJ IDEA**: 57% reduction in indexing time
- **Meta Glean**: O(changes) complexity, not O(repository)

Key pattern: Track changes efficiently, update only what's needed.

### Current State
- Full scan implemented (`crewchief maproom scan`)
- Basic upsert for specific files
- No file watching capability
- No change detection system
- No incremental edge updates

## Key Insights

### 1. File Hashing is Essential
Content-based hashing (blake3, SHA-256) enables:
- Quick change detection
- Deduplication across worktrees
- Cache key generation
- Incremental updates

### 2. Watch vs Poll Trade-offs
- **File watching**: Instant but complex, OS-specific
- **Polling**: Simple but delayed, resource usage
- **Hybrid**: Watch with periodic reconciliation

### 3. Edge Updates are Complex
When a file changes, must update:
- Chunks from that file
- Edges from/to those chunks
- Dependent relationships
- Test associations

### 4. Worktree Isolation Critical
Each worktree needs independent indexing:
- Agents work in isolation
- Different branches/commits
- Parallel development
- No interference

## Success Criteria

### Functional Requirements
- [ ] Detect file changes within 2 seconds
- [ ] Update only changed files
- [ ] Maintain relationship integrity
- [ ] Support multiple worktrees
- [ ] Handle file moves/renames

### Performance Requirements
- [ ] Incremental update <5s for 10 files
- [ ] Watch mode <1% CPU usage
- [ ] Memory usage <50MB overhead
- [ ] Support 10+ concurrent watchers

## Risk Assessment

### Technical Risks
1. **File system events unreliable**
   - Mitigation: Periodic reconciliation
2. **Race conditions**
   - Mitigation: Locking, queuing
3. **Memory leaks in watchers**
   - Mitigation: Restart periodically

## Recommendations

### MVP Scope
- Simple file watching
- Hash-based change detection
- Queue-based processing
- Basic incremental updates

### Production Scope
- Merkle tree optimization
- Parallel processing
- Smart edge updates
- Git integration