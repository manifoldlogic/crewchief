# Ticket: BINPKG-1902: Fix dead code warning in VectorExecutor::process_rows

## Status
- [x] **Task completed** - Code fix implemented (removed unused function)
- [x] **Tests pass** - All tests pass with fix
- [x] **Verified** - Build succeeds in GitHub Actions

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Fix compilation error in GitHub Actions workflow caused by unused `VectorExecutor::process_rows` function in `crates/maproom/src/search/vector.rs`. The workflow uses `RUSTFLAGS="-D warnings"` which treats all warnings as errors, causing all platform builds to fail.

## Background
During BINPKG-1901 canary release testing, the GitHub Actions workflow failed during the darwin-arm64 build (and would fail for all platforms) with:

```
error: associated function `process_rows` is never used
  --> crates/maproom/src/search/vector.rs:320:8
   |
320 |     fn process_rows(rows: Vec<tokio_postgres::Row>) -> Result<Vec<RankedResult>, VectorError> {
   |        ^^^^^^^^^^^^
   |
   = note: `-D dead-code` implied by `-D warnings`
```

The `actions-rust-lang/setup-rust-toolchain@v1` action automatically sets `RUSTFLAGS="-D warnings"` to ensure code quality. This is a best practice for CI/CD but requires all warnings to be addressed.

The function `process_rows` at line 320 was likely replaced by `process_rows_with_dimension` (which is actively used at lines 195, 251, and 315) but was not removed. This dead code blocks all builds in the automated release pipeline.

**Workflow Run**: https://github.com/danielbushman/crewchief/actions/runs/19048176542
**Failed Job**: Build darwin-arm64 (all other builds would fail identically)
**Impact**: CRITICAL - Blocks entire automated release workflow for all platforms

## Acceptance Criteria
- [x] Unused `process_rows` function is removed from `vector.rs` (lines 319-338), OR
- [x] Function is marked with `#[allow(dead_code)]` if needed for future use (with comment explaining why)
- [x] Code compiles successfully with `RUSTFLAGS="-D warnings"`
- [x] Local build succeeds: `cargo build --release --manifest-path crates/maproom/Cargo.toml`
- [x] All existing tests pass: `cargo test --manifest-path crates/maproom/Cargo.toml`
- [x] No other dead code warnings remain in the maproom crate
- [x] GitHub Actions workflow succeeds on all platforms after fix is merged

## Technical Requirements

### Code Analysis
The dead code appears to be a legacy function:
- **Unused function**: `VectorExecutor::process_rows` (line 320)
- **Active replacement**: `VectorExecutor::process_rows_with_dimension` (line 341)
- **Usage count**: `process_rows` = 0 usages, `process_rows_with_dimension` = 3 usages

The active function includes dimension information in results, suggesting it's the intended implementation.

### Solution Options

**Option 1: Remove the function (RECOMMENDED)**
- Simplest solution
- Removes maintenance burden
- No performance impact
- If functionality is needed later, it's in git history

**Option 2: Add `#[allow(dead_code)]` attribute**
```rust
#[allow(dead_code)]
fn process_rows(rows: Vec<tokio_postgres::Row>) -> Result<Vec<RankedResult>, VectorError> {
    // Implementation...
}
```
- Only if there's a documented reason to keep it
- Requires comment explaining future use case
- Adds maintenance burden

**Recommended**: Option 1 (remove) unless there's a documented plan to use it.

### Verification Commands

```bash
# Verify no dead code warnings
cargo clean
RUSTFLAGS="-D warnings" cargo build --release --manifest-path crates/maproom/Cargo.toml

# Run tests
cargo test --manifest-path crates/maproom/Cargo.toml

# Check for other dead code warnings in vector.rs
cargo clippy --manifest-path crates/maproom/Cargo.toml -- -D warnings

# Verify function usage (should show 0 results for process_rows if removed)
grep -n "process_rows" crates/maproom/src/search/vector.rs
```

### Workflow Impact
This fix unblocks:
- All 4 platform builds (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- Binary validation job
- npm publish job
- Complete automated release pipeline

## Implementation Notes

### Root Cause
The `actions-rust-lang/setup-rust-toolchain@v1` GitHub Action sets `RUSTFLAGS="-D warnings"` by default. This is intentional and ensures high code quality by treating warnings as errors in CI.

Local builds typically don't set this flag, which is why the issue wasn't caught during development.

### Why This Matters
Dead code indicates:
1. **Potential bugs**: Function might have been meant to be used
2. **Maintenance burden**: Developers must maintain unused code
3. **Confusion**: Future developers don't know if it's intentional
4. **Code smell**: Suggests incomplete refactoring

### Testing Strategy
1. **Local verification**: Build with strict flags before pushing
2. **Test coverage**: Ensure no tests depend on removed function
3. **Functionality check**: Verify search operations still work
4. **CI validation**: Let GitHub Actions verify on all platforms

### Related Code
The function at line 320 appears to be superseded by `process_rows_with_dimension` at line 341, which adds embedding dimension metadata to results. All call sites use the newer function:
- Line 195: FTS search results
- Line 251: Vector search results
- Line 315: Hybrid search results

This suggests `process_rows` is legacy code from before dimension tracking was added.

## Dependencies

**Discovered During**:
- BINPKG-1901: Canary release integration test (workflow execution revealed this issue)

**Blocks**:
- BINPKG-1901: Cannot complete canary test until builds succeed
- BINPKG-5001: Dry run documentation (needs working workflow)
- BINPKG-5002: Production release execution (critical blocker)

**No Dependencies**: This is a straightforward code cleanup with no external dependencies.

## Risk Assessment

- **Risk**: Removing function breaks something unexpected
  - **Likelihood**: Very Low (0 usages found in codebase)
  - **Impact**: Low (would be caught by tests)
  - **Mitigation**: Run full test suite before and after removal, verify no grep matches for function name

- **Risk**: Other dead code warnings exist that will block future builds
  - **Likelihood**: Low
  - **Impact**: Medium (requires additional fixes)
  - **Mitigation**: Run `cargo clippy` with `-D warnings` to find all issues preemptively

- **Risk**: Fix introduces new compilation errors
  - **Likelihood**: Very Low (simple deletion)
  - **Impact**: Low (caught immediately in local build)
  - **Mitigation**: Test with `RUSTFLAGS="-D warnings"` locally

- **Risk**: Function was meant to be used but wasn't wired up
  - **Likelihood**: Very Low (replaced by better function)
  - **Impact**: None (replacement function is better)
  - **Mitigation**: Code review confirms `process_rows_with_dimension` is the intended implementation

## Files/Packages Affected

### Files to Modify
- `/workspace/crates/maproom/src/search/vector.rs` - Remove or annotate `process_rows` function (lines 319-338)

### Files to Verify (Read Only)
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Workflow that triggered the error
- `/workspace/crates/maproom/Cargo.toml` - Build configuration
- `/workspace/crates/maproom/src/search/vector.rs` - Full file review for other issues

### No New Files Required
This is a code cleanup task.

## Estimated Effort
**15-30 minutes**

Breakdown:
- 5 min: Review code and confirm function is truly unused
- 2 min: Remove function (or add annotation)
- 5 min: Run local build and tests with strict warnings
- 5 min: Commit and push fix
- 5-10 min: Monitor GitHub Actions workflow success
- 3 min: Update ticket and verify completion

## Priority
**CRITICAL** - Blocks entire automated release pipeline. All 4 platform builds fail without this fix.

This is a fix ticket (19XX series) for an issue discovered during integration testing. Must be resolved before continuing with release process.

## Related Workflow Error

### Error Message
```
Run cross build --release --target aarch64-apple-darwin --manifest-path crates/maproom/Cargo.toml
   Compiling maproom v0.1.0 (/workspace/crates/maproom)
error: associated function `process_rows` is never used
  --> crates/maproom/src/search/vector.rs:320:8
   |
320 |     fn process_rows(rows: Vec<tokio_postgres::Row>) -> Result<Vec<RankedResult>, VectorError> {
   |        ^^^^^^^^^^^^
   |
   = note: `-D dead-code` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(dead_code)]`

error: could not compile `maproom` (bin "crewchief-maproom") due to 1 previous error
Error: Process completed with exit code 101.
```

### Workflow Context
- **Trigger**: Tag push `v1.3.1-canary.1`
- **Job**: build-binaries (darwin-arm64 matrix)
- **Step**: Build binary
- **Exit Code**: 101 (compilation failure)

### Fix Verification
After implementing fix, verify by:
1. Pushing fix to test branch
2. Triggering workflow with manual dispatch (dry_run: true)
3. Confirming all 4 build jobs succeed
4. Merging to main if successful

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md`
- **Test report**: `.agents/projects/BINPKG_binary-packaging/test-reports/canary-1.3.1-test-report.md`

### Rust Documentation
- **Dead code lint**: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#dead-code
- **Clippy lints**: https://rust-lang.github.io/rust-clippy/master/

### GitHub Actions
- **Workflow run**: https://github.com/danielbushman/crewchief/actions/runs/19048176542
- **rust-toolchain action**: https://github.com/actions-rust-lang/setup-rust-toolchain
- **RUSTFLAGS documentation**: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-reads

## Related Tickets

### Discovered In
- BINPKG-1901: Canary release integration test (this issue blocked workflow execution)

### Blocks
- BINPKG-1901: Must fix before completing canary test
- BINPKG-5001: Dry run documentation
- BINPKG-5002: Production release execution

### Sequence
1. BINPKG-1901: Integration test started
2. **BINPKG-1902**: Fix discovered issue (this ticket)
3. BINPKG-1901: Complete integration test after fix
4. BINPKG-5001-5002: Proceed with production release

## Success Criteria

The fix is complete when:
1. ✓ Code compiles with `RUSTFLAGS="-D warnings"`
2. ✓ All tests pass locally
3. ✓ GitHub Actions workflow succeeds on all 4 platforms
4. ✓ No other dead code warnings remain
5. ✓ Fix is committed to main branch
6. ✓ BINPKG-1901 canary test can proceed
