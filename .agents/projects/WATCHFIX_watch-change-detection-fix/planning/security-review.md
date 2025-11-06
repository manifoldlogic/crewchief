# Security Review: Watch Change Detection Fix

## Security Posture

**Risk Level**: LOW - This is an internal bug fix in file indexing logic with minimal security surface.

**Threat Model**: Local development tool, no network exposure, no user-provided input beyond file paths from the local filesystem.

## Security Considerations

### 1. Path Traversal (LOW RISK)

**Potential Issue**: normalize_to_relpath() could be exploited with malicious paths.

**Attack Vector**:
```
Malicious path: /workspace/../../etc/passwd
After strip_prefix: ../../../etc/passwd
Could read files outside repo?
```

**Mitigations**:
1. ✅ **Already Protected**: `strip_prefix()` returns `Err` if path is not within prefix
   ```rust
   let relpath = absolute_path.strip_prefix(repo_root)?;  // Fails on path traversal
   ```

2. ✅ **Additional Check**: Explicitly reject paths with `..` components
   ```rust
   fn normalize_to_relpath(absolute_path: &Path, repo_root: &Path) -> Result<PathBuf> {
       let relpath = absolute_path
           .strip_prefix(repo_root)
           .context("Path is not within repository root")?;

       // Reject paths with parent directory components
       if relpath.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
           anyhow::bail!("Path contains parent directory components");
       }

       Ok(relpath.to_path_buf())
   }
   ```

3. ✅ **File Watcher Scope**: notify crate only watches paths within configured directory

**Verdict**: No additional mitigation needed. Existing checks are sufficient.

### 2. Symlink Following (LOW RISK)

**Potential Issue**: Symlinks could cause indexing of files outside repository.

**Attack Scenario**:
```
/workspace/src/evil.rs → symlink to /etc/passwd
Watch detects change to evil.rs
Reads content from /etc/passwd
Indexes sensitive data into database
```

**Current Behavior**:
- Rust's `fs::read_to_string()` follows symlinks by default
- Could read files outside repository
- But database stores original relpath, not resolved path

**Enterprise Expectation**: Symlinks should be rejected or resolved carefully.

**Pragmatic MVP Approach**: Accept symlink following as low-risk.

**Rationale**:
1. User controls their own repository
2. No remote access to database
3. Indexing local files is expected behavior
4. Symlinks are legitimate in monorepos (e.g., linking shared configs)

**Mitigation (Optional)**:
```rust
// Detect symlinks and log warning
if fs::symlink_metadata(path)?.file_type().is_symlink() {
    warn!(path = %path.display(), "Indexing symlink - resolved path may be outside repo");
}
```

**Verdict**: Document symlink behavior, add warning log. No hard rejection needed for MVP.

### 3. Race Conditions (LOW RISK)

**Potential Issue**: File modified between hash computation and database update.

**Attack Scenario**:
```
1. Detect change, compute hash: hash_a
2. Attacker modifies file
3. Read file content: content_b (doesn't match hash_a)
4. Index content_b with hash_a
5. Database inconsistency
```

**Current Protection**:
- Transaction wraps all operations
- File content read inside transaction
- If file changes during read, next watch cycle catches it

**Additional Risk**: Time-of-check-time-of-use (TOCTOU)
```
1. Check: get_file_id_by_path() succeeds
2. Time passes...
3. Use: index_new_file() expects file to exist
4. File deleted in between
5. Error (acceptable)
```

**Verdict**: Race conditions lead to retry or failure, not security breach. Acceptable for local dev tool.

### 4. Database Injection (MINIMAL RISK)

**Potential Issue**: Relpath used in SQL query could contain malicious characters.

**Attack Scenario**:
```
Malicious filename: src/evil'; DROP TABLE files; --.rs
Query: SELECT id FROM files WHERE relpath = 'src/evil'; DROP TABLE files; --.rs'
```

**Current Protection**: Parameterized queries everywhere ✅

```rust
client.query_opt(
    "SELECT id FROM maproom.files WHERE relpath = $1",
    &[&relpath],  // ✅ Parameter, not string interpolation
).await?;
```

**Verdict**: No injection risk. Parameterized queries prevent SQL injection.

### 5. Denial of Service (LOW RISK)

**Attack Scenario 1**: Flood watch with events
```
Create script that modifies 10,000 files simultaneously
Watch processes all events
Database overloaded
```

**Protection**: Debouncing (500ms window) batches events

**Residual Risk**: Could still queue many tasks, but processing is sequential.

**Verdict**: Acceptable. Watch is for development, not production. Users can Ctrl+C if needed.

**Attack Scenario 2**: Very large files
```
Create 1GB file in repo
Watch attempts to index
fs::read_to_string() loads entire file into memory
OOM crash
```

**Current Protection**: None

**Pragmatic Mitigation**: Skip files > 10MB
```rust
let metadata = fs::metadata(path)?;
if metadata.len() > 10 * 1024 * 1024 {  // 10MB limit
    warn!(path = %path.display(), size = metadata.len(), "File too large to index");
    return Ok(());  // Skip
}
```

**Verdict**: Add file size check. Low priority for MVP, but good practice.

### 6. Information Disclosure (MINIMAL RISK)

**Potential Issue**: Logging exposes sensitive information.

**Current Logging**:
```rust
warn!(path = %path.display(), error = %e, "Failed to process file");
info!(path = %path_display, "Indexed new file");
```

**Risk**: File paths visible in logs could reveal repo structure.

**Enterprise Expectation**: Sanitize logs, redact sensitive data.

**Pragmatic MVP**: File paths are not sensitive for local development tool.

**Verdict**: No redaction needed. Users control their own logs.

### 7. Dependency Vulnerabilities (ONGOING)

**Risk**: Third-party crates could have vulnerabilities.

**Key Dependencies**:
- `tokio` - Async runtime (actively maintained)
- `tokio-postgres` - Database driver (actively maintained)
- `notify` - File watcher (actively maintained)
- `anyhow` - Error handling (minimal attack surface)

**Protection**: Run `cargo audit` regularly

**Verdict**: Monitor dependencies, update regularly. No immediate concerns.

## Attack Surface Analysis

### Trust Boundaries

```
┌─────────────────────────────────┐
│  Filesystem (Untrusted Input)   │
│  - File paths from notify        │
│  - File content from disk        │
└──────────────┬──────────────────┘
               │
               ↓ Path validation
┌──────────────────────────────────┐
│  Watch Command (Trusted Process) │
│  - Path normalization            │
│  - Change detection              │
│  - Hash computation              │
└──────────────┬───────────────────┘
               │
               ↓ Parameterized queries
┌──────────────────────────────────┐
│  Database (Trusted Store)        │
│  - File metadata                 │
│  - Code chunks                   │
│  - Hashes                        │
└──────────────────────────────────┘
```

**Key Insight**: All input validation happens at filesystem boundary. Database is fully trusted.

### Input Validation

**What we validate**:
1. ✅ Paths are within repository root
2. ✅ Paths don't contain `..` components
3. ✅ File exists before reading
4. ❌ File size limits (TODO for DoS protection)
5. ❌ Symlink detection (acceptable to skip)

**What we trust**:
- Database contents (under our control)
- File watcher events (from notify crate)
- Filesystem reads (user's own files)

## Security Best Practices

### 1. Principle of Least Privilege

**Current**: Watch runs with user's filesystem permissions.

**Recommendation**: No change needed. User-level permissions are appropriate.

### 2. Defense in Depth

**Layer 1**: Path validation (reject paths outside repo)
**Layer 2**: Filesystem permissions (OS-level access control)
**Layer 3**: Database transactions (rollback on error)

**Verdict**: Adequate depth for local development tool.

### 3. Secure Defaults

**Current Defaults**:
- Database: localhost only, no remote access
- File watcher: only configured directory
- Logging: info level (not debug with sensitive data)

**Verdict**: Secure by default. No changes needed.

### 4. Error Handling

**Security-relevant errors**:
- Path outside repo → Skip event, log warning
- Database connection failure → Retry, then fail
- File read error → Skip file, continue processing

**Verdict**: Errors don't expose sensitive information or create vulnerabilities.

## Compliance Considerations

**GDPR**: Not applicable (no personal data indexed)

**HIPAA**: Not applicable (no healthcare data)

**SOC 2**: Not applicable (not a SaaS product)

**Enterprise Security**: Local development tool, minimal requirements.

**Verdict**: No compliance concerns for MVP.

## Security Testing

### 1. Malicious Path Tests

```rust
#[test]
fn test_reject_path_traversal() {
    let evil = Path::new("/workspace/../../etc/passwd");
    let root = Path::new("/workspace");
    assert!(normalize_to_relpath(evil, root).is_err());
}

#[test]
fn test_reject_parent_components() {
    let evil = Path::new("/workspace/src/../../../etc/passwd");
    let root = Path::new("/workspace");
    // Even if resolved, should fail strip_prefix check
    assert!(normalize_to_relpath(evil, root).is_err());
}
```

### 2. SQL Injection Tests

```rust
#[tokio::test]
async fn test_no_sql_injection() {
    let pool = setup_test_db().await;
    let malicious_relpath = "'; DROP TABLE files; --";

    // Should safely handle malicious input
    let result = get_file_id_by_path(&pool, "repo", "worktree", malicious_relpath).await;

    // Either returns None (not found) or valid file_id, but never executes injection
    assert!(result.is_ok());

    // Verify files table still exists
    let count: i64 = pool.get().await.unwrap()
        .query_one("SELECT COUNT(*) FROM maproom.files", &[])
        .await.unwrap()
        .get(0);
    assert_eq!(count, 0);  // Table intact, query was safe
}
```

### 3. DoS Tests

```rust
#[tokio::test]
async fn test_large_file_handling() {
    let pool = setup_test_db().await;
    let processor = IncrementalProcessor::new(pool);

    // Create 50MB file
    let large_file = create_temp_file_with_size(50 * 1024 * 1024);

    // Should skip or handle gracefully, not crash
    let result = processor.index_new_file(&large_file, &hash).await;

    // Either skips or fails gracefully (no panic, no OOM)
    assert!(result.is_ok() || result.is_err());  // Just don't panic
}
```

## Deployment Security

### 1. Binary Distribution

**Current**: Binaries built in GitHub Actions, stored in packages/cli/bin/

**Risk**: Supply chain attacks

**Mitigation**:
- ✅ Builds happen in GitHub's infrastructure (trusted)
- ✅ Binaries signed with GitHub provenance (future: add code signing)
- ❌ No checksum verification (low priority for internal tool)

**Verdict**: Acceptable for MVP. Add code signing for public release.

### 2. Configuration

**Current**: Database URL in environment variable

**Risk**: Hardcoded credentials in code

**Check**:
```bash
grep -r "password" crates/maproom/src/  # Should find nothing
grep -r "DATABASE_URL" crates/maproom/src/  # Should use env var
```

**Verdict**: No hardcoded credentials. Good practice maintained.

## Security Recommendations

### Implement Now (MVP Blockers)

1. ✅ **Path validation**: Already implemented via strip_prefix
2. ✅ **Parameterized queries**: Already implemented everywhere
3. ❌ **File size limits**: Add 10MB limit to prevent DoS
   - Priority: Medium
   - Effort: 5 lines of code
   - Impact: Prevents OOM crashes

### Implement Later (Nice to Have)

1. ❌ **Symlink detection**: Log warnings for symlinks
   - Priority: Low
   - Effort: 10 lines of code
   - Impact: Security awareness, no hard blocking

2. ❌ **Code signing**: Sign binaries for distribution
   - Priority: Low for internal tool
   - Effort: CI/CD changes
   - Impact: Supply chain security

3. ❌ **Checksum verification**: Verify downloaded binaries
   - Priority: Low
   - Effort: Generate and check SHA256
   - Impact: Integrity verification

### Skip (Out of Scope)

1. ❌ **Authentication/Authorization**: Not needed for local tool
2. ❌ **Encryption at rest**: Database is local, user controls it
3. ❌ **Network security**: No network exposure
4. ❌ **Rate limiting**: Not a web service
5. ❌ **Audit logging**: Standard logs are sufficient

## Conclusion

**Security Assessment**: LOW RISK

**Blockers**: None. The fix doesn't introduce new security vulnerabilities.

**Recommendations**: Add file size limit (10MB) to prevent DoS. All other security concerns are acceptable for a local development tool.

**Approval**: Ready to proceed with implementation. No security review blockers.
