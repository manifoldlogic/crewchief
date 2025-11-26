# Security Review: SQLite Backend Fixes

## 1. Architecture Security Analysis

### 1.1 Attack Surface Comparison

| Surface | PostgreSQL Backend | SQLite Backend |
|---------|-------------------|----------------|
| Network | Docker port 5432 | None (file-based) |
| File access | Container isolated | Direct filesystem |
| Authentication | Username/password | Filesystem permissions |
| SQL injection | Parameterized queries | Parameterized queries |

**Assessment**: SQLite has a smaller attack surface (no network), but requires careful file permission management.

### 1.2 Data at Rest

**SQLite File**: `maproom.db` contains:
- Repository paths (potentially sensitive directory structure)
- Code snippets in chunks (IP exposure risk)
- Embedding vectors (model output, low sensitivity)

**Current State**: File created with default permissions (umask dependent)

**Recommendation**: Explicitly set 0600 on database file
```rust
// In SqliteStore::connect()
use std::os::unix::fs::PermissionsExt;
std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
```

## 2. Input Validation

### 2.1 SQL Injection Prevention

**Status**: ✅ Safe - All queries use parameterized statements

```rust
// Good (current implementation)
conn.execute(
    "INSERT INTO repos(name, root_path) VALUES (?1, ?2)",
    params![name, root_path],
)?;

// Never do this (not present in codebase)
conn.execute(
    &format!("SELECT * FROM repos WHERE name = '{}'", name),
    [],
)?;
```

### 2.2 Path Traversal

**Risk**: User-controlled paths could escape intended directories

**Current State**: `relpath` stored as-is in database

**Mitigation**: Path validation happens at indexer level, not storage level. Database stores what indexer provides.

**Recommendation**: No changes needed - trust boundary is at indexer.

## 3. Dependency Security

### 3.1 rusqlite

**Version**: 0.29.0
**Known vulnerabilities**: None in RUSTSEC database
**Features used**: `bundled` (includes SQLite source)

**Note**: `bundled` feature compiles SQLite from source, isolating from system SQLite vulnerabilities but requiring updates when SQLite patches are released.

### 3.2 sqlite-vec (Vendored C)

**Status**: C extension compiled from vendored source

**Risks**:
- Buffer overflows in C code
- Memory corruption
- No Rust safety guarantees

**Mitigations**:
1. Pin to specific commit (currently done)
2. Limited to vector operations (constrained functionality)
3. Input vectors validated for dimension before passing to C
4. sqlite-vec is gaining adoption and scrutiny

**Monitoring**: Watch sqlite-vec GitHub for security advisories

### 3.3 r2d2/r2d2_sqlite

**Status**: Pool library for connection management

**Risk**: Low - mature crate, doesn't handle untrusted input directly

## 4. Concurrency Safety

### 4.1 SQLite Locking

**Mode**: WAL (Write-Ahead Logging)

**Behavior**:
- Multiple readers allowed
- Single writer at a time
- Readers don't block writers (and vice versa in WAL mode)

**Risk**: `SQLITE_BUSY` errors under high contention

**Mitigation**:
```rust
// Set busy timeout
conn.execute("PRAGMA busy_timeout = 5000", [])?;  // 5 second timeout
```

### 4.2 Thread Safety

**Current**: Uses `r2d2` pool with `Send + Sync` bounds

**Rust guarantees**: Type system prevents data races

**Remaining risk**: Logic errors in transaction handling (mitigated by tests)

## 5. Known Gaps (Accepted for MVP)

### 5.1 No Encryption at Rest

**Status**: SQLite database is unencrypted

**Risk**: If attacker gains filesystem access, data is readable

**Mitigation options** (not implemented):
- SQLCipher (encrypted SQLite fork) - adds complexity
- Filesystem encryption - OS-level
- Don't store sensitive data - already the case (code is public anyway)

**Decision**: Accept for MVP. Code being indexed is typically not secret.

### 5.2 No Audit Logging

**Status**: No logging of database operations

**Risk**: Cannot detect unauthorized access

**Mitigation**: OS-level file access logging if needed

**Decision**: Accept for MVP. Single-user tool doesn't need audit trail.

## 6. Secure Defaults

### 6.1 Recommended PRAGMA settings

```sql
PRAGMA journal_mode = WAL;         -- Better concurrency, crash safety
PRAGMA synchronous = NORMAL;       -- Balance of safety and speed
PRAGMA foreign_keys = ON;          -- Enforce referential integrity
PRAGMA busy_timeout = 5000;        -- Prevent immediate SQLITE_BUSY
```

**Status**: WAL and foreign_keys already set. Add busy_timeout.

### 6.2 File Creation

```rust
// Ensure database directory exists with safe permissions
let db_dir = Path::new(path).parent().unwrap();
std::fs::create_dir_all(db_dir)?;
#[cfg(unix)]
std::fs::set_permissions(db_dir, std::fs::Permissions::from_mode(0o700))?;
```

## 7. Security Checklist

| Item | Status | Notes |
|------|--------|-------|
| Parameterized queries | ✅ | All SQL uses parameters |
| File permissions | ⚠️ | Need to set explicitly |
| Dependency audit | ✅ | No known vulnerabilities |
| Input validation | ✅ | Trust boundary at indexer |
| Error handling | ⚠️ | Ensure no stack traces to users |
| Encryption at rest | ❌ | Accepted gap for MVP |

## 8. Recommendations Summary

1. **Must fix**: Add explicit file permissions (0600)
2. **Should fix**: Add `busy_timeout` PRAGMA
3. **Nice to have**: Error message sanitization
4. **Future**: Consider SQLCipher if handling proprietary code
