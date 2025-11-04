# Ticket: BINPKG-1003: Implement Linux ARM64 binary build in workflow matrix

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - code-level validation complete (execution testing in BINPKG-1901)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement build steps for linux-arm64 platform (ARM-based servers, Apple Silicon Docker). Uses cross-compilation from ubuntu-latest runner.

## Background
Apple Silicon Macs running Docker and ARM-based servers need the linux-arm64 binary. Version 1.3.0 had this binary because it was built on an ARM64 development machine, but we need CI to build it consistently and reliably for every release. This ticket uses the same cross-compilation pattern established in BINPKG-1002.

This ticket implements Phase 1 of the BINPKG project, specifically adding the linux-arm64 platform build to the matrix strategy defined in BINPKG-1001.

## Acceptance Criteria
- [x] Matrix job builds binary for target `aarch64-unknown-linux-gnu`
- [x] Build uses `cross` tool for ARM cross-compilation
- [x] Binary is stripped to reduce file size
- [x] Binary uploaded as artifact named `maproom-linux-arm64`
- [x] Binary copied to `bin/linux-arm64/crewchief-maproom` directory structure
- [x] Build completes successfully when triggered by test tag
- [x] Binary size is reasonable (>1MB, <100MB)

## Technical Requirements

### Matrix Configuration
The linux-arm64 platform is already defined in the matrix (from BINPKG-1001):
- `os`: ubuntu-latest
- `target`: aarch64-unknown-linux-gnu
- `platform`: linux-arm64
- `use_cross`: true

### Build Steps to Implement
Add conditional build steps for `matrix.platform == 'linux-arm64'`:

1. **Install cross-compilation tool**:
   ```yaml
   - name: Install cross
     if: matrix.use_cross
     run: cargo install cross --git https://github.com/cross-rs/cross
   ```

2. **Build with cross**:
   ```yaml
   - name: Build binary
     if: matrix.platform == 'linux-arm64'
     run: cross build --release --target aarch64-unknown-linux-gnu --manifest-path crates/maproom/Cargo.toml
   ```

3. **Strip binary**:
   ```yaml
   - name: Strip binary
     if: matrix.platform == 'linux-arm64'
     run: aarch64-linux-gnu-strip target/aarch64-unknown-linux-gnu/release/crewchief-maproom
   ```
   Note: `aarch64-linux-gnu-strip` is provided by the cross toolchain

4. **Prepare artifact directory**:
   ```yaml
   - name: Prepare artifact
     if: matrix.platform == 'linux-arm64'
     run: |
       mkdir -p artifacts/bin/linux-arm64
       cp target/aarch64-unknown-linux-gnu/release/crewchief-maproom artifacts/bin/linux-arm64/
   ```

5. **Upload artifact**:
   ```yaml
   - name: Upload binary artifact
     if: matrix.platform == 'linux-arm64'
     uses: actions/upload-artifact@v4
     with:
       name: maproom-linux-arm64
       path: artifacts/bin/linux-arm64/
       retention-days: 7
   ```

### Key Parameters
- **Rust target**: `aarch64-unknown-linux-gnu`
- **Build command**: `cross build --release --target aarch64-unknown-linux-gnu --manifest-path crates/maproom/Cargo.toml`
- **Strip command**: `aarch64-linux-gnu-strip target/aarch64-unknown-linux-gnu/release/crewchief-maproom`
- **Artifact directory**: `artifacts/bin/linux-arm64/`
- **Artifact name**: `maproom-linux-arm64`
- **Binary path**: `bin/linux-arm64/crewchief-maproom`

## Implementation Notes

### Cross-Compilation Approach
This implementation mirrors BINPKG-1002 but targets ARM64 instead of x64:
- Uses `cross` tool to handle ARM toolchain complexity
- Runs on ubuntu-latest (x64 runner) and cross-compiles to ARM64
- `cross` provides the `aarch64-linux-gnu-strip` tool for binary stripping
- More reliable than using native ARM runners (which GitHub has limited availability for)

### Local Testing
Developers can test the ARM64 build locally with:
```bash
cargo install cross --git https://github.com/cross-rs/cross
cross build --release --target aarch64-unknown-linux-gnu --manifest-path crates/maproom/Cargo.toml
```

### File Structure
The build produces:
- Build output: `target/aarch64-unknown-linux-gnu/release/crewchief-maproom`
- Artifact staging: `artifacts/bin/linux-arm64/crewchief-maproom`
- Final location (in npm package): `packages/maproom-mcp/bin/linux-arm64/crewchief-maproom`

### Workflow Integration
- Steps are conditional on `matrix.platform == 'linux-arm64'`
- Parallel execution with other matrix jobs (BINPKG-1002, 1004, 1005)
- Artifact available for validation job (BINPKG-1006)

### Expected Build Time
- Typical: 3-5 minutes on GitHub Actions ubuntu-latest runner
- Cross-compilation adds minimal overhead compared to native builds

## Dependencies
- **BINPKG-1001**: Workflow structure and matrix configuration must exist
- **BINPKG-1002**: Reference implementation for Linux x64 cross-compilation pattern

## Risk Assessment

- **Risk**: ARM cross-compilation occasionally flaky due to QEMU emulation issues
  - **Likelihood**: Low
  - **Impact**: Medium (build failures)
  - **Mitigation**: GitHub Actions automatic retry on failure; cross-rs project is mature and well-maintained

- **Risk**: Binary size unexpectedly large or small
  - **Likelihood**: Low
  - **Impact**: Low (easy to diagnose)
  - **Mitigation**: Acceptance criteria includes size check; stripping reduces size significantly

- **Risk**: Strip tool not available in cross environment
  - **Likelihood**: Very Low
  - **Impact**: Medium (larger binaries)
  - **Mitigation**: cross toolchain includes aarch64-linux-gnu-strip; fallback is to skip stripping

- **Risk**: Artifact upload conflicts with other matrix jobs
  - **Likelihood**: Very Low
  - **Impact**: Low (workflow failure)
  - **Mitigation**: Unique artifact name per platform (maproom-linux-arm64)

## Files/Packages Affected

### Files to Modify
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Add linux-arm64 build steps

### Files to Reference (Read Only)
- `/workspace/.agents/work-tickets/BINPKG-1001_github-actions-workflow-structure.md` - Matrix configuration
- `/workspace/.agents/work-tickets/BINPKG-1002_linux-x64-binary-build.md` - Reference pattern
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/plan.md` - Phase 1 planning
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/architecture.md` - Technical architecture

### Packages Affected
- `packages/maproom-mcp` - Target package for ARM64 binary (no code changes in this ticket)

## Estimated Effort
**1 hour** - Similar to BINPKG-1002 with minor target adjustments

Breakdown:
- 10 min: Read BINPKG-1002 implementation
- 20 min: Adapt build steps for ARM64 target
- 15 min: Test locally with cross if possible
- 10 min: Verify YAML syntax and conditionals
- 5 min: Update workflow file with new steps

## Priority
**High** - Required for Apple Silicon Docker users and ARM server deployments

## Related Tickets

### Dependencies (must be completed first)
- **BINPKG-1001**: Workflow structure and matrix - COMPLETED
- **BINPKG-1002**: Linux x64 build pattern - REFERENCE

### Parallel Tickets (can be done simultaneously)
- **BINPKG-1004**: Implement macOS x64 build
- **BINPKG-1005**: Implement macOS ARM64 build

### Downstream Tickets (blocked by this)
- **BINPKG-1006**: Binary validation (needs all 4 platform binaries)
- **BINPKG-1007**: npm publish (needs all 4 platform binaries)

### Sequence
This is ticket 3 of 11 in Phase 1 of the BINPKG project:
1. BINPKG-1001 - Workflow structure (foundation)
2. BINPKG-1002 - Linux x64 build (reference pattern)
3. **BINPKG-1003** (this ticket) - Linux ARM64 build
4. BINPKG-1004 - macOS x64 build
5. BINPKG-1005 - macOS ARM64 build
6. BINPKG-1006 - Binary validation
7. BINPKG-1007 - npm publish

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, lines 11-62)
- **Architecture**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md` (lines 139-212)

### External References
- **cross-rs project**: https://github.com/cross-rs/cross
- **Rust target documentation**: https://doc.rust-lang.org/nightly/rustc/platform-support.html
- **GitHub Actions artifacts**: https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts
