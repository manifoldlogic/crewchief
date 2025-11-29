# VECSTORE Security Review

## Scope

This project adds methods to the `VectorStore` trait and implements them in `PostgresStore` and `SqliteStore`. No new external interfaces or user-facing APIs are added—all changes are internal Rust code.

## Security Assessment

### SQL Injection

**Risk Level**: LOW

**Current Protections**:
- PostgreSQL: All queries use parameterized queries (`$1`, `$2`, etc.)
- SQLite: All queries use `params![]` macro for parameterized queries
- No string interpolation for user-provided values

**Evidence**:
```rust
// PostgreSQL (queries.rs)
client.query(
    "SELECT * FROM maproom.chunks WHERE symbol_name = $1",
    &[&symbol_name],  // Parameterized
).await?;

// SQLite (mod.rs)
conn.query_row(
    "SELECT * FROM chunks WHERE symbol_name = ?1",
    params![symbol_name],  // Parameterized
    |row| Ok(...)
)?;
```

**New Methods Risk**: All new methods follow the same pattern. Trait method signatures force parameters to be passed as typed values, not SQL strings.

**Mitigation**: Code review checklist includes SQL injection verification.

### Path Traversal

**Risk Level**: LOW

**Current Protections**:
- Database paths come from `MAPROOM_DATABASE_URL` environment variable
- SQLite file permissions set to 0600 on Unix
- No user-controlled file paths in database operations

**Evidence**:
```rust
// sqlite/mod.rs:73-80
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let db_path = std::path::Path::new(path);
    if db_path.exists() && !path.contains(":memory:") {
        std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600))?;
    }
}
```

**New Methods Risk**: No new file operations added. All methods operate on existing database connections.

### Denial of Service

**Risk Level**: MEDIUM

**Current Protections**:
- Connection pools limit concurrent connections (10 default)
- Query timeouts via PostgreSQL/SQLite settings
- Busy timeout for SQLite (5000ms)

**Potential Concerns**:
- Large batch operations could consume memory
- Complex graph traversals could be slow

**Mitigations Already in Place**:
```rust
// sqlite/mod.rs:61
PRAGMA busy_timeout = 5000;

// Pool limits
let pool = r2d2::Pool::builder()
    .max_size(10)
    .build(manager)?;
```

**New Methods Risk**:
- `batch_upsert_embeddings` already exists and handles batching
- New `get_chunk_context` method should limit surrounding chunks

**Recommendation**: Add depth limits to graph traversal queries (already implemented in sqlite/graph.rs with `max_depth = 10`).

### Information Disclosure

**Risk Level**: LOW

**Current Protections**:
- Database credentials in environment variables, not logged
- Error messages don't expose SQL queries
- Debug mode controlled by explicit flag

**Evidence**:
```rust
// queries.rs - errors use .context() without exposing values
client.execute(&sql, &[&code, &text, &chunk_id])
    .await
    .context("Failed to upsert embeddings")?;
```

**New Methods Risk**: New methods follow same error handling pattern.

### Authentication & Authorization

**Risk Level**: N/A

This project operates at the database layer. Authentication/authorization is handled by:
- PostgreSQL: Connection credentials
- SQLite: File system permissions

No changes to authentication model.

### Dependency Security

**Risk Level**: LOW

**Current Dependencies** (no new additions):
- `tokio-postgres`: Well-maintained, widely used
- `rusqlite`: Official SQLite bindings, well-maintained
- `r2d2`/`deadpool`: Established connection pool libraries
- `pgvector`: Vector extension, maintained

**New Dependencies**: None required for this project.

## Known Security Gaps

### Gap 1: SQLite Database Encryption

**Status**: DEFERRED (documented limitation)

SQLite databases are stored unencrypted. For local development use case, this is acceptable. For production/sensitive data, users should:
- Use PostgreSQL with TLS
- Or apply filesystem-level encryption

**Recommendation**: Document in CLAUDE.md that SQLite is for development/local use.

### Gap 2: Network Security

**Status**: OUT OF SCOPE

PostgreSQL connections use environment-configured settings. TLS configuration is a deployment concern, not a code concern.

**Recommendation**: Document TLS configuration in deployment docs (SQLINFRA project).

### Gap 3: Input Size Limits

**Status**: PARTIALLY ADDRESSED

Large inputs could cause memory issues:
- Embeddings: 1536 floats × 4 bytes = 6KB each
- Batch operations: Could be thousands of embeddings

**Current Mitigations**:
- Indexer batches at ~100 chunks
- No external API accepts arbitrary batch sizes

**Recommendation**: Add documentation about recommended batch sizes.

## Security Checklist for Implementation

Each ticket should verify:

- [ ] All SQL uses parameterized queries
- [ ] No string interpolation with user input
- [ ] Error messages don't expose sensitive data
- [ ] New file paths (if any) are validated
- [ ] Batch operations have reasonable size limits
- [ ] Tests include edge cases (empty input, special characters)

## Compliance Considerations

### OWASP Top 10 (2021)

| Risk | Status | Notes |
|------|--------|-------|
| A01: Broken Access Control | N/A | No access control layer |
| A02: Cryptographic Failures | Low | No crypto operations |
| A03: Injection | Low | Parameterized queries |
| A04: Insecure Design | Low | Follows Rust safety patterns |
| A05: Security Misconfiguration | Low | Minimal configuration |
| A06: Vulnerable Components | Low | Well-maintained deps |
| A07: Auth Failures | N/A | No auth layer |
| A08: Data Integrity Failures | Low | Database handles integrity |
| A09: Logging Failures | Low | Tracing in place |
| A10: SSRF | N/A | No network requests |

## Approval

This project is **APPROVED** for implementation with the following conditions:

1. All SQL must use parameterized queries (code review required)
2. No new dependencies without security review
3. Document SQLite encryption limitation in user-facing docs
4. Include edge case tests (empty input, special characters)

**Security Review Date**: 2025-11-26
**Reviewer**: AI Agent (technical-researcher pattern)
