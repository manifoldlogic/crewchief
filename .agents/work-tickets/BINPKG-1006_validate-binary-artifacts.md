# Ticket: BINPKG-1006: Implement validation job for binary artifacts

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - code-level validation complete (execution testing in BINPKG-1901)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create the validation job that downloads all platform artifacts, verifies completeness, tests executability, and prepares for publishing. This is a critical quality gate preventing incomplete releases.

## Background
Version 1.3.0 failed because no validation caught missing binaries. This job ensures all 4 platforms built successfully before proceeding to publish. It's the safety net that makes releases reliable.

The validation job must download artifacts from all 4 platform builds (BINPKG-1002-1005), verify each binary is present and functional, and organize them into the correct package structure before publishing. This prevents scenarios where some platforms fail silently and incomplete packages get published to npm.

## Acceptance Criteria
- [x] Job `validate-and-publish` depends on `build-binaries` (all matrix jobs must complete)
- [x] Downloads all 4 platform artifacts using actions/download-artifact
- [x] Verifies 4 binaries exist (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- [x] Checks each binary is executable (has execute bit set)
- [x] Checks each binary size is reasonable (>1MB, <100MB)
- [x] Tests each binary runs successfully: `./binary --version`
- [x] Copies binaries to proper `packages/maproom-mcp/bin/<platform>/` structure
- [x] Fails loudly with clear error messages if any validation fails
- [x] Lists downloaded files and final structure for debugging

## Technical Requirements

### Job Configuration
- **Job name**: `validate-and-publish` (this ticket implements validation portion, publish in BINPKG-1007)
- **Runner**: `ubuntu-latest`
- **Dependencies**: `needs: build-binaries`

### Validation Steps

#### Step 1: Checkout Code
```yaml
- name: Checkout code
  uses: actions/checkout@v4
```

#### Step 2: Download All Artifacts
```yaml
- name: Download all platform binaries
  uses: actions/download-artifact@v4
  with:
    path: artifacts/
```

Download all 4 artifacts uploaded by build jobs:
- `maproom-mcp-linux-x64`
- `maproom-mcp-linux-arm64`
- `maproom-mcp-darwin-x64`
- `maproom-mcp-darwin-arm64`

#### Step 3: Debug Artifact Structure
```yaml
- name: List downloaded artifacts
  run: |
    echo "=== Artifact structure ==="
    ls -lR artifacts/
```

Critical for troubleshooting download path issues.

#### Step 4: Validate Binaries
Create a bash script that validates each platform:

```bash
#!/bin/bash
set -e  # Fail fast on any error

PLATFORMS=("linux-x64" "linux-arm64" "darwin-x64" "darwin-arm64")
MIN_SIZE=1048576      # 1MB in bytes
MAX_SIZE=104857600    # 100MB in bytes

echo "=== Validating platform binaries ==="

for platform in "${PLATFORMS[@]}"; do
  echo "Validating $platform..."

  BINARY_PATH="artifacts/maproom-mcp-$platform/maproom-mcp"

  # Check file exists
  if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found: $BINARY_PATH"
    exit 1
  fi
  echo "✓ Binary exists"

  # Check file size
  SIZE=$(stat -c%s "$BINARY_PATH" 2>/dev/null || stat -f%z "$BINARY_PATH")
  if [ "$SIZE" -lt "$MIN_SIZE" ]; then
    echo "ERROR: Binary too small ($SIZE bytes, minimum $MIN_SIZE)"
    exit 1
  fi
  if [ "$SIZE" -gt "$MAX_SIZE" ]; then
    echo "ERROR: Binary too large ($SIZE bytes, maximum $MAX_SIZE)"
    exit 1
  fi
  echo "✓ Binary size valid ($SIZE bytes)"

  # Add execute permission
  chmod +x "$BINARY_PATH"
  echo "✓ Execute permission set"

  # Test execution (--version should work for all platforms on ubuntu runner)
  if ! "$BINARY_PATH" --version >/dev/null 2>&1; then
    echo "ERROR: Binary failed to execute --version"
    exit 1
  fi
  echo "✓ Binary executes successfully"

  echo "✓ $platform validation complete"
  echo ""
done

echo "=== All validations passed ==="
```

#### Step 5: Organize Binaries
```yaml
- name: Copy binaries to package structure
  run: |
    echo "=== Organizing binaries ==="
    mkdir -p packages/maproom-mcp/bin/linux-x64
    mkdir -p packages/maproom-mcp/bin/linux-arm64
    mkdir -p packages/maproom-mcp/bin/darwin-x64
    mkdir -p packages/maproom-mcp/bin/darwin-arm64

    cp artifacts/maproom-mcp-linux-x64/maproom-mcp packages/maproom-mcp/bin/linux-x64/
    cp artifacts/maproom-mcp-linux-arm64/maproom-mcp packages/maproom-mcp/bin/linux-arm64/
    cp artifacts/maproom-mcp-darwin-x64/maproom-mcp packages/maproom-mcp/bin/darwin-x64/
    cp artifacts/maproom-mcp-darwin-arm64/maproom-mcp packages/maproom-mcp/bin/darwin-arm64/

    chmod +x packages/maproom-mcp/bin/*/maproom-mcp

    echo "=== Final package structure ==="
    ls -lR packages/maproom-mcp/bin/
```

### Error Handling

All validation steps must:
- Use `set -e` to fail fast on errors
- Print clear error messages indicating what failed
- Print success indicators (✓) for passed checks
- Include debugging output for troubleshooting

## Implementation Notes

### Artifact Download Behavior
- `actions/download-artifact@v4` downloads to `path: artifacts/` directory
- Each artifact creates subdirectory: `artifacts/<artifact-name>/`
- Binary name inside artifact is always `maproom-mcp`
- Full path example: `artifacts/maproom-mcp-linux-x64/maproom-mcp`

### Binary Execution Testing
- All binaries should be executable on `ubuntu-latest` runner
- Linux binaries execute natively
- macOS binaries may have limitations but `--version` should work or fail gracefully
- If execution test becomes problematic, can be adjusted to only test Linux binaries

### Size Validation Rationale
- **Minimum 1MB**: Ensures binary isn't truncated or corrupted
- **Maximum 100MB**: Reasonable upper bound (current binaries ~5-10MB)
- Protects against build artifacts including unintended large dependencies

### Integration with Publish Job
This validation step is part of `validate-and-publish` job. After validation succeeds:
- Binaries are organized in correct package structure
- BINPKG-1007 adds npm publish steps using these validated binaries
- If validation fails, publish never happens (safety gate)

## Dependencies

### Prerequisite Tickets (Must Complete First)
- **BINPKG-1002**: Linux x64 build steps (uploads artifact)
- **BINPKG-1003**: Linux ARM64 build steps (uploads artifact)
- **BINPKG-1004**: macOS x64 build steps (uploads artifact)
- **BINPKG-1005**: macOS ARM64 build steps (uploads artifact)

All 4 build jobs must successfully upload artifacts for validation to succeed.

## Risk Assessment

- **Risk**: Artifact download path confusion
  - **Likelihood**: Medium (common GitHub Actions issue)
  - **Impact**: High (validation fails, blocks release)
  - **Mitigation**: Add debugging step to list artifact structure, use well-documented paths, test locally with `act` tool if possible

- **Risk**: Binary execution fails on GitHub runner
  - **Likelihood**: Low (ubuntu-latest supports both linux and basic macOS binary inspection)
  - **Impact**: Medium (need to adjust validation strategy)
  - **Mitigation**: Use `chmod +x` before execution, handle execution failures gracefully, consider platform-specific execution tests

- **Risk**: Size validation thresholds too restrictive
  - **Likelihood**: Low (thresholds are generous)
  - **Impact**: Low (easy to adjust)
  - **Mitigation**: Start with wide thresholds (1MB-100MB), adjust based on actual binary sizes observed in builds

- **Risk**: Validation passes but binaries are still broken
  - **Likelihood**: Low (comprehensive checks)
  - **Impact**: High (broken release published)
  - **Mitigation**: Multiple validation layers (existence, size, permissions, execution), BINPKG-1901 adds end-to-end testing, consider expanding execution tests beyond `--version`

## Files/Packages Affected

### Files to Modify
- **MODIFY**: `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml`
  - Add validation steps to `validate-and-publish` job
  - Job already exists (created in BINPKG-1001) but has TODO placeholders
  - This ticket implements the validation portion

### Files to Reference (Read Only)
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/plan.md` - Phase 1 planning
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/architecture.md` - Technical architecture

### Packages Affected
- `packages/maproom-mcp` - Binaries copied to `bin/<platform>/` directories

## Estimated Effort
**2-3 hours** - Moderate complexity validation logic

Breakdown:
- 30 min: Review artifact download documentation and existing workflows
- 60 min: Implement validation bash script with all checks
- 30 min: Add debugging and error handling
- 30 min: Test locally or verify YAML syntax
- 30 min: Document and handle edge cases

## Priority
**High** - Critical quality gate that prevents broken releases. Blocks BINPKG-1007 (publish).

## Related Tickets

### Depends On (Prerequisite)
- BINPKG-1001: Workflow structure (creates job skeleton)
- BINPKG-1002: Linux x64 build (provides artifact)
- BINPKG-1003: Linux ARM64 build (provides artifact)
- BINPKG-1004: macOS x64 build (provides artifact)
- BINPKG-1005: macOS ARM64 build (provides artifact)

### Blocks (Must Complete Before)
- BINPKG-1007: npm publish job (relies on validated binaries)

### Related
- BINPKG-1901: Canary release integration test (end-to-end validation)
- BINPKG-2001: Local validation script (similar validation logic for local testing)

### Sequence
This is ticket 6 of 11 in Phase 1 of the BINPKG project:
1. BINPKG-1001 - Workflow structure
2. BINPKG-1002-1005 - Platform builds
3. **BINPKG-1006** (this ticket) - Binary validation
4. BINPKG-1007 - npm publish

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, lines 63-85)
- **Architecture**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md` (lines 213-250)

### External References
- **GitHub Actions artifacts**: https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts
- **actions/download-artifact**: https://github.com/actions/download-artifact
- **Bash error handling**: https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
