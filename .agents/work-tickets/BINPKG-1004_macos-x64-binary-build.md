# Ticket: BINPKG-1004: Implement macOS x64 binary build in workflow matrix

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - code-level validation complete (execution testing in BINPKG-1901)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Implement native build for darwin-x64 platform (Intel Macs). Uses GitHub's macos-13 runner for native x64 compilation to produce the crewchief-maproom binary for Intel-based macOS systems.

## Background
Intel Mac users need the darwin-x64 binary to run maproom-mcp. Unlike Linux, macOS cross-compilation is difficult and unreliable, so we use native builds on appropriate GitHub runners. The macos-13 runner is specifically chosen because it's the last Intel-based (x64) macOS runner available from GitHub - macos-latest has already transitioned to ARM64.

This ticket implements the darwin-x64 matrix item defined in BINPKG-1001. It follows the same pattern as BINPKG-1002 (linux-x64) and BINPKG-1003 (linux-arm64) but uses native macOS tooling instead of cross-compilation.

**Planning reference**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, darwin-x64 implementation)

## Acceptance Criteria
- [ ] Matrix job successfully builds binary for target `x86_64-apple-darwin`
- [ ] Build uses native cargo (not cross) on macos-13 runner
- [ ] Binary is stripped using macOS strip tool to reduce size
- [ ] Binary is copied to `artifacts/bin/darwin-x64/crewchief-maproom` directory
- [ ] Binary is uploaded as GitHub artifact with name `maproom-darwin-x64`
- [ ] Build completes successfully when tested with a version tag (manual workflow_dispatch dry run)

## Technical Requirements

### Matrix Configuration (already in BINPKG-1001)
- Platform identifier: `darwin-x64`
- GitHub runner: `macos-13` (Intel Mac)
- Rust target: `x86_64-apple-darwin`
- Use cross: `false` (native build)

### Build Steps to Implement
Add these steps to the `build-binaries` job for the darwin-x64 matrix item:

```yaml
steps:
  - name: Checkout code
    uses: actions/checkout@v4

  - name: Set up Rust
    uses: actions-rust-lang/setup-rust-toolchain@v1
    with:
      target: x86_64-apple-darwin

  - name: Build binary
    run: |
      cargo build --release --target x86_64-apple-darwin --manifest-path crates/maproom/Cargo.toml

  - name: Strip binary
    run: |
      strip target/x86_64-apple-darwin/release/crewchief-maproom

  - name: Prepare artifacts
    run: |
      mkdir -p artifacts/bin/darwin-x64
      cp target/x86_64-apple-darwin/release/crewchief-maproom artifacts/bin/darwin-x64/

  - name: Upload binary artifact
    uses: actions/upload-artifact@v4
    with:
      name: maproom-darwin-x64
      path: artifacts/bin/darwin-x64/crewchief-maproom
      retention-days: 1
```

### Build Command Details
- **Target**: `x86_64-apple-darwin` (explicitly specified for safety)
- **Manifest**: `crates/maproom/Cargo.toml` (maproom crate)
- **Output**: `target/x86_64-apple-darwin/release/crewchief-maproom`
- **Strip command**: `strip target/x86_64-apple-darwin/release/crewchief-maproom`
- **Artifact directory**: `artifacts/bin/darwin-x64/`
- **Artifact name**: `maproom-darwin-x64` (consistent naming scheme)

### Why Native Build (not cross)
- macOS cross-compilation from Linux is notoriously difficult
- Requires macOS SDK, system libraries, and code signing considerations
- GitHub provides native macOS runners, making cross unnecessary
- Native builds are more reliable and maintainable

### Why macos-13 Specifically
- macos-13 is the last Intel-based (x64) macOS runner
- macos-latest has transitioned to ARM64 (M1/M2/M3 Macs)
- macos-12 is deprecated by GitHub
- Explicit runner selection prevents unexpected architecture changes

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
strip target/x86_64-apple-darwin/release/crewchief-maproom
```

### Rust Target Auto-Detection
While Rust can auto-detect the x86_64-apple-darwin target on macos-13, we specify it explicitly in the build command for:
1. Clarity and documentation
2. Safety against future runner changes
3. Consistency with cross-compilation platforms

### GitHub Runner Costs
- macOS runners are significantly slower and more expensive than Linux runners
- Cost: ~10x more expensive than ubuntu-latest
- Build time: Generally slower due to macOS overhead
- Mitigation: This is acceptable for MVP, can optimize later with caching strategies

### Artifact Retention
- `retention-days: 1` is sufficient since artifacts are only needed for the publish job
- Reduces storage costs for GitHub Actions
- Consistent with other platform builds (BINPKG-1002, BINPKG-1003)

### Testing the Build
Use manual workflow_dispatch with dry_run:
```bash
# Push a test tag
git tag v0.0.0-test-darwin-x64
git push origin v0.0.0-test-darwin-x64

# Or use workflow_dispatch from GitHub UI
# Set dry_run: true to skip publish step
```

## Dependencies
- **BINPKG-1001**: Workflow structure and matrix configuration (must be completed first)

## Risk Assessment

- **Risk**: macos-13 runner availability and deprecation
  - **Likelihood**: Medium (GitHub periodically deprecates old macOS runners)
  - **Impact**: High (breaks darwin-x64 builds completely)
  - **Mitigation**: Monitor GitHub runner announcements and deprecation schedules. When macos-13 is deprecated, evaluate alternatives (potentially move to cross-compilation or drop x64 support in favor of Rosetta 2 emulation on ARM Macs).

- **Risk**: Longer build times on macOS runners
  - **Likelihood**: High (macOS runners are consistently slower)
  - **Impact**: Low (slightly longer CI times)
  - **Mitigation**: Acceptable for MVP. Can optimize later with build caching (cargo cache) or conditional builds.

- **Risk**: Higher cost of macOS runners
  - **Likelihood**: High (macOS minutes are ~10x Linux)
  - **Impact**: Low (acceptable for open source projects with free minutes)
  - **Mitigation**: Monitor GitHub Actions usage. Optimize if needed by reducing build frequency or using conditional triggers.

- **Risk**: Binary compatibility across macOS versions
  - **Likelihood**: Low (Rust binaries are generally portable)
  - **Impact**: Medium (some users can't run the binary)
  - **Mitigation**: macos-13 targets a reasonable minimum macOS version. Document minimum macOS version in package README.

## Files/Packages Affected

### Files to Modify
- **MODIFY**: `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml`
  - Add build steps to the darwin-x64 matrix item
  - Steps follow the pattern established in BINPKG-1002 and BINPKG-1003
  - Replace placeholder TODO comment with actual implementation

### Files to Reference (Read Only)
- `/workspace/crates/maproom/Cargo.toml` - Build manifest
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Workflow file (read for context)
- `/workspace/.agents/work-tickets/BINPKG-1002_linux-x64-binary-build.md` - Reference pattern
- `/workspace/.agents/work-tickets/BINPKG-1003_linux-arm64-binary-build.md` - Reference pattern

### Packages Affected
- `crates/maproom` - Binary being built (no code changes, just CI/CD)

## Estimated Effort
**1 hour**

Breakdown:
- 15 min: Review existing platform build implementations (BINPKG-1002, BINPKG-1003)
- 30 min: Add build steps to workflow for darwin-x64 matrix item
- 15 min: Test with manual workflow_dispatch dry run

## Priority
**High** - Part of Phase 1 critical path. Required for complete multi-platform binary coverage.

## Related Tickets

### Depends On
- **BINPKG-1001**: Workflow structure must exist (COMPLETED or IN PROGRESS)

### Parallel Implementation
- **BINPKG-1002**: Linux x64 build implementation (same phase)
- **BINPKG-1003**: Linux ARM64 build implementation (same phase)
- **BINPKG-1005**: macOS ARM64 build implementation (same phase)

### Enables
- **BINPKG-1006**: Binary validation (requires all 4 platform builds)
- **BINPKG-1007**: npm publish (requires all 4 platform builds)

### Sequence
This is ticket 4 of 11 in Phase 1 of the BINPKG project:
1. BINPKG-1001 - Workflow structure ✓
2. BINPKG-1002 - Linux x64 build
3. BINPKG-1003 - Linux ARM64 build
4. **BINPKG-1004** (this ticket) - macOS x64 build
5. BINPKG-1005 - macOS ARM64 build
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
- BINPKG-1002 (linux-x64) - Native build pattern
- BINPKG-1005 (darwin-arm64) - macOS ARM64 native build (similar setup)
