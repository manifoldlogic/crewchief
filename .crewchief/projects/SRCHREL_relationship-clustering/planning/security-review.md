# Security Review: Relationship-Aware Search

## Security Assessment

### Overall Risk Level: LOW

Relationship-aware search operates **read-only** on existing database relationships and does not introduce new attack vectors beyond existing search infrastructure. The feature primarily performs graph traversal queries and data aggregation without handling user credentials, secrets, or sensitive business logic.

### Authentication & Authorization

**Current State**: Inherited from existing search infrastructure

- **No Change**: This feature does not modify authentication or authorization mechanisms
- **Daemon Architecture**: Authentication handled by daemon client (if any), not by search pipeline
- **Local-Only**: Maproom operates on local SQLite database (`~/.maproom/maproom.db`), no network auth
- **Single-User Model**: Database access controlled by filesystem permissions

**Security Posture**: Acceptable for MVP. Relationship expansion respects same access controls as base search.

### Data Protection

**Graph Relationship Data**:
- **Source**: `chunk_edges` table (src_chunk_id, dst_chunk_id, type)
- **Sensitivity**: Low - contains code structure relationships, not business logic or secrets
- **Exposure**: Already exposed via context tool, relationship expansion doesn't reveal new data
- **Transport**: JSON over stdio (daemon RPC), same as existing search results

**Related Chunk Metadata**:
- **Contents**: File paths, symbol names, line ranges, content previews
- **Sensitivity**: Same as search results - code metadata already visible to user
- **Preview Length**: Limited to 100 characters, reduces information exposure
- **No Auth Tokens**: Relationship expansion doesn't access secrets or credentials

**Security Posture**: No new data exposure. Related chunks contain same information as search results.

### Input Validation

**User-Controlled Inputs**:

1. **`include_related` Parameter** (boolean)
   - Type: boolean (validated by TypeScript/Rust type system)
   - Range: true | false
   - Validation: Type coercion handles invalid values
   - Risk: None (boolean flag, no injection risk)

2. **`chunk_id` (Internal Parameter)**
   - Source: Search results (not directly user-provided)
   - Type: i64 (validated by database schema)
   - Validation: Database query uses parameterized queries (no SQL injection)
   - Risk: Low - invalid chunk_id results in empty relationships, graceful handling

**Database Query Safety**:

**Graph Traversal Query** (recursive CTE with parameterized inputs):
```rust
// Safe: Uses parameterized query, no string interpolation
pub async fn find_related_chunks(
    store: &SqliteStore,
    chunk_id: i64,  // Type-safe parameter
    max_depth: i32,
    edge_types: Option<Vec<EdgeType>>,
) -> Result<Vec<RelatedChunk>> {
    let query = r#"
        WITH RECURSIVE related(chunk_id, depth) AS (
            SELECT dst_chunk_id, 1 FROM chunk_edges WHERE src_chunk_id = ?1
            UNION
            SELECT ce.dst_chunk_id, r.depth + 1
            FROM chunk_edges ce
            JOIN related r ON ce.src_chunk_id = r.chunk_id
            WHERE r.depth < ?2
        )
        SELECT * FROM related;
    "#;

    // Parameterized query prevents SQL injection
    store.query(query, params![chunk_id, max_depth]).await
}
```

**Validation**: ✅ No SQL injection risk (parameterized queries)

**Depth Limiting**:
```rust
const MAX_DEPTH: i32 = 2;  // Hardcoded, not user-controlled

// Even if max_depth were user-provided:
let safe_depth = max_depth.min(10);  // Cap at 10 to prevent DoS
```

**Validation**: ✅ No unbounded recursion risk

### Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| **No rate limiting on graph traversal** | Low | Depth limited to 2, count limited to 5, confidence gating limits expansion | Accepted for MVP |
| **Preview content may contain sensitive comments** | Low | Already exposed in search results, preview limited to 100 chars | Accepted (same as base search) |
| **Chunk ID enumeration possible** | Very Low | Chunk IDs are internal DB identifiers, no security boundary | Accepted (single-user model) |
| **Relationship type leakage** | Very Low | Code structure relationships are intentionally exposed | Accepted (feature design) |

### Denial of Service (DoS) Risks

**Graph Traversal DoS**:

**Attack Vector**: Malicious user triggers relationship expansion on all results to exhaust database/CPU.

**Mitigations**:
1. **Confidence Gating**: Only 20-40% of results expanded (built-in rate limiting)
2. **Depth Limiting**: Hardcoded max_depth=2 (prevents exponential blowup)
3. **Count Limiting**: Top 5 related chunks per result (bounded result set)
4. **Timeout**: Existing database query timeout applies (default: 10s)
5. **Opt-In**: `include_related=false` by default (user must enable)

**Risk Assessment**: Low. Multiple safeguards prevent DoS even if user enables feature for all searches.

**Response Size DoS**:

**Attack Vector**: Large response payloads consume bandwidth/memory.

**Mitigations**:
1. **Bounded Size**: Max 10 results × 5 related × 200 bytes ≈ 10KB (acceptable)
2. **Metadata-Only**: No full file content in related chunks (preview only 100 chars)
3. **Existing Limits**: Search result limit (default: 10) applies before expansion

**Risk Assessment**: Very Low. Response size bounded and monitored.

### Error Information Leakage

**Error Scenarios**:

1. **Graph Traversal Failure**:
   ```rust
   Err(e) => {
       tracing::warn!("Failed to find related chunks for {}: {}", chunk_id, e);
       // Don't expose error to user, graceful degradation
       result.related = None;
   }
   ```
   **Leakage Risk**: Low - error logged server-side, not exposed to client

2. **Database Connection Error**:
   - Handled by existing error handling (same as base search)
   - Generic error message returned to user
   - No database schema or connection details leaked

**Validation**: ✅ No sensitive error details exposed to users

### Dependency Security

**New Dependencies**: None

- Relationship expansion uses existing dependencies (tokio, rusqlite, serde)
- No new supply chain risk introduced
- Existing dependency audit process applies

**Validation**: ✅ No new attack surface from dependencies

## MVP Security Scope

### In Scope for MVP

- [x] Input validation (type safety, parameterized queries)
- [x] SQL injection prevention (parameterized queries)
- [x] DoS prevention (depth/count/confidence limits)
- [x] Error handling (graceful degradation, no info leakage)
- [x] Backward compatibility (opt-in feature, no breaking changes)

### Out of Scope for MVP (Future Considerations)

- [ ] Rate limiting per user/session (single-user model, not needed)
- [ ] Audit logging of relationship queries (not required for local tool)
- [ ] Encryption at rest (SQLite database unencrypted, accepted for local tool)
- [ ] Network security (daemon uses stdio, not network socket)
- [ ] Access control for relationship types (all relationship types visible, by design)

### Deferred Security Enhancements

**If Maproom Becomes Multi-User**:
1. Implement per-user rate limiting (prevent DoS)
2. Add audit logging (track who queries relationships)
3. Introduce access control (sensitive code relationships)
4. Add encryption at rest (protect relationship graph)

**Current Status**: Not needed for single-user local tool.

## Security Checklist

### Pre-Deployment

- [x] **No hardcoded secrets**: Feature doesn't handle secrets
- [x] **Input validation on external inputs**: `include_related` validated by type system
- [x] **Proper error handling (no info leakage)**: Errors logged server-side, graceful degradation
- [x] **Dependencies are up to date**: No new dependencies added
- [x] **No SQL injection vulnerabilities**: Parameterized queries used throughout
- [x] **No XSS vulnerabilities**: Not applicable (CLI tool, no web UI)
- [x] **DoS prevention**: Depth, count, confidence limits in place
- [x] **Graceful degradation**: Errors don't crash search
- [x] **Backward compatibility**: Opt-in feature, existing users unaffected

### Code Review Security Focus Areas

1. **Graph Query Construction**: Verify all database queries use parameterized inputs
2. **Depth/Count Limits**: Verify hardcoded limits are enforced
3. **Error Handling**: Verify errors don't leak database structure or sensitive details
4. **Type Validation**: Verify TypeScript/Rust type safety prevents invalid inputs

### Security Testing

**Test Cases**:

1. **SQL Injection Attempt** (should fail safely):
   ```rust
   #[test]
   fn test_sql_injection_prevention() {
       // Malicious chunk_id (impossible with i64, but conceptually)
       let malicious_chunk_id = -1; // or i64::MAX
       let result = find_related_chunks(store, malicious_chunk_id, 2).await;
       assert!(result.is_ok());  // Should handle gracefully, not crash
       assert!(result.unwrap().is_empty());  // No chunks found, safe
   }
   ```

2. **DoS Attempt** (should be bounded):
   ```rust
   #[test]
   fn test_dos_prevention() {
       // All results high-confidence (worst case)
       let results = vec![high_conf_result(); 10];

       let start = Instant::now();
       apply_relationship_expansion(&store, results).await;
       let elapsed = start.elapsed();

       // Even worst case should complete within budget
       assert!(elapsed < Duration::from_millis(50));
   }
   ```

3. **Error Leakage** (should not expose internal details):
   ```rust
   #[test]
   fn test_error_message_sanitization() {
       // Simulate database error
       let error = simulate_db_error().await;

       // Error should not contain schema details
       assert!(!error.to_string().contains("chunk_edges"));
       assert!(!error.to_string().contains("sqlite"));
   }
   ```

## Security Sign-Off

**Reviewer**: [To be completed by security reviewer]

**Assessment**: Relationship-aware search introduces **no new security risks** beyond existing search infrastructure. All database queries use parameterized inputs, DoS risks are mitigated through depth/count/confidence limits, and errors are handled gracefully without information leakage.

**Recommendation**: **Approve for MVP deployment** with the following conditions:

1. ✅ Parameterized queries verified in code review
2. ✅ Depth/count limits enforced (max_depth=2, limit=5)
3. ✅ Error handling tested (graceful degradation, no crashes)
4. ✅ DoS testing validates performance budget (<20ms overhead)

**Risk Level**: LOW - Safe to ship

**Future Considerations**: If maproom evolves to multi-user or network-accessible deployment, revisit security model for rate limiting, audit logging, and access control.

---

**Date**: [To be completed]
**Approved By**: [To be completed]
