# Security Review: Incremental Scan Completion

## Threat Model

### Attack Surface

**Input Vectors:**
1. Git repository path (user-controlled)
2. Repository name (user-controlled)
3. Worktree name (user-controlled)
4. Database connection string (environment variable)

**Trust Boundaries:**
1. CLI arguments → Git commands (shell injection risk)
2. Git output → Database (injection risk)
3. User filesystem → Database (path traversal risk)

### Threat Actors

**Primary Concern:** Malicious repository owner

**Scenarios:**
- Attacker provides crafted repository
- Attacker manipulates git state
- Attacker provides malicious path

**Out of Scope:** Network attacks (local tool only)

## Security Analysis by Component

### 1. Git Tree SHA Retrieval

**Function:** `get_git_tree_sha(repo_path)`

**Risk:** Command injection via path

**Current Mitigation:**
```rust
// From existing code (crates/maproom/src/git/mod.rs)
Command::new("git")
    .args(&["-C", repo_path.to_str().unwrap(), "rev-parse", "HEAD^{tree}"])
    .output()
```

**Assessment:** ✅ **SAFE**
- Uses `Command::new()` with args array (not shell)
- `-C` flag scopes to specific directory
- No shell expansion possible

**Additional Validation:**
- Tree SHA output is validated (40-char hex)
- Invalid format rejected

**Residual Risk:** LOW - Path could point to attacker-controlled repo, but:
- User already trusts repo (they're scanning it)
- No privilege escalation possible
- Worst case: scan wrong repo (user error)

### 2. Database Queries

**Risk:** SQL injection via repo/worktree names

**Parameterized Queries:**
```rust
// All queries use $1, $2 placeholders
client.query_opt(
    "SELECT last_tree_sha FROM worktree_index_state WHERE worktree_id = $1",
    &[&worktree_id]
)
```

**Assessment:** ✅ **SAFE**
- All database operations use parameterized queries
- No string concatenation in SQL
- PostgreSQL client library handles escaping

**Validation:**
- Repo/worktree names validated by database constraints
- Foreign key relationships prevent orphaned records

**Residual Risk:** NONE - Standard parameterized query protection

### 3. Filesystem Operations

**Risk:** Path traversal via repository path

**Code Path:**
```rust
// User provides path via --path flag
let path = path.unwrap_or_else(|| PathBuf::from("."));
let tree_sha = get_git_tree_sha(&path)?;
```

**Assessment:** ⚠️ **MEDIUM RISK**

**Attack Scenario:**
```bash
# Attacker tries to scan outside intended directory
crewchief-maproom scan --path "../../../etc/passwd"

# Would fail: not a git repo
# But what if they point to a git repo elsewhere?
crewchief-maproom scan --path "/tmp/malicious-repo"
```

**Impact:**
- Scan indexer database with wrong repository
- Potential DoS (scan huge repo)
- No privilege escalation (tool runs as user)

**Mitigation Strategy:**

**Option 1: Path Validation (Recommended)**
```rust
// Before any operations
fn validate_repo_path(path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize()
        .context("Invalid or inaccessible path")?;

    // Ensure path is within workspace or explicitly allowed
    if !is_allowed_path(&canonical) {
        return Err(anyhow!("Path outside allowed directories"));
    }

    // Ensure is actually a git repository
    if !canonical.join(".git").exists() {
        return Err(anyhow!("Not a git repository"));
    }

    Ok(canonical)
}
```

**Option 2: Require Explicit Confirmation (Simpler)**
```rust
// If path is absolute and outside CWD, warn user
if path.is_absolute() && !path.starts_with(env::current_dir()?) {
    eprintln!("⚠️  Warning: Scanning repository outside current directory");
    eprintln!("   Path: {}", path.display());
    eprintln!("   Press Enter to continue, Ctrl+C to cancel");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
}
```

**Decision for This Project:** Use existing behavior (no new restrictions).

**Rationale:**
- Users already trust the repository (providing path explicitly)
- CLI tool, not web service (user = attacker scenario less relevant)
- Adding restrictions might break legitimate use cases
- Can be enhanced in future if abuse discovered

**Residual Risk:** LOW - User must explicitly provide malicious path

### 4. State Persistence

**Risk:** Database poisoning with malicious tree SHA

**Attack Scenario:**
```bash
# Attacker provides repo with crafted tree SHA
# (Not possible - git controls SHA format)
```

**Assessment:** ✅ **SAFE**
- Git generates tree SHAs cryptographically
- 40-character hex format enforced
- No user control over tree SHA content

**Additional Protection:**
```rust
// Validate tree SHA format
fn is_valid_tree_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())
}
```

**Residual Risk:** NONE - Git controls tree SHA generation

### 5. Race Conditions

**Risk:** Concurrent scans corrupt state

**Scenario:**
```
Thread A: Scan repo, get tree abc123
Thread B: Scan repo, get tree def456
Thread A: Update state with abc123
Thread B: Update state with def456
Result: State = def456 (last writer wins)
```

**Assessment:** ✅ **ACCEPTABLE**

**Why Safe:**
- `ON CONFLICT DO UPDATE` is atomic
- No partial state possible
- Both scans are valid (just different timing)
- Eventually consistent (next scan will be correct)

**Alternative Mitigation (if needed):**
```sql
-- Add version column for optimistic locking
UPDATE worktree_index_state
SET last_tree_sha = $1, version = version + 1
WHERE worktree_id = $2 AND version = $3;
```

**Decision:** Not needed for this project. Race is benign.

**Residual Risk:** NONE - Eventual consistency acceptable

## Known Gaps

### 1. No Git Signature Verification

**Risk:** Malicious commits in scanned repository

**Impact:** Index contains malicious code patterns

**Mitigation:** Out of scope - trust boundary is "user trusts repo"

**Future Enhancement:** Optional GPG signature checking

### 2. No Rate Limiting

**Risk:** Attacker triggers many rapid scans (DoS)

**Impact:** High database load, API costs

**Mitigation:** Not applicable (local CLI tool, user controls execution)

**Future Enhancement:** Add rate limit if tool becomes service

### 3. No Audit Logging

**Risk:** Cannot detect abuse patterns

**Impact:** Hard to diagnose security issues

**Mitigation:** Existing tracing logs cover basic auditing

**Future Enhancement:** Structured audit log for compliance

## Security Best Practices Applied

### ✅ Input Validation
- Git commands use array args (no shell)
- Database queries use parameterized statements
- Tree SHA format validated

### ✅ Least Privilege
- Tool runs as user (no elevation)
- Database permissions scoped (SELECT, INSERT, UPDATE on specific tables)
- No system-wide modifications

### ✅ Error Handling
- Git failures logged, not exposed to user
- Database errors don't reveal schema details
- Fallback to safe defaults (full scan on error)

### ✅ Logging
- Security-relevant events logged (tree SHA comparisons, state updates)
- No sensitive data in logs (paths only, no file contents)
- Tracing enabled for debugging

### ⚠️ Needs Improvement
- No path sandboxing (allow scanning any git repo)
- No explicit user confirmation for unusual paths

## Deployment Security

### Environment Variables

**MAPROOM_DATABASE_URL:**
- Contains database credentials
- Must not be logged or exposed
- Sanitized in error messages (existing `sanitize_db_url()`)

**Risk:** Credential leakage in logs

**Mitigation:**
```rust
// Already implemented in crates/maproom/src/search-optimization/security/sanitize.rs
pub fn sanitize_db_url(url: &str) -> String {
    // Redacts password from postgres://user:pass@host/db
    url.split('@').last().unwrap_or("***").to_string()
}
```

### File Permissions

**Database Socket:**
- Unix socket requires file permissions
- User must have read/write access

**Repository Files:**
- Read-only access sufficient for scanning
- No files modified by tool

**Risk:** NONE - Standard file permission model applies

## Compliance Considerations

### Data Privacy

**No Personal Data:**
- Tool indexes code structure only
- No user credentials, PII, or business data
- Embeddings are derived features, not raw content

**GDPR/CCPA:** Not applicable (no personal data processing)

### Supply Chain Security

**Dependencies:**
- `tokio-postgres`: Well-maintained, security audits
- `git` command: System package (user's responsibility)
- `anyhow`, `tracing`: Minimal attack surface

**Rust Safety:**
- Memory safety guaranteed by compiler
- No unsafe blocks in new code
- Bounds checking enforced

## Security Testing

### Test Cases

**1. Command Injection Attempt**
```bash
# Try to inject shell commands via path
crewchief-maproom scan --path "/tmp/repo; rm -rf /"
# Expected: Treat as literal path, fail (not a git repo)
```

**2. SQL Injection Attempt**
```bash
# Try to inject SQL via repo name
crewchief-maproom scan --repo "'; DROP TABLE worktrees; --"
# Expected: Parameterized query prevents injection
```

**3. Path Traversal Attempt**
```bash
# Try to escape intended directory
crewchief-maproom scan --path "../../../../etc"
# Expected: Fail (not a git repo), but no privilege escalation
```

**4. Race Condition Test**
```bash
# Run two scans concurrently
crewchief-maproom scan &
crewchief-maproom scan &
wait
# Expected: Both complete, state is consistent
```

### Fuzzing Opportunities

**Future Enhancement:**
- Fuzz git output parser (malformed tree SHAs)
- Fuzz path handling (various path formats)
- Fuzz concurrent state updates

## Security Sign-Off

### Risk Assessment Summary

| Risk Category | Severity | Likelihood | Residual Risk | Accepted? |
|---------------|----------|------------|---------------|-----------|
| Command Injection | High | Very Low | Low | ✅ Yes |
| SQL Injection | High | Very Low | None | ✅ Yes |
| Path Traversal | Medium | Low | Low | ✅ Yes |
| Race Conditions | Low | Medium | None | ✅ Yes |
| Data Leakage | Low | Very Low | None | ✅ Yes |

### Overall Security Posture: ✅ **ACCEPTABLE FOR PRODUCTION**

**Rationale:**
- Local CLI tool with user-level privileges
- All high-severity risks mitigated (injection attacks)
- Residual risks are low impact and low likelihood
- Security follows Rust and PostgreSQL best practices
- No new attack vectors introduced (only optimization)

### Recommendations for Future Enhancements

**P0 (Critical):** None - No critical vulnerabilities identified

**P1 (High):**
- Add explicit warning for absolute paths outside CWD
- Optional GPG signature verification for commits

**P2 (Medium):**
- Path sandboxing (restrict to workspace directories)
- Audit logging for security events

**P3 (Low):**
- Fuzzing integration
- Rate limiting (if tool becomes service)

### Approval

**Security Review Status:** ✅ **APPROVED**

**Conditions:**
- No changes to trust model (user trusts scanned repository)
- Standard deployment practices (secure DATABASE_URL)
- Monitoring for unusual scan patterns (optional)

**Reviewer Notes:**
This is a performance optimization with minimal security impact. The core security properties (parameterized queries, no shell injection) were already present. New code adds tree SHA checking and state persistence, both of which have low attack surface.

The main residual risk is path traversal, but this is inherent to the tool's design (users explicitly provide repository paths). Adding restrictions might break legitimate workflows without meaningful security benefit.

**Sign-Off:** This project can proceed to implementation.
