# Security Review: MAPCLI - Maproom CLI Abstraction

## Scope

This review covers security implications of:
1. Adding SQLite backend support to CLI/daemon
2. Changing database connection handling
3. Runtime backend selection

## Architecture Security Analysis

### 1. Database URL Handling

**Current**: `MAPROOM_DATABASE_URL` environment variable

**Concern**: URL may contain credentials for PostgreSQL

**Assessment**: LOW RISK
- This is existing behavior, not changed by MAPCLI
- SQLite URLs don't contain credentials
- Standard pattern for database configuration

**Recommendation**: No changes needed. Document that PostgreSQL URLs with passwords should use environment variables rather than command-line flags.

### 2. SQLite File Permissions

**Concern**: SQLite database file created with insecure permissions

**Assessment**: LOW RISK
- `rusqlite` creates files with 0644 permissions by default
- Database contains code index, not secrets
- Single-user tool, not multi-tenant

**Recommendation**: Document that SQLite databases should not be world-readable if containing sensitive code. Consider:
```rust
// Optional: Restrict permissions on Unix
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600))?;
}
```

**Action**: Not blocking for MVP. Can add in future enhancement.

### 3. Path Traversal in Database URL

**Concern**: Malicious `sqlite://` URL could write to sensitive locations

**Assessment**: LOW RISK
- User controls their own configuration
- No untrusted input to database URL
- Rust's file operations respect permissions

**Recommendation**: No changes needed. This is user-controlled configuration.

### 4. Daemon JSON-RPC Input Validation

**Current**: Daemon accepts JSON-RPC requests over stdin

**Concern**: Malicious requests could cause issues

**Assessment**: LOW RISK (no change from current)
- Daemon runs locally, not network-exposed
- Input is deserialized via serde with type validation
- VectorStore trait methods have bounded parameters

**Recommendation**: No changes needed. Current validation is adequate.

### 5. SQL Injection

**Concern**: User input in search queries could inject SQL

**Assessment**: LOW RISK
- All queries use parameterized statements
- VectorStore trait enforces parameter passing
- No string concatenation for SQL

**PostgreSQL Example** (existing):
```rust
client.query("SELECT ... WHERE name = $1", &[&user_input])
```

**SQLite Example** (implemented in VECSTORE):
```rust
conn.prepare("SELECT ... WHERE name = ?1")?.query(params![user_input])
```

**Recommendation**: No changes needed. Parameterized queries prevent injection.

### 6. Resource Exhaustion

**Concern**: Large search results or unbounded queries

**Assessment**: LOW RISK
- Search queries have `k` (limit) parameter
- Pagination prevents unbounded results
- Database operations have reasonable timeouts

**Recommendation**: Ensure all search methods respect the `k` limit parameter. Already implemented in VectorStore trait.

## Known Gaps (MVP-Acceptable)

### Gap 1: No Database Encryption

**Description**: SQLite databases are unencrypted by default

**Risk Level**: LOW for MVP
- Code indexes are not highly sensitive
- File permissions provide basic protection
- Full-disk encryption at OS level is common

**Enterprise Consideration**: Future enhancement could add SQLCipher support

### Gap 2: No Audit Logging

**Description**: No logging of database operations or queries

**Risk Level**: LOW for MVP
- Single-user development tool
- Not compliance-regulated
- Debug logging available via RUST_LOG

**Enterprise Consideration**: Add structured audit logging if needed

### Gap 3: No Rate Limiting

**Description**: Daemon doesn't rate-limit requests

**Risk Level**: LOW for MVP
- Runs locally, not network-exposed
- stdin/stdout transport inherently slow
- Single user

**Enterprise Consideration**: Add rate limiting if daemon exposed over network

## Security Checklist

### Pre-Implementation
- [x] No new external network communication
- [x] No new credential handling
- [x] No new file system access beyond database
- [x] Existing security patterns preserved

### Implementation
- [ ] Parameterized queries in all new database calls
- [ ] Input validation on CLI arguments
- [ ] Error messages don't leak sensitive info
- [ ] File paths validated before use

### Post-Implementation
- [ ] Review generated SQL for injection vulnerabilities
- [ ] Test with malformed input
- [ ] Verify error handling doesn't panic

## Threat Model

### Actors
1. **Local User**: Has shell access, runs CLI commands
2. **External Code**: Code being indexed (untrusted content)
3. **Network Attacker**: N/A - no network exposure

### Attack Vectors

| Vector | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| SQL Injection via search query | Low | Medium | Parameterized queries |
| Path traversal via db URL | Low | Low | User-controlled config |
| DoS via large scan | Low | Low | Existing resource limits |
| Malicious code in indexed files | Medium | Low | Read-only parsing |

### Trust Boundaries

```
┌─────────────────────────────────────────────┐
│           User's Machine (Trusted)           │
│  ┌─────────────────┐   ┌─────────────────┐  │
│  │   CLI Process   │   │  Indexed Code   │  │
│  │   (Trusted)     │   │  (Untrusted)    │  │
│  └────────┬────────┘   └────────┬────────┘  │
│           │                     │           │
│           ▼                     ▼           │
│  ┌─────────────────────────────────────┐   │
│  │         SQLite/PostgreSQL            │   │
│  │         (Trusted Storage)            │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

## Recommendations Summary

### Blocking (Must Fix Before Ship)
None identified.

### Non-Blocking (Should Address)
1. Document SQLite file permission considerations
2. Ensure all new database queries use parameters

### Future Enhancements (Enterprise)
1. SQLCipher support for encrypted databases
2. Audit logging for compliance
3. Rate limiting if daemon network-exposed

## Conclusion

The MAPCLI project introduces **no new security vulnerabilities** beyond existing code. The SQLite backend follows the same secure patterns as PostgreSQL:
- Parameterized queries
- Type-safe deserialization
- Local-only operation

**Security Approval**: ✅ Cleared for MVP implementation
