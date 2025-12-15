# Security Review: edge extraction

## Security Assessment

**Risk Level:** LOW

Edge extraction is a server-side indexing optimization with no new external inputs, authentication, or data exposure. The primary risk is performance DoS from malicious files.

### Authentication & Authorization

**Scope:** Not applicable (server-side indexing only)

**Rationale:** Edge extraction runs during `scan` and `upsert` commands, which are server-side operations. No new API endpoints, no user input, no authentication changes.

### Data Protection

**Scope:** Read-only file access, database writes

**Protection Measures:**
- File content read-only (no modifications)
- Database writes use parameterized queries (SQL injection-safe)
- No sensitive data stored in edges (only chunk IDs and edge types)

**Data Flow:**
```
File on disk (read-only)
  ↓
Tree-sitter parse (in-memory)
  ↓
Symbol resolution (in-memory + database lookup)
  ↓
Edge insertion (parameterized SQL)
```

**No new data exposure:** Edges are internal relationships, not exposed to users directly.

### Input Validation

**File Content:**
- Parsed by tree-sitter (trusted library)
- Parse failures handled gracefully (log warning, continue)
- No execution of user code

**Database Inputs:**
- Chunk IDs from database (trusted source)
- Symbol names from parser (sanitized by tree-sitter)
- Edge types from enum (hardcoded values)

**Parameterized Queries:**
```rust
INSERT OR IGNORE INTO chunk_edges (src_chunk_id, dst_chunk_id, type)
VALUES (?1, ?2, ?3)
```

No string interpolation, no SQL injection risk.

## Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| Performance DoS (malicious files with 10,000+ call expressions) | Low | Warn if >1000 edges per file, skip files >10,000 lines (existing limit) | Mitigated |
| Symbol name collisions (multiple chunks with same name) | Low | Accept spurious edges, log at trace level | Accepted |
| Tree-sitter vulnerabilities | Low | Use released versions, update regularly | Accepted |
| No edge verification (incorrect edges inserted) | Low | Accuracy testing, manual spot-checks | Accepted for MVP |

## MVP Security Scope

**In Scope:**
- Read-only file operations
- Parameterized SQL queries
- No new authentication/authorization
- No new data exposure

**Out of Scope (Future):**
- Edge confidence scores (could reduce spurious edges)
- Edge verification (prevent incorrect relationships)
- Rate limiting (not needed for server-side operations)

## Security Checklist

- [x] No hardcoded secrets (not applicable)
- [x] Input validation on external inputs (file content parsed by tree-sitter)
- [x] Proper error handling (parse failures logged, no stack traces to users)
- [x] Dependencies are up to date (tree-sitter latest stable)
- [x] No SQL injection vulnerabilities (parameterized queries only)
- [x] No XSS vulnerabilities (not applicable - server-side only)
- [x] No file path traversal (uses existing file scanning logic)
- [x] No arbitrary code execution (only parses files, does not execute)

## Performance DoS Mitigation

**Attack Vector:** Malicious file with excessive call expressions to slow indexing.

**Example:**
```typescript
// Malicious file with 10,000 calls
function foo() {}
function bar() {
    foo(); foo(); foo(); // ... 10,000 times
}
```

**Mitigations:**
1. **Existing:** Skip files >10,000 lines (hard limit in scanner)
2. **Existing:** Tree-sitter parse timeout (30 seconds)
3. **New:** Warn if >1000 edges per file
4. **New:** Skip edge extraction if call_expression count >5000

**Implementation:**
```rust
fn extract_calls(source: &str, chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    let tree = parser.parse(source, None)?;
    let call_nodes = find_call_expressions(tree.root_node());

    if call_nodes.len() > 5000 {
        warn!("File has {} call expressions, skipping edge extraction", call_nodes.len());
        return Ok(Vec::new());
    }

    // Continue with extraction...
}
```

## Data Integrity

**Risk:** Incorrect edges pollute database, degrade search quality.

**Mitigation:**
- Accuracy testing (≥70% precision)
- Manual spot-checks on known repositories
- Logging unresolved calls (trace level)
- Deletion logic in EdgeUpdater (can clean up bad edges)

**Recovery:** If edges are incorrect:
1. Delete all edges: `DELETE FROM chunk_edges;`
2. Rescan repository with fixed logic
3. Edges repopulated correctly

## Dependency Security

**Tree-Sitter:**
- Version: Latest stable (specified in Cargo.toml)
- Source: Official repository (https://github.com/tree-sitter/tree-sitter)
- Risk: Parsing library vulnerabilities
- Mitigation: Update regularly, monitor security advisories

**Database:**
- SQLite: Trusted, widely used
- sqlite-vec: Statically linked (no external dependencies)
- Parameterized queries: SQL injection-safe

## Approved for MVP

**Security posture:** LOW risk, acceptable for MVP deployment.

**Rationale:**
- No new authentication or authorization
- No user-facing features (server-side only)
- Read-only file operations
- Parameterized database queries
- Performance DoS mitigated
- Dependencies trusted and up-to-date

**Post-MVP Security Improvements:**
- Edge confidence scores (reduce spurious edges)
- Cross-file resolution validation (prevent incorrect links)
- Monitoring for edge count anomalies
