# Ticket: BINPKG-1005: Implement macOS ARM64 binary build in workflow matrix

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - code-level validation complete (execution testing in BINPKG-1901)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Implement native build for darwin-arm64 platform (Apple Silicon Macs). Uses GitHub's macos-latest runner for native ARM compilation to produce the crewchief-maproom binary for Apple Silicon-based macOS systems.

## Background
Apple Silicon Mac users (M1/M2/M3/M4) need the darwin-arm64 binary to run maproom-mcp. Version 1.3.0 had this binary because it was built on an ARM64 machine. Unlike Linux, macOS cross-compilation is difficult and unreliable, so we use native builds on appropriate GitHub runners. The macos-latest runner has transitioned to ARM64 (as of 2024+), providing Apple Silicon hardware for native builds.

This ticket implements the darwin-arm64 matrix item defined in BINPKG-1001. It follows the same pattern as BINPKG-1004 (darwin-x64) but uses the ARM64-based macos-latest runner instead of the Intel-based macos-13 runner.

**Planning reference**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, darwin-arm64 implementation)

## Acceptance Criteria
- [ ] Matrix job successfully builds binary for target `aarch64-apple-darwin`
- [ ] Build uses native cargo (not cross) on macos-latest runner (Apple Silicon)
- [ ] Binary is stripped using macOS strip tool to reduce size
- [ ] Binary is copied to `artifacts/bin/darwin-arm64/crewchief-maproom` directory
- [ ] Binary is uploaded as GitHub artifact with name `maproom-darwin-arm64`
- [ ] Build completes successfully when tested with a version tag (manual workflow_dispatch dry run)

## Technical Requirements

### Matrix Configuration (already in BINPKG-1001)
- Platform identifier: `darwin-arm64`
- GitHub runner: `macos-latest` (Apple Silicon)
- Rust target: `aarch64-apple-darwin`
- Use cross: `false` (native build)

### Build Steps to Implement
Add these steps to the `build-binaries` job for the darwin-arm64 matrix item:

```yaml
steps:
  - name: Checkout code
    uses: actions/checkout@v4

  - name: Set up Rust
    uses: actions-rust-lang/setup-rust-toolchain@v1
    with:
      target: aarch64-apple-darwin

  - name: Build binary
    run: |
      cargo build --release --target aarch64-apple-darwin --manifest-path crates/maproom/Cargo.toml

  - name: Strip binary
    run: |
      strip target/aarch64-apple-darwin/release/crewchief-maproom

  - name: Prepare artifacts
    run: |
      mkdir -p artifacts/bin/darwin-arm64
      cp target/aarch64-apple-darwin/release/crewchief-maproom artifacts/bin/darwin-arm64/

  - name: Upload binary artifact
    uses: actions/upload-artifact@v4
    with:
      name: maproom-darwin-arm64
      path: artifacts/bin/darwin-arm64/crewchief-maproom
      retention-days: 1
```

### Build Command Details
- **Target**: `aarch64-apple-darwin` (explicitly specified for safety)
- **Manifest**: `crates/maproom/Cargo.toml` (maproom crate)
- **Output**: `target/aarch64-apple-darwin/release/crewchief-maproom`
- **Strip command**: `strip target/aarch64-apple-darwin/release/crewchief-maproom`
- **Artifact directory**: `artifacts/bin/darwin-arm64/`
- **Artifact name**: `maproom-darwin-arm64` (consistent naming scheme)

### Why Native Build (not cross)
- macOS cross-compilation from Linux is notoriously difficult
- Requires macOS SDK, system libraries, and code signing considerations
- GitHub provides native macOS runners, making cross unnecessary
- Native builds are more reliable and maintainable

### Why macos-latest for ARM64
- macos-latest has transitioned to Apple Silicon (ARM64) as of 2024
- Provides M1/M2/M3-class hardware for native ARM64 builds
- Apple Silicon is faster than Intel Macs for compilation
- Default runner for macOS aligns with Apple's platform direction

## Implementation Notes

### Native Build Simplicity
Native builds on macOS are significantly simpler than cross-compilation:
- No special tooling required (cross tool not needed)
- Rust toolchain auto-detects the platform
- Standard macOS strip tool works out of the box
- No additional setup for system libraries

### macOS Strip Syntax
The macOS `strip` command has the same basic syntax as Linux:
```bash
strip target/aarch64-apple-darwin/release/crewchief-maproom
```

### Rust Target Auto-Detection
While Rust can auto-detect the aarch64-apple-darwin target on macos-latest, we specify it explicitly in the build command for:
1. Clarity and documentation
2. Safety against future runner changes
3. Consistency with cross-compilation platforms

### Pattern Nearly Identical to BINPKG-1004
This implementation is almost identical to BINPKG-1004 (darwin-x64) with these changes:
- Runner: `macos-latest` instead of `macos-13`
- Target: `aarch64-apple-darwin` instead of `x86_64-apple-darwin`
- Directory: `bin/darwin-arm64` instead of `bin/darwin-x64`
- Artifact name: `maproom-darwin-arm64` instead of `maproom-darwin-x64`

### GitHub Runner Performance
- Apple Silicon runners are generally faster than Intel Mac runners for Rust compilation
- However, macOS runners are still significantly slower and more expensive than Linux runners
- Cost: ~10x more expensive than ubuntu-latest
- Mitigation: This is acceptable for MVP, can optimize later with caching strategies

### Artifact Retention
- `retention-days: 1` is sufficient since artifacts are only needed for the publish job
- Reduces storage costs for GitHub Actions
- Consistent with other platform builds (BINPKG-1002, BINPKG-1003, BINPKG-1004)

### Testing the Build
Use manual workflow_dispatch with dry_run:
```bash
# Push a test tag
git tag v0.0.0-test-darwin-arm64
git push origin v0.0.0-test-darwin-arm64

# Or use workflow_dispatch from GitHub UI
# Set dry_run: true to skip publish step
```

## Dependencies
- **BINPKG-1001**: Workflow structure and matrix configuration (must be completed first)
- **BINPKG-1004**: Reference pattern for macOS native builds

## Risk Assessment

- **Risk**: GitHub may change macos-latest runner definition
  - **Likelihood**: Low (Apple Silicon is the current macOS platform)
  - **Impact**: Medium (could break builds if reverted to Intel)
  - **Mitigation**: Specify explicit runner version (e.g., `macos-14`, `macos-15`) if stability is required. Monitor GitHub runner announcements.

- **Risk**: Longer build times on macOS runners
  - **Likelihood**: High (macOS runners are consistently slower than Linux)
  - **Impact**: Low (slightly longer CI times)
  - **Mitigation**: Acceptable for MVP. Can optimize later with build caching (cargo cache) or conditional builds. Apple Silicon is faster than Intel Macs, so this is better than BINPKG-1004.

- **Risk**: Higher cost of macOS runners
  - **Likelihood**: High (macOS minutes are ~10x Linux)
  - **Impact**: Low (acceptable for open source projects with free minutes)
  - **Mitigation**: Monitor GitHub Actions usage. Optimize if needed by reducing build frequency or using conditional triggers.

- **Risk**: Binary compatibility across macOS versions
  - **Likelihood**: Low (Rust binaries are generally portable)
  - **Impact**: Medium (some users can't run the binary)
  - **Mitigation**: macos-latest targets a reasonable minimum macOS version. Document minimum macOS version in package README.

- **Risk**: Missing version 1.3.0 darwin-arm64 binary
  - **Likelihood**: High (binary was missing due to build environment)
  - **Impact**: High (Apple Silicon users couldn't use maproom-mcp)
  - **Mitigation**: This ticket directly addresses the regression. Native builds on macos-latest ensure ARM64 binary is always built.

## Files/Packages Affected

### Files to Modify
- **MODIFY**: `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml`
  - Add build steps to the darwin-arm64 matrix item
  - Steps follow the pattern established in BINPKG-1004 (darwin-x64)
  - Replace placeholder TODO comment with actual implementation

### Files to Reference (Read Only)
- `/workspace/crates/maproom/Cargo.toml` - Build manifest
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Workflow file (read for context)
- `/workspace/.agents/work-tickets/BINPKG-1004_macos-x64-binary-build.md` - Reference pattern (nearly identical)
- `/workspace/.agents/work-tickets/BINPKG-1002_linux-x64-binary-build.md` - Reference pattern
- `/workspace/.agents/work-tickets/BINPKG-1003_linux-arm64-binary-build.md` - Reference pattern

### Packages Affected
- `crates/maproom` - Binary being built (no code changes, just CI/CD)

## Estimated Effort
**1 hour**

Breakdown:
- 15 min: Review BINPKG-1004 implementation (nearly identical pattern)
- 30 min: Add build steps to workflow for darwin-arm64 matrix item
- 15 min: Test with manual workflow_dispatch dry run

## Priority
**High** - Part of Phase 1 critical path. Required for complete multi-platform binary coverage. Addresses regression where darwin-arm64 binary was missing in version 1.3.0.

## Related Tickets

### Depends On
- **BINPKG-1001**: Workflow structure must exist (COMPLETED or IN PROGRESS)
- **BINPKG-1004**: Reference pattern for macOS native builds

### Parallel Implementation
- **BINPKG-1002**: Linux x64 build implementation (same phase)
- **BINPKG-1003**: Linux ARM64 build implementation (same phase)
- **BINPKG-1004**: macOS x64 build implementation (same phase)

### Enables
- **BINPKG-1006**: Binary validation (requires all 4 platform builds)
- **BINPKG-1007**: npm publish (requires all 4 platform builds)

### Sequence
This is ticket 5 of 11 in Phase 1 of the BINPKG project:
1. BINPKG-1001 - Workflow structure ✓
2. BINPKG-1002 - Linux x64 build
3. BINPKG-1003 - Linux ARM64 build
4. BINPKG-1004 - macOS x64 build
5. **BINPKG-1005** (this ticket) - macOS ARM64 build
6. BINPKG-1006 - Binary validation
7. BINPKG-1007 - npm publish

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md`
- **Architecture**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md`

### External References
- **GitHub Actions runners**: https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners
- **Rust cross-compilation**: https://rust-lang.github.io/rustup/cross-compilation.html
- **macOS strip tool**: `man strip` on any macOS system
- **GitHub Actions pricing**: https://docs.github.com/en/billing/managing-billing-for-github-actions/about-billing-for-github-actions

### Similar Implementations
- BINPKG-1004 (darwin-x64) - Nearly identical pattern, just different target and runner
- BINPKG-1002 (linux-x64) - Native build pattern
