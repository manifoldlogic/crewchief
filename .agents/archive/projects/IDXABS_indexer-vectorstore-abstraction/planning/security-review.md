# Security Review: Indexer VectorStore Abstraction

> **Note:** PostgreSQL references in this document are legacy. The project scope has
> changed to **SQLite-only** - PostgreSQL is being completely removed. The core security
> analysis (parameterized queries, file permissions, threat model) remains valid.

## 1. Overview

This project refactors internal code paths to use the `VectorStore` trait instead of PostgreSQL-specific types. The security posture does not fundamentally change - we're routing through a different interface, not adding new capabilities.

## 2. Security Assessment

### 2.1 Attack Surface Analysis

| Component | Before | After | Change |
|-----------|--------|-------|--------|
| Database connections | PostgreSQL only | PostgreSQL or SQLite | Expanded |
| File system access | Indexer reads source files | Unchanged | None |
| Network access | PostgreSQL + embedding APIs | Same + SQLite local file | Reduced (SQLite is local) |
| User input handling | CLI arguments | Unchanged | None |

### 2.2 Risk Analysis

#### Low Risk: SQLite File Permissions

**Description**: SQLite database stored as local file could be read by other users.

**Current Mitigation**: `SqliteStore::connect()` already sets `0600` permissions:
```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600))?;
}
```

**Assessment**: ✓ Already handled, no new work needed.

#### Low Risk: Path Traversal in Indexer

**Description**: Indexer processes file paths from repository.

**Current Mitigation**:
- `ignore` crate respects `.gitignore` boundaries
- Paths are relative to repository root
- No symbolic link following by default

**Assessment**: ✓ Existing protections sufficient.

#### Low Risk: SQL Injection

**Description**: User input (repo name, worktree name, query) used in database queries.

**Current Mitigation**:
- All queries use parameterized statements
- VectorStore trait methods take typed parameters
- No string interpolation into SQL

**PostgreSQL example**:
```rust
client.query("SELECT id FROM repos WHERE name = $1", &[&name]).await?
```

**SQLite example**:
```rust
conn.query_row("SELECT id FROM repos WHERE name = ?1", params![name], |row| row.get(0))?
```

**Assessment**: ✓ Both backends use parameterized queries.

#### Low Risk: Denial of Service (Large Repos)

**Description**: Indexing very large repositories could exhaust resources.

**Current Mitigation**:
- Concurrent file limits in parallel mode
- Batch processing with controlled sizes
- SQLite WAL mode prevents blocking

**Assessment**: ✓ Existing controls sufficient.

### 2.3 No New Security Concerns

This refactoring:
- Does not add new user input channels
- Does not change authentication/authorization
- Does not expose new network endpoints
- Does not change file access patterns
- Does not modify credential handling

## 3. SQLite-Specific Considerations

### 3.1 Database File Location

**Default**: `~/.maproom/maproom.db`

**Security Properties**:
- User-owned directory (`~/.maproom/`)
- File permissions `0600` (owner read/write only)
- No world-readable data

**Recommendation**: Document that database contains code snippets and should be treated as sensitive as source code.

### 3.2 WAL Mode Security

SQLite WAL (Write-Ahead Log) creates additional files:
- `maproom.db-wal` (write-ahead log)
- `maproom.db-shm` (shared memory)

**Security Properties**:
- Same directory, same permissions inherited
- Automatically cleaned up on connection close
- No sensitive data exposure beyond main database

**Assessment**: ✓ No additional security measures needed.

### 3.3 No Encryption

SQLite database is not encrypted. This matches PostgreSQL behavior (data at rest in PostgreSQL is also not encrypted by default).

**Recommendation**: Document as known limitation. Enterprise encryption is out of scope for MVP.

## 4. Threat Model

### 4.1 Trusted Inputs

| Input | Trust Level | Validation |
|-------|-------------|------------|
| Repository path | Trusted (user-provided) | Must exist, must be git repo |
| File contents | Trusted (user's code) | Parsed by tree-sitter |
| CLI arguments | Trusted (user-provided) | Validated by clap |
| Database URL | Trusted (env var or config) | URL parsing |

### 4.2 Untrusted Inputs

| Input | Source | Handling |
|-------|--------|----------|
| None | - | Project indexes user's own code |

This is a local tool processing local code - there are no untrusted inputs.

### 4.3 Threats Not Addressed (Out of Scope)

| Threat | Why Out of Scope |
|--------|------------------|
| Malicious code in indexed repo | User is indexing their own code |
| Man-in-the-middle on embedding API | Existing concern, not changed by this project |
| Multi-user access to SQLite | SQLite is single-user by design |

## 5. Security Checklist

### Pre-Implementation
- [x] No new user input channels added
- [x] No new network endpoints added
- [x] No credential handling changes
- [x] SQLite file permissions already handled

### Implementation
- [ ] Use parameterized queries (enforce in code review)
- [ ] No `format!()` for SQL construction
- [ ] Validate file paths stay within repo root
- [ ] Log security-relevant events (connection errors, permission issues)

### Post-Implementation
- [ ] Verify SQLite database file permissions
- [ ] Test with paths containing special characters
- [ ] Verify no SQL injection via repo/worktree names

## 6. Recommendations

### 6.1 Documentation

Add to `crates/maproom/CLAUDE.md`:

```markdown
## Security Notes

### SQLite Database

The SQLite database (`~/.maproom/maproom.db`) contains:
- Repository and worktree metadata
- Code chunk previews and signatures
- Embeddings (vector representations of code)

**Treat this file with the same sensitivity as your source code.**

File permissions are automatically set to `0600` (owner-only) on Unix systems.

### Data at Rest

Neither PostgreSQL nor SQLite backends encrypt data at rest by default.
For sensitive codebases, consider:
- Full-disk encryption
- Encrypted home directory
- Self-hosted PostgreSQL with TDE
```

### 6.2 No Additional Mitigations Required

This refactoring maintains the existing security posture. No new security controls are needed.

## 7. Conclusion

**Security Impact**: Neutral

This project:
- Does not introduce new vulnerabilities
- Does not change the threat model
- Maintains existing security controls
- SQLite backend has appropriate file permissions

**Recommendation**: Proceed with implementation. Standard code review for parameterized query usage is sufficient.
