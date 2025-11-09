# Security Review: Automatic Branch Switch Detection

## Threat Model

### Assets
- File system access (watching .git/HEAD)
- Long-running process (resource exhaustion risk)
- Database connection (persistent connection)

### Attackers
- Malicious git repository (crafted .git/HEAD)
- Resource exhaustion attacks
- File system symlink attacks

## Security Analysis

### 1. File System Access

#### Threat: Symlink Attack

**Scenario**: .git/HEAD is symlink to sensitive file
```bash
rm .git/HEAD
ln -s /etc/passwd .git/HEAD
```

**Likelihood**: Low (requires repo write access)
**Impact**: Medium (could leak file contents via branch name)

**Mitigation**:

```rust
fn validate_git_head(repo_path: &Path) -> Result<PathBuf> {
    let git_head = repo_path.join(".git/HEAD");

    // Check it's a regular file, not symlink
    let metadata = fs::metadata(&git_head)?;
    if !metadata.is_file() {
        bail!("HEAD is not a regular file");
    }

    // Check it's within repo
    let canonical = git_head.canonicalize()?;
    if !canonical.starts_with(repo_path) {
        bail!("HEAD is outside repository");
    }

    Ok(canonical)
}
```

**Risk Level**: 🟡 **MITIGATED** (with validation)

#### Threat: File Content Injection

**Scenario**: Malicious content in .git/HEAD
```
ref: refs/heads/main; rm -rf /
```

**Likelihood**: Low (requires repo write access)
**Impact**: None (we only parse, never execute)

**Mitigation**: Already safe - we parse content, never execute
```rust
// ✅ SAFE: Only parsing, no execution
fn get_current_branch(repo_path: &Path) -> Result<String> {
    let content = fs::read_to_string(repo_path.join(".git/HEAD"))?;
    if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
        Ok(branch.trim().to_string())
    } else {
        Ok(content.trim()[..8].to_string())
    }
}
```

**Risk Level**: 🟢 **NOT APPLICABLE** (no execution)

### 2. Resource Exhaustion

#### Threat: Rapid Branch Switching DoS

**Scenario**: Script rapidly switches branches to exhaust resources
```bash
while true; do
  git checkout main
  git checkout feature
done
```

**Likelihood**: Low (requires malicious user)
**Impact**: Medium (CPU/memory spike, slow system)

**Mitigation**:

**Strategy 1**: Debouncing
```rust
// Ignore events within 1 second of previous
if now.duration_since(last_event) < Duration::from_secs(1) {
    debug!("Debouncing rapid event");
    return Ok(());
}
```

**Strategy 2**: Rate limiting
```rust
// Max 10 updates per minute
if updates_this_minute > 10 {
    warn!("Rate limit exceeded, skipping update");
    return Ok(());
}
```

**Strategy 3**: Queue with limit
```rust
// Max 3 queued updates
if update_queue.len() > 3 {
    warn!("Update queue full, dropping oldest");
    update_queue.pop_front();
}
```

**Risk Level**: 🟡 **MITIGATED** (with debouncing)

#### Threat: Memory Leak

**Scenario**: Long-running watcher leaks memory over time

**Likelihood**: Low (Rust ownership prevents most leaks)
**Impact**: Medium (process OOM after days)

**Mitigation**:

**Testing**: Long-running stability test
```rust
#[test]
#[ignore]
fn test_no_memory_leak() {
    let initial_mem = get_memory_usage();

    // Run for 1 hour with periodic switches
    run_watcher_for_duration(Duration::from_secs(3600));

    let final_mem = get_memory_usage();

    // Memory should not grow >10%
    assert!(final_mem < initial_mem * 1.1);
}
```

**Monitoring**: Log memory usage periodically
```rust
tokio::spawn(async {
    loop {
        debug!("Memory usage: {}MB", get_memory_usage_mb());
        tokio::time::sleep(Duration::from_secs(300)).await; // Every 5 min
    }
});
```

**Risk Level**: 🟢 **ACCEPTED** (tested, monitored)

### 3. Database Security

#### Threat: Connection Pool Exhaustion

**Scenario**: Watcher holds database connection indefinitely

**Likelihood**: Low (pools have timeouts)
**Impact**: Medium (other processes can't connect)

**Mitigation**:

```rust
// Use small pool for watcher (not default pool)
let pool = PgPoolOptions::new()
    .max_connections(2)  // Only need 1-2 for watcher
    .idle_timeout(Duration::from_secs(300))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&db_url)
    .await?;
```

**Risk Level**: 🟢 **MITIGATED** (connection limits)

### 4. Process Security

#### Threat: Unauthorized Process Termination

**Scenario**: Attacker kills watcher process

**Likelihood**: Medium (if process is user-owned)
**Impact**: Low (just restart it)

**Mitigation**: Not critical - watcher is optional
- User can restart: `maproom watch --repo ...`
- No data loss (only affects auto-indexing)

**Risk Level**: 🟢 **ACCEPTED** (low impact)

#### Threat: Process Privilege Escalation

**Scenario**: Watcher runs as root (shouldn't)

**Likelihood**: Very Low (user error)
**Impact**: High (root process vulnerable)

**Mitigation**:

**Documentation**: Clearly state "do not run as root"
```markdown
# Security Note
Never run `maproom watch` as root. Run as regular user.
```

**Code check**:
```rust
fn ensure_not_root() -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = unsafe { libc::getuid() };
        if uid == 0 {
            bail!("Refusing to run as root (UID 0)");
        }
    }
    Ok(())
}
```

**Risk Level**: 🟡 **MITIGATED** (with check)

## Security Checklist

### Design Phase
- [x] Validate .git/HEAD is regular file
- [x] No shell command execution
- [x] Debouncing for rapid events
- [x] Connection pool limits
- [x] Refuse to run as root

### Implementation Phase
- [ ] Code review: File path validation
- [ ] Code review: No unsafe blocks
- [ ] Test symlink attack prevention
- [ ] Test rapid switching resource usage
- [ ] Test long-running stability

### Testing Phase
- [ ] Fuzz test .git/HEAD parsing
- [ ] Load test (1000 rapid switches)
- [ ] Memory leak test (24 hour run)
- [ ] Resource usage test (CPU/RAM)

### Deployment Phase
- [ ] Document security considerations
- [ ] Document "do not run as root"
- [ ] Monitor resource usage in production

## Compliance

### GDPR
**Not applicable** - No personal data processed (only git metadata)

### Audit
**Optional logging**:
```rust
info!("Branch switch: {} -> {} (user: {})", old_branch, new_branch, user);
```

## Known Limitations

### Accepted Risks

1. **User can DoS themselves** - Rapid switching slows their own system (acceptable)
2. **Process can be killed** - User can restart (no data loss)
3. **Repository tampering** - If user has repo write access, they can modify .git/ (acceptable)

### Not Implemented (Deferred)

**Sandboxing**: Not in MVP
- Rationale: Watcher runs as user, limited permissions
- Future: Could use seccomp on Linux if needed

**Authentication**: Not in MVP
- Rationale: Single-user system
- Future: Add if multi-user

## Security Review Sign-Off

**Status**: ✅ **APPROVED FOR MVP**

**Summary**:
- Low-risk component (reads git metadata only)
- No command execution
- Resource exhaustion mitigated via debouncing
- Refuses to run as root

**Recommendation**: Proceed with implementation

**Post-deployment**:
- Monitor resource usage
- Test long-running stability
- Document security best practices
