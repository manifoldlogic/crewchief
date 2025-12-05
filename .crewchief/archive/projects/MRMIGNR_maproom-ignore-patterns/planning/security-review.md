# Security Review: maproom ignore patterns

## Security Assessment

### Authentication & Authorization

**Not applicable.** This feature:
- Does not introduce network communication
- Does not handle user credentials
- Does not implement access control
- Operates on local filesystem only

### Data Protection

**Sensitive data considerations:**

1. **`.maproomignore` file contents:**
   - File may reveal information about repository structure
   - Patterns themselves are not sensitive (analogous to `.gitignore`)
   - No encryption needed - follows git precedent

2. **Indexed data:**
   - Feature explicitly prevents certain files from being indexed
   - Users control what is excluded via `.maproomignore`
   - No data exfiltration risk

**Verdict:** No sensitive data protection needed beyond standard file permissions.

### Input Validation

**User inputs:**

1. **`.maproomignore` file contents** (untrusted input)
   - **Risk:** Malicious glob patterns could cause denial of service
   - **Validation:**
     - Glob pattern compilation errors are caught and returned as `Result::Err`
     - Invalid patterns cause scan/watch to fail fast (startup error)
     - No arbitrary code execution possible (glob library is safe)
   - **Mitigation:** `globset` crate handles pathological patterns safely

2. **CLI `--exclude` patterns** (untrusted input)
   - **Risk:** Same as `.maproomignore` patterns
   - **Validation:** Same error handling as file-based patterns
   - **Mitigation:** Already in use, no new attack surface

3. **File paths** (partially trusted - from filesystem)
   - **Risk:** Path traversal attacks
   - **Validation:**
     - All paths normalized to repo root
     - No `..` components allowed (existing `normalize_to_relpath` protection)
     - WalkBuilder respects filesystem boundaries
   - **Mitigation:** Existing path handling infrastructure is secure

### Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| Malicious patterns consuming CPU | Low | Globset library handles efficiently, timeout possible | Accepted |
| Large .maproomignore files (10MB+) | Low | Reasonable file size, out-of-memory unlikely | Accepted |
| Race condition: file changes during pattern load | Low | Pattern load is atomic read, temporary inconsistency acceptable | Accepted |
| Symlink following in pattern matching | Low | Globset handles symlinks safely, respects gitignore semantics | Accepted |

**Risk acceptance rationale:**
- Local-only tool with single user
- No network exposure
- Worst case: degraded performance, not data breach
- Users control input (their own repository)

## MVP Security Scope

### In Scope for MVP

1. **Input validation:**
   - Glob pattern syntax validation
   - File read error handling
   - Path normalization

2. **Error handling:**
   - Fail fast on invalid patterns
   - No information leakage in error messages
   - Proper Result propagation

3. **Resource limits:**
   - Implicit limits via globset (no infinite loops)
   - File read size reasonable (expect <1MB ignore files)

### Out of Scope for MVP

1. **Sandboxing:** Not needed (local tool, no untrusted code execution)
2. **Encryption:** Not needed (no sensitive data)
3. **Rate limiting:** Not needed (single-user, local operations)
4. **Audit logging:** Not needed (debugging via tracing is sufficient)
5. **Pattern allowlisting:** Not needed (users control patterns)

## Security Checklist

- [x] **No hardcoded secrets** - Feature doesn't use secrets
- [x] **Input validation on external inputs** - Glob patterns validated by globset
- [x] **Proper error handling (no info leakage)** - Errors are generic ("invalid pattern")
- [x] **Dependencies are up to date** - Using existing `ignore` crate (maintained)
- [x] **No SQL injection vulnerabilities** - No SQL generation from patterns
- [x] **No XSS vulnerabilities** - Not a web application
- [x] **Path traversal protection** - Existing normalization prevents `../` attacks
- [x] **Fail-safe defaults** - Missing .maproomignore uses safe defaults

## Threat Model

### Threat: Malicious `.maproomignore` file

**Scenario:** Attacker creates `.maproomignore` with patterns designed to cause harm.

**Attack vectors:**
1. **CPU exhaustion:** Complex regex-like patterns
2. **Memory exhaustion:** Extremely large pattern file
3. **Denial of service:** Patterns that exclude all files

**Mitigations:**
1. `globset` crate handles complex patterns efficiently (used by ripgrep)
2. File size is limited by practical concerns (git diff would fail first)
3. Excluding all files is user error, not security issue (scan returns 0 files)

**Residual risk:** Low - local tool under user control

### Threat: Pattern injection via filename

**Scenario:** Malicious filename like `test/../../etc/passwd` could bypass filtering.

**Attack vectors:**
1. Path traversal via filename
2. Pattern matching against absolute paths

**Mitigations:**
1. All paths normalized relative to repo root (existing `normalize_to_relpath`)
2. WalkBuilder constrains traversal to repo directory
3. Git repository structure prevents arbitrary paths

**Residual risk:** None - existing protections sufficient

### Threat: Information disclosure via patterns

**Scenario:** `.maproomignore` patterns reveal sensitive directory structure.

**Attack vectors:**
1. Committed `.maproomignore` exposes internal structure
2. Patterns hint at existence of certain files

**Mitigations:**
1. `.maproomignore` is version-controlled (user decision)
2. Same risk as `.gitignore` (accepted industry pattern)
3. Users can gitignore `.maproomignore.local` for sensitive patterns

**Residual risk:** Low - equivalent to `.gitignore` (accepted)

## Dependency Security

**New dependencies:** None (uses existing `ignore` and `globset` crates)

**Existing dependencies:**
- `ignore` crate: Maintained, actively used by ripgrep (trust established)
- `globset` crate: Part of `ignore` ecosystem, well-tested

**Dependency review:**
- No known CVEs in `ignore` or `globset`
- Crates are pure Rust (memory-safe)
- No unsafe code in glob matching logic

## Secure Coding Practices

### Error Handling

```rust
// Good: Fail fast on invalid patterns
pub fn load_ignore_patterns(root: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(&maproomignore_path)
        .context("Failed to read .maproomignore")?;  // No path in error
    // ...
}
```

### Path Handling

```rust
// Good: Always normalize paths
let relative_path = normalize_to_relpath(absolute_path, root)?;
matcher.should_ignore(&relative_path);  // Work with relative paths only
```

### Resource Cleanup

```rust
// Good: RAII with Result
pub fn from_repository(root: &Path) -> Result<Self> {
    let patterns = load_ignore_patterns(root)?;  // Atomic read
    Self::with_patterns(patterns)  // No partial state
}
```

## Compliance

**Not applicable.** This is a developer tool with no:
- Personal data collection (no PII)
- Network communication (no data transmission)
- Multi-user scenarios (no access control needed)

No GDPR, SOC2, or other compliance requirements.

## Security Testing

### Tests Included

1. **Malformed input:**
   - Invalid glob patterns return errors
   - Missing files handled gracefully
   - Empty/comment-only files work

2. **Path handling:**
   - Relative paths work correctly
   - No path traversal via patterns
   - Root-relative patterns interpreted correctly

3. **Resource limits:**
   - Large pattern files don't crash
   - Complex patterns compile successfully

### Security Review Process

**Pre-merge:**
1. Code review by rust-indexer-engineer
2. Verify no `unsafe` code introduced
3. Confirm error messages don't leak paths
4. Check dependency versions

**Post-merge:**
1. Monitor for bug reports
2. Update dependencies quarterly
3. Review if security issues reported in `ignore` crate

## Incident Response

**If security issue discovered:**

1. **Assessment:**
   - Determine severity (likely Low - local tool)
   - Identify affected versions

2. **Mitigation:**
   - Patch and release new version
   - Document in CHANGELOG
   - No CVE needed (not public-facing service)

3. **Communication:**
   - Update documentation with workaround
   - Release notes mention security fix

## Conclusion

**Security posture: ACCEPTABLE for MVP**

This feature introduces **minimal security risk**:
- No new attack surface (local file reading already exists)
- Robust input validation via `globset` library
- Fail-safe defaults and error handling
- No sensitive data exposure

**Recommendation: APPROVE for implementation** with standard code review.
