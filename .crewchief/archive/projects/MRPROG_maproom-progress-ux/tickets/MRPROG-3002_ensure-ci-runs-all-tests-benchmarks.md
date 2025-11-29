# Ticket: MRPROG-3002: Ensure CI runs all tests and benchmarks

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Verify that the CI pipeline (GitHub Actions) runs all tests (unit, integration) and benchmarks for the maproom crate. Add or update CI configuration if needed to ensure progress tracking code is validated on every PR.

## Background
With new unit tests (MRPROG-1004), integration tests (MRPROG-2003), and benchmarks (MRPROG-1005), we need to ensure CI runs them all. This prevents regressions and validates that performance targets are maintained.

This is pragmatic CI integration: ensure tests run and pass, benchmarks run informational (don't block on performance variations across CI runners).

This ticket implements Phase 3, task 3 from the MRPROG project plan: ensuring robust CI validation of all progress tracking code.

## Acceptance Criteria
- [ ] CI workflow runs `cargo test` for maproom crate
- [ ] CI workflow runs `cargo test --lib` (unit tests)
- [ ] CI workflow runs `cargo test --test '*'` (integration tests)
- [ ] CI workflow runs `cargo bench` (informational, not blocking)
- [ ] CI workflow runs `cargo clippy --all-targets`
- [ ] CI workflow runs `cargo audit` (security check)
- [ ] All checks pass on a test PR
- [ ] CI configuration documented (if modified)

## Technical Requirements

### GitHub Actions Workflow Configuration

Check existing CI workflow (likely `.github/workflows/rust.yml` or similar) and ensure it includes:

```yaml
name: Rust CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
        components: clippy

    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Run unit tests
      run: cargo test --lib
      working-directory: ./crates/maproom

    - name: Run integration tests
      run: cargo test --test '*'
      working-directory: ./crates/maproom

    - name: Run all tests
      run: cargo test
      working-directory: ./crates/maproom

    - name: Run clippy
      run: cargo clippy --all-targets -- -D warnings
      working-directory: ./crates/maproom

    - name: Run benchmarks (informational)
      run: cargo bench --no-run  # Build but don't execute (too slow for CI)
      working-directory: ./crates/maproom
      continue-on-error: true  # Don't fail CI on benchmark issues

    - name: Security audit
      run: cargo audit
      working-directory: ./crates/maproom
      continue-on-error: true  # Advisory, not blocking
```

### Key Requirements

1. **Test execution:**
   - Run both unit and integration tests
   - Use `cargo test` to run all tests
   - Execute in `crates/maproom` directory

2. **Benchmarks:**
   - Use `--no-run` to build benchmarks without executing (CI runners vary)
   - Mark as `continue-on-error: true` (informational only)
   - OR remove if benchmarks are too slow for CI

3. **Clippy:**
   - Run with `-D warnings` to fail on warnings
   - Ensures code quality standards

4. **Cargo audit:**
   - Check for security vulnerabilities
   - `continue-on-error: true` (advisory, not blocking immediately)

## Implementation Notes

### Implementation Steps

1. Locate existing CI workflow file (`.github/workflows/*.yml`)
2. Verify it includes maproom crate testing
3. Add missing steps if needed
4. Ensure working directory is set correctly
5. Test CI by creating a draft PR
6. Verify all checks pass
7. Document any changes made

### Testing Strategy

1. Create a test PR with trivial change
2. Verify all CI checks run
3. Verify tests pass
4. Check CI logs for any issues
5. Confirm that benchmark build succeeds (even if not executed)

### Documentation

If CI configuration is modified:
- Document changes in commit message
- Update `.github/CLAUDE.md` if new workflows added
- Note any CI-specific configuration decisions

## Dependencies

### Blocked By
- **MRPROG-1004** - Unit tests must exist
- **MRPROG-1005** - Benchmarks must exist
- **MRPROG-2003** - Integration tests must exist

These tickets must be completed before CI can validate their execution.

## Risk Assessment

- **Risk**: CI runners might have different performance characteristics
  - **Mitigation**: Don't block on benchmark execution time; use `--no-run` or `continue-on-error: true`

- **Risk**: Integration tests might be flaky in CI environment
  - **Mitigation**: Increase timeouts if needed; ensure tests are hermetic

- **Risk**: Cargo audit might fail on transitive dependencies
  - **Mitigation**: Use `continue-on-error: true` for audit (advisory, not blocking)

## Files/Packages Affected

- **MODIFY**: `.github/workflows/*.yml` (if changes needed)
- **OPTIONAL**: Add `.github/workflows/maproom-ci.yml` if dedicated workflow desired

## Planning References

- **Plan**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Phase 3, task 3)
- **Quality strategy**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/quality-strategy.md` (CI integration)

## Estimated Effort
1-2 hours

## Success Criteria
- All tests run automatically on PRs
- CI passes for MRPROG branch
- No false failures due to configuration issues
- Progress tracking code is validated on every commit
