# Ticket: [WTCLEAN-1001]: Implement Binary Discovery Utility

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a binary discovery utility that finds the crewchief-maproom binary using multiple fallback strategies across all platforms (macOS, Linux, Windows).

## Background
The `worktree clean` command needs to call `crewchief-maproom db cleanup-stale` to remove stale database records. This requires discovering where the maproom binary is located.

This ticket implements Phase 1, Deliverable 1 from the plan: Binary discovery utility (`packages/cli/src/utils/maproom-binary.ts`).

**Note on code reuse:** This utility copies `findMaproomBinary()` from maproom-mcp as a pragmatic MVP decision. This creates temporary code duplication that will be consolidated when the MRBIN project completes.

## Acceptance Criteria
- [ ] `findMaproomBinary()` function exported from `packages/cli/src/utils/maproom-binary.ts`
- [ ] Finds binary from `CREWCHIEF_MAPROOM_BIN` environment variable (Strategy 1)
- [ ] Finds platform-specific packaged binary in `bin/{platform}-{arch}/` directory (Strategy 2)
- [ ] Finds development builds in `target/release/` directories (Strategy 3)
- [ ] Falls back to command name `crewchief-maproom` for PATH lookup (Strategy 4)
- [ ] Returns `null` when binary not found (doesn't crash)
- [ ] Handles Windows `.exe` extension correctly
- [ ] Uses `fs.existsSync()` to verify binary exists before returning path

## Technical Requirements
- New file: `packages/cli/src/utils/maproom-binary.ts`
- Export function: `findMaproomBinary(): string | null`
- Use Node.js `fs`, `path` modules
- Check `process.env.CREWCHIEF_MAPROOM_BIN` first
- Check `process.platform` and `process.arch` for packaged binary path
- Handle Windows by appending `.exe` to executable name
- Return absolute paths for found binaries
- Return command name (`crewchief-maproom`) as fallback for PATH lookup
- Return `null` only if no strategies found the binary

## Implementation Notes
Copy the implementation from `packages/maproom-mcp/src/utils/process.ts`:

```typescript
export function findMaproomBinary(): string | null {
  // Strategy 1: Environment variable
  if (process.env.CREWCHIEF_MAPROOM_BIN) {
    const binPath = process.env.CREWCHIEF_MAPROOM_BIN
    if (fs.existsSync(binPath)) return binPath
  }

  // Strategy 2: Platform-specific packaged binary
  const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'
  const packagedPath = path.join(__dirname, '..', 'bin', `${process.platform}-${process.arch}`, execName)
  if (fs.existsSync(packagedPath)) return packagedPath

  // Strategy 3: Development builds
  const devPaths = [
    './target/release/crewchief-maproom',
    '../../../crates/maproom/target/release/crewchief-maproom',
  ]
  for (const devPath of devPaths) {
    if (fs.existsSync(devPath)) return path.resolve(devPath)
  }

  // Strategy 4: System PATH (will be tried by spawnSync)
  return 'crewchief-maproom'  // Fallback to command name
}
```

**Platform considerations:**
- macOS: `darwin` platform, `arm64` or `x64` arch
- Linux: `linux` platform, `x64` or `arm64` arch
- Windows: `win32` platform, `x64` arch, `.exe` extension required

**Decision rationale:**
- Multi-strategy fallback ensures binary found in most scenarios
- Explicit file existence checks prevent returning invalid paths
- Returning command name as fallback allows `spawnSync` to check PATH
- This approach is proven (already used in maproom-mcp)

## Dependencies
- None (foundational infrastructure)

## Risk Assessment
- **Risk**: Binary discovery fails on untested platform combinations
  - **Mitigation**: Cover all common platforms (darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64) in tests, manual verification on each platform before release
- **Risk**: Windows `.exe` handling missed
  - **Mitigation**: Explicit test case for Windows platform, manual testing on Windows
- **Risk**: Code duplication with maproom-mcp
  - **Mitigation**: Document as temporary, create follow-up ticket after MRBIN completes

## Files/Packages Affected
- `packages/cli/src/utils/maproom-binary.ts` (new file)
- `packages/cli/src/utils/index.ts` (export new utility if applicable)

## Verification Notes
Verify-ticket agent should check:
- [ ] File `packages/cli/src/utils/maproom-binary.ts` exists
- [ ] Function `findMaproomBinary()` is exported
- [ ] Code handles all 4 fallback strategies in order
- [ ] Windows `.exe` extension logic present
- [ ] Function returns `string | null` type
- [ ] No TypeScript compilation errors
- [ ] Code is well-commented explaining each strategy
