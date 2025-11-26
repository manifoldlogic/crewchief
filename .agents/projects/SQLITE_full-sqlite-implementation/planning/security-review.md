# Security Review: Full SQLite Implementation

## Scope

This review covers security considerations for the SQLite-based code indexing and search system. The system stores and indexes source code locally - there is no network component or multi-user access.

## Threat Model

### Assets to Protect
1. **Source code** - Indexed code chunks and previews
2. **Embeddings** - Vector representations of code (could reveal structure)
3. **Metadata** - File paths, symbol names, repository information

### Threat Actors
1. **Local malware** - Processes with file system access
2. **Accidental exposure** - Database file shared unintentionally
3. **Unauthorized users** - Other users on shared systems

### Out of Scope
- Network attacks (no network component)
- Remote code execution (local CLI tool)
- Authentication/authorization (single-user system)

## Security Analysis

### 1. File System Security

**Risk**: Database file readable by other users

**Current Mitigation** (from SQLFIX):
```rust
// crates/maproom/src/db/sqlite/mod.rs
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
}
```

**Assessment**: Adequate for MVP
- Unix permissions set to owner-only (600)
- WAL and SHM files inherit directory permissions
- Windows relies on NTFS ACLs (user responsibility)

**Recommendation**: Document that users should ensure `~/.maproom/` directory has appropriate permissions.

### 2. SQL Injection

**Risk**: Malicious input in search queries could execute unintended SQL

**Mitigations in Place**:
```rust
// Parameterized queries throughout
conn.execute(
    "INSERT INTO repos (name, root_path) VALUES (?1, ?2)",
    params![name, root],
)?;

// FTS5 queries sanitize special characters
let clean = term
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "");
```

**Assessment**: Low risk
- All queries use parameterized statements
- FTS5 input is sanitized before building MATCH clause
- rusqlite prevents multi-statement execution by default

**Recommendation**: Add explicit tests for SQL injection attempts in queries.

### 3. Path Traversal

**Risk**: Malicious file paths could access files outside intended directories

**Current State**:
- File paths stored as-is from indexer
- No validation that paths are within repository root

**Assessment**: Low risk for this component
- The indexer (separate component) controls what paths are submitted
- Database stores whatever paths it receives
- Display/export would need path validation

**Recommendation**: Document that path validation is the indexer's responsibility, not the database layer.

### 4. Extension Loading (sqlite-vec)

**Risk**: Loading untrusted extensions could execute arbitrary code

**Current State**:
```rust
// Extension is BUNDLED and loaded via sqlite3_auto_extension FFI
// See: crates/maproom/src/db/sqlite/mod.rs
unsafe {
    sqlite3_auto_extension(Some(std::mem::transmute(
        sqlite3_vec_init as *const (),
    )));
}
```

**Assessment**: Low risk - extension is bundled
- sqlite-vec is compiled into the binary, not loaded from filesystem
- Extension source is verified at build time via Cargo.toml
- No runtime loading from untrusted paths

**Mitigations already in place**:
1. ✅ Extension bundled with application (not loaded from system path)
2. ✅ Built from crates.io verified source
3. Runtime verification added in Phase -1 (extension verification ticket)

**Graceful Degradation**:
- If sqlite-vec functions unavailable: fall back to FTS-only search mode
- Clear error message logged, no crash
- Vector search methods return empty results with warning

**Recommendation for MVP**: Bundle is sufficient. Add runtime verification to detect corrupted builds.

### 5. Data at Rest

**Risk**: Sensitive code exposed if database file is stolen/copied

**Current State**: No encryption

**Assessment**: Acceptable for MVP
- This is a developer tool for local use
- Source code is already on disk in plaintext
- Database contains derived data (chunks, embeddings)

**Enterprise Consideration** (future):
- SQLite Encryption Extension (SEE) for encrypted databases
- Or application-level encryption of sensitive columns

**Recommendation**: Document that database contains code snippets and should be treated as sensitive as source code.

### 6. Denial of Service

**Risk**: Malformed queries or large datasets could exhaust resources

**Mitigations**:
```rust
// Busy timeout prevents indefinite hangs
PRAGMA busy_timeout = 5000;

// Connection pool limits concurrent access
pool_size: 10

// Query limits prevent unbounded results
LIMIT ?limit
```

**Assessment**: Low risk
- Local tool, attacker would need local access
- SQLite handles resource limits well
- Pool prevents connection exhaustion

### 7. Information Disclosure

**Risk**: Error messages expose sensitive information

**Current State**:
```rust
// Errors propagate through anyhow
anyhow::bail!("Repository not found: {}", name);
```

**Assessment**: Low risk for local CLI
- Error messages include repository names and paths
- Acceptable for developer-facing tool
- Would need sanitization for user-facing errors

### 8. Integrity

**Risk**: Database corruption from crashes or concurrent access

**Mitigations**:
```sql
PRAGMA journal_mode = WAL;      -- Write-ahead logging
PRAGMA synchronous = NORMAL;    -- Reasonable durability
PRAGMA foreign_keys = ON;       -- Referential integrity
```

**Assessment**: Adequate
- WAL mode provides crash recovery
- Foreign keys prevent orphaned records
- SQLite is well-tested for integrity

## Security Checklist

### Must Have (MVP)
- [x] Parameterized queries for all SQL
- [x] FTS5 query sanitization
- [x] File permissions on Unix (0600)
- [x] Connection pool limits
- [x] Query result limits
- [x] WAL mode for integrity

### Should Have (Post-MVP)
- [ ] Document security considerations for users
- [ ] SQL injection test cases
- [ ] Extension integrity verification
- [ ] Windows file permission guidance

### Nice to Have (Enterprise)
- [ ] Database encryption option
- [ ] Audit logging
- [ ] Fine-grained access control

## Conclusion

The SQLite implementation has **acceptable security posture for an MVP**:

1. **No critical vulnerabilities** - Parameterized queries, proper file permissions
2. **Low risk profile** - Local-only, single-user, no network exposure
3. **Known limitations documented** - No encryption, relies on file system security

The main security concern is ensuring the database file is protected at the file system level, which is the user's responsibility (same as protecting their source code).

## References

- [SQLite Security](https://sqlite.org/security.html)
- [rusqlite Security Considerations](https://docs.rs/rusqlite/)
- [OWASP SQL Injection Prevention](https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html)
