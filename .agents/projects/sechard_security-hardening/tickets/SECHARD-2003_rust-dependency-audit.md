# SECHARD-2003: Audit and Fix Rust Dependencies

**Status:** Open  
**Phase:** 2 (Execution)  
**Estimated Effort:** 45-60 minutes  
**Priority:** High  

---

## Summary

Install cargo-audit, run security audit on Rust crates, and remediate any vulnerabilities found in the Rust dependency tree.

---

## Background

**Current State:**
- `cargo audit` is not installed on this system
- Rust dependencies have not been audited for known vulnerabilities
- Unknown security posture for Rust crates

**Why This Matters:**
- Rust crates can have vulnerabilities just like npm packages
- Database drivers (tokio-postgres, sqlx), HTTP clients, and serialization libraries are common attack surfaces
- Proactive auditing prevents future security incidents

**Scope:**
- All Rust workspaces in `crates/` directory
- Transitive dependencies in Cargo.lock

---

## Acceptance Criteria

1. ✅ `cargo-audit` installed and working
2. ✅ Security audit run successfully
3. ✅ All critical/high vulnerabilities fixed
4. ✅ Moderate/low vulnerabilities fixed or documented as accepted risks
5. ✅ `cargo audit` reports clean (or documented exceptions)
6. ✅ All Rust tests pass
7. ✅ All Rust builds succeed

---

## Technical Requirements

### Prerequisites

**Install cargo-audit:**
```bash
cargo install cargo-audit
```

**Verify installation:**
```bash
cargo audit --version
```

### Audit Execution

**Run full audit:**
```bash
# From workspace root
cargo audit

# With JSON output for parsing
cargo audit --json > rust-audit.json

# Show only specific severity
cargo audit --deny warnings
```

---

## Implementation Steps

### Step 1: Install cargo-audit

```bash
# Install the tool
cargo install cargo-audit

# Verify
cargo audit --version
# Expected: cargo-audit X.Y.Z
```

**Alternative (if cargo install fails):**
```bash
# Using cargo-binstall
cargo install cargo-binstall
cargo binstall cargo-audit
```

### Step 2: Run Initial Audit

```bash
# Full audit
cargo audit 2>&1 | tee rust-audit-before.txt

# Check exit code
echo $?
# 0 = no issues, 1 = vulnerabilities found
```

**Parse results:**
```bash
# Count by severity
grep -i "critical" rust-audit-before.txt
grep -i "high" rust-audit-before.txt
grep -i "moderate" rust-audit-before.txt
```

### Step 3: Analyze Vulnerabilities

For each vulnerability found:

1. **Identify affected crate:**
   - Note crate name and version
   - Check if direct or transitive dependency

2. **Check advisory details:**
   - CVE number
   - GHSA identifier
   - Severity rating
   - Patched versions

3. **Determine fix strategy:**
   - Can we update the crate?
   - Is there a patch available?
   - Do we need to change our code?

### Step 4: Apply Fixes

**Strategy A: Update Dependencies**
```bash
# Update all dependencies to latest compatible
cargo update

# Update specific crate
cargo update -p <crate-name>

# Update to specific version
cargo update -p <crate-name> --precise <version>
```

**Strategy B: Upgrade to Newer Version**
```toml
# Cargo.toml
[dependencies]
vulnerable-crate = "2.0"  # was "1.x"
```

**Strategy C: Use Patches (Last Resort)**
```toml
# Cargo.toml (workspace root)
[patch.crates-io]
vulnerable-crate = { git = "https://github.com/maintainer/fork", branch = "security-fix" }
```

### Step 5: Verify Fixes

```bash
# Re-run audit
cargo audit 2>&1 | tee rust-audit-after.txt

# Compare
diff rust-audit-before.txt rust-audit-after.txt

# Build all crates
cargo build --all

# Run all tests
cargo test --all
```

---

## Common Vulnerability Patterns

### Database-Related Crates

**tokio-postgres, sqlx, rusqlite:**
- SQL injection vulnerabilities
- Connection handling issues
- Fix: Update to latest stable version

### Serialization Crates

**serde, serde_json, toml, yaml-rust:**
- Deserialization attacks
- DoS via crafted input
- Fix: Update to patched versions

### HTTP/Network Crates

**reqwest, hyper, tokio:**
- Request smuggling
- TLS validation issues
- Fix: Update and verify TLS configuration

### Crypto Crates

**ring, rustls, openssl:**
- Timing attacks
- Weak cipher suites
- Fix: Update immediately (critical)

---

## Verification Steps

1. **Audit Clean:**
   ```bash
   cargo audit
   # Should output: "Success No vulnerable packages found"
   ```

2. **Build Verification:**
   ```bash
   cargo build --all --release
   # Should complete without errors
   ```

3. **Test Verification:**
   ```bash
   cargo test --all
   # All tests should pass
   ```

4. **CLI Functionality:**
   ```bash
   # Test main binaries
   cargo run --bin crewchief-maproom -- --help
   cargo run --bin crewchief-maproom -- status
   ```

5. **Integration Tests:**
   ```bash
   # Run any shell-based integration tests
   ./scripts/test-*.sh
   ```

---

## Files to Modify

1. **`Cargo.toml` files**
   - Update dependency versions
   - Add patches if needed

2. **`Cargo.lock`**
   - Will be auto-updated by cargo update
   - Commit this file!

3. **Source code (if needed)**
   - API changes may require code updates
   - Check migration guides for major version bumps

---

## Handling Specific Scenarios

### Scenario 1: No Fix Available

If a vulnerability has no patch:

1. **Check alternatives:**
   - Is there a maintained fork?
   - Can we use a different crate?

2. **Implement workaround:**
   - Disable vulnerable feature
   - Add input validation
   - Limit exposure

3. **Document accepted risk:**
   ```toml
   # Cargo.toml
   # SECURITY NOTE: crate-name@1.2.3 has CVE-YYYY-XXXXX
   # Accepted because: [reason]
   # Mitigation: [what we're doing instead]
   # Review date: YYYY-MM-DD
   ```

### Scenario 2: Breaking Changes

If update requires code changes:

1. **Review changelog:**
   ```bash
   # Check what changed
   cargo search <crate-name>
   # Visit crates.io for changelog
   ```

2. **Update gradually:**
   - Update one crate at a time
   - Fix compilation errors
   - Update tests

3. **Test thoroughly:**
   - Run full test suite
   - Manual testing of affected features

### Scenario 3: Transitive Dependency

If vulnerability is deep in tree:

1. **Find path:**
   ```bash
   cargo tree -i <vulnerable-crate>
   ```

2. **Update parent:**
   ```bash
   cargo update -p <parent-crate>
   ```

3. **Force specific version:**
   ```toml
   [dependencies]
   parent-crate = { version = "x.y", features = ["..."] }
   
   [patch.crates-io]
   vulnerable-crate = "=fixed.version"
   ```

---

## Documentation Requirements

Create `SECURITY-AUDIT.md`:

```markdown
# Security Audit Results

**Date:** 2025-11-21
**Tool:** cargo-audit v{version}

## Summary
- Total vulnerabilities found: X
- Critical: X (fixed)
- High: X (fixed)
- Moderate: X (fixed)
- Low: X (fixed/accepted)

## Accepted Risks
None / [List with justification]

## Next Audit
Scheduled: [Date, e.g., quarterly]
```

---

## Definition of Done

- [ ] cargo-audit installed
- [ ] Audit run and results documented
- [ ] All critical/high vulnerabilities fixed
- [ ] Moderate/low vulnerabilities fixed or accepted with documentation
- [ ] All builds succeed
- [ ] All tests pass
- [ ] CLI binaries work correctly
- [ ] SECURITY-AUDIT.md created
- [ ] Changes committed
- [ ] Ticket marked as Complete

---

## Resources

- **cargo-audit repo:** https://github.com/rustsec/rustsec/tree/main/cargo-audit
- **RustSec Advisory DB:** https://github.com/rustsec/advisory-db
- **Cargo Book (patches):** https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html
- **Security advisories:** https://rustsec.org/

---

## Rollback Plan

```bash
# Restore previous state
git checkout Cargo.toml Cargo.lock

# Rebuild
cargo clean
cargo build --all
```

---

## Notes

**Why Rust Security Matters:**
- Rust's memory safety prevents many vulnerability classes
- But logic bugs, algorithmic complexity, and supply chain issues still exist
- Regular audits are essential for maintaining security posture

**Frequency:**
- Initial audit: This ticket
- Ongoing: Run `cargo audit` in CI/CD
- Periodic: Quarterly security reviews

**CI Integration (Future):**
```yaml
# .github/workflows/security.yml
- name: Audit Rust dependencies
  run: |
    cargo install cargo-audit
    cargo audit --deny warnings
```
