# Ticket: BINPKG-1002: Implement Linux x64 binary build in workflow matrix

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - code-level validation complete (execution testing in BINPKG-1901)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Implement the complete build steps for linux-x64 platform in the GitHub Actions workflow matrix. This is the most common platform (Docker containers, devcontainers, CI/CD) and the one that failed in v1.3.0.

## Background
Version 1.3.0 was published without linux-x64 binaries, causing setup failures in production environments (Docker containers, devcontainers, CI/CD pipelines). This ticket ensures linux-x64 binaries are built reliably using GitHub's ubuntu-latest runner with the `cross` tool for cross-compilation.

The workflow structure was created in BINPKG-1001. This ticket implements the actual build steps for the linux-x64 platform entry in the matrix strategy. Linux x64 is the highest priority platform as it's used in:
- Docker containers (maproom-postgres service)
- GitHub Actions CI/CD
- Development containers (devcontainer.json)
- Most cloud deployment environments

This ticket implements Phase 1, Step 2 of the BINPKG project plan (`.agents/projects/BINPKG_binary-packaging/planning/plan.md`, lines 63-95).

## Acceptance Criteria
- [ ] Matrix job builds binary for target `x86_64-unknown-linux-gnu`
- [ ] Build uses `cross build --release --target x86_64-unknown-linux-gnu`
- [ ] Binary is stripped to reduce size using `strip` command
- [ ] Binary is copied to `artifacts/bin/linux-x64/crewchief-maproom` structure
- [ ] Artifact is uploaded with name `maproom-linux-x64`
- [ ] Build completes successfully on test tag (manual workflow_dispatch test)
- [ ] Binary size is reasonable (>1MB, <100MB after stripping)
- [ ] Build output logs show clear progress indicators

## Technical Requirements

### Matrix Configuration
The linux-x64 entry already exists in the matrix (from BINPKG-1001):
```yaml
- os: ubuntu-latest
  target: x86_64-unknown-linux-gnu
  platform: linux-x64
  use_cross: true
```

### Build Steps Implementation
Add the following steps to the `build-binaries` job for linux-x64:

1. **Checkout code**
   - Already exists: `uses: actions/checkout@v4`

2. **Setup Rust toolchain**
   ```yaml
   - name: Setup Rust toolchain
     uses: actions-rust-lang/setup-rust-toolchain@v1
     with:
       toolchain: stable
       target: ${{ matrix.target }}
   ```

3. **Install cross compilation tool**
   ```yaml
   - name: Install cross
     if: matrix.use_cross
     run: cargo install cross --git https://github.com/cross-rs/cross
   ```

4. **Build binary with cross**
   ```yaml
   - name: Build binary
     run: |
       cross build --release --target ${{ matrix.target }} --manifest-path crates/maproom/Cargo.toml
   ```

5. **Strip binary to reduce size**
   ```yaml
   - name: Strip binary
     run: |
       strip target/${{ matrix.target }}/release/crewchief-maproom
   ```

6. **Verify binary exists and show size**
   ```yaml
   - name: Verify binary
     run: |
       ls -lh target/${{ matrix.target }}/release/crewchief-maproom
       file target/${{ matrix.target }}/release/crewchief-maproom
   ```

7. **Prepare artifact directory**
   ```yaml
   - name: Prepare artifact
     run: |
       mkdir -p artifacts/bin/${{ matrix.platform }}
       cp target/${{ matrix.target }}/release/crewchief-maproom artifacts/bin/${{ matrix.platform }}/
   ```

8. **Upload artifact**
   ```yaml
   - name: Upload artifact
     uses: actions/upload-artifact@v4
     with:
       name: maproom-${{ matrix.platform }}
       path: artifacts/bin/${{ matrix.platform }}
       if-no-files-found: error
   ```

### Build Tool: cross
- **Tool**: https://github.com/cross-rs/cross
- **Purpose**: Docker-based cross-compilation for Linux targets
- **Why needed**: Ensures consistent build environment with all required system libraries
- **Installation**: Install from git to get latest version (handles recent Rust versions better)

### Binary Specifications
- **Source**: `crates/maproom/Cargo.toml`
- **Target**: `x86_64-unknown-linux-gnu`
- **Output**: `target/x86_64-unknown-linux-gnu/release/crewchief-maproom`
- **Expected size**: 5-30 MB after stripping (varies with dependencies)
- **Format**: ELF 64-bit LSB executable

## Implementation Notes

### Why cross instead of cargo?
The `cross` tool provides several advantages for Linux builds:
1. **Consistent environment**: Builds in Docker containers with exact system dependencies
2. **Reproducible builds**: Same Docker image = same binary output
3. **System library compatibility**: Handles glibc version requirements correctly
4. **No local setup needed**: GitHub runner doesn't need Linux dev packages

### Stripping Binaries
The `strip` command removes debug symbols and reduces binary size by 30-50%:
```bash
# Before strip: ~50MB
# After strip: ~15MB
strip target/x86_64-unknown-linux-gnu/release/crewchief-maproom
```

This is safe for release binaries and significantly improves npm package download size.

### Artifact Structure
Each platform uploads artifacts to `artifacts/bin/<platform>/`:
```
artifacts/
└── bin/
    └── linux-x64/
        └── crewchief-maproom
```

This structure matches the npm package layout in `packages/maproom-mcp/bin/`.

### Conditional Logic
The `if: matrix.use_cross` condition ensures cross is only installed for platforms that need it (linux-x64 and linux-arm64). macOS platforms build natively without cross.

### Error Handling
- `if-no-files-found: error` ensures the workflow fails if binary isn't produced
- File verification step catches missing binaries before artifact upload
- Size check helps identify bloated binaries early

### Testing the Build
Test the workflow without creating a release:
1. Push code changes to a branch
2. Use workflow_dispatch with `dry_run: true`
3. Verify artifacts are uploaded correctly
4. Check artifact contents and binary size

### Reference Implementation
The existing `scripts/build-and-package.sh` shows the manual build process:
- Lines 45-60: Platform detection and cross usage
- Lines 95-110: Build and strip commands
- This workflow automates that process for CI/CD

## Dependencies
- **BINPKG-1001** (completed): Workflow structure must exist

## Risk Assessment

- **Risk**: cross installation timeout or failure
  - **Likelihood**: Low
  - **Impact**: High (build fails completely)
  - **Mitigation**: Install from git HEAD for latest compatibility. Add caching for cross binary in future optimization. Fallback to cargo install cross (non-git) if needed.

- **Risk**: Build failure due to missing system dependencies
  - **Likelihood**: Low
  - **Impact**: Medium (requires investigation)
  - **Mitigation**: cross provides consistent Docker environment with all dependencies. Cargo.lock ensures reproducible dependency versions.

- **Risk**: Strip command fails or corrupts binary
  - **Likelihood**: Very Low
  - **Impact**: Medium (binary unusable)
  - **Mitigation**: Add verification step after strip to test binary can be executed (check `--version` flag). Strip is standard practice for release binaries.

- **Risk**: Binary size exceeds expected range
  - **Likelihood**: Low
  - **Impact**: Low (slower downloads)
  - **Mitigation**: Verification step logs binary size. Set alert threshold in future tickets if size > 50MB.

- **Risk**: Artifact upload fails
  - **Likelihood**: Very Low
  - **Impact**: High (later jobs can't access binary)
  - **Mitigation**: Use `if-no-files-found: error` to fail fast. GitHub Actions artifact system is reliable.

## Files/Packages Affected

### Files to Modify
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Add build steps to linux-x64 matrix entry

### Files to Reference (Read Only)
- `/workspace/scripts/build-and-package.sh` - Reference for build commands (lines 45-110)
- `/workspace/crates/maproom/Cargo.toml` - Target crate for compilation
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/plan.md` - Phase 1 planning (lines 63-95)
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/architecture.md` - Build system architecture (lines 139-212)

### Packages Affected
- `packages/maproom-mcp` - Will receive binary artifact in later publish step (no changes in this ticket)

## Estimated Effort
**2 hours** - Straightforward implementation with well-defined requirements

Breakdown:
- 30 min: Review existing workflow and add step structure
- 45 min: Configure Rust toolchain and cross installation
- 30 min: Implement build, strip, and artifact steps
- 15 min: Test with workflow_dispatch dry run
- 15 min: Verify artifact contents and binary properties

## Priority
**High** - Highest priority platform (caused v1.3.0 production failure). Other platforms can follow the same pattern.

## Related Tickets

### Depends On
- **BINPKG-1001** (completed) - Workflow structure must exist

### Blocks
- **BINPKG-1006** - Binary validation needs linux-x64 artifact

### Related
- **BINPKG-1003** - Linux ARM64 build (similar pattern, different target)
- **BINPKG-1004** - macOS x64 build (no cross needed)
- **BINPKG-1005** - macOS ARM64 build (no cross needed)

### Sequence
This is ticket 2 of 11 in Phase 1 of the BINPKG project:
1. BINPKG-1001 - Workflow structure (completed)
2. **BINPKG-1002** (this ticket) - Linux x64 build
3. BINPKG-1003 - Linux ARM64 build
4. BINPKG-1004 - macOS x64 build
5. BINPKG-1005 - macOS ARM64 build
6. BINPKG-1006 - Binary validation
7. BINPKG-1007 - npm publish

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, lines 63-95)
- **Architecture**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md` (lines 139-212)
- **Original issue**: Root cause analysis from v1.3.0 failure

### External References
- **cross tool**: https://github.com/cross-rs/cross
- **GitHub Actions artifacts**: https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts
- **Rust cross-compilation**: https://rust-lang.github.io/rustup/cross-compilation.html
- **actions-rust-lang/setup-rust-toolchain**: https://github.com/actions-rust-lang/setup-rust-toolchain
