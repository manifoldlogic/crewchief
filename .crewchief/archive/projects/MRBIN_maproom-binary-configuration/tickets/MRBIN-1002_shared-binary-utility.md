# Ticket: [MRBIN-1002]: Implement Shared Binary Resolution Utility

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create a new shared utility at packages/cli/src/utils/maproom-binary.ts that implements maproom binary resolution with clear precedence order: env var > config > global install > packaged binary.

## Background
Currently, maproom binary resolution is duplicated across maproom.ts (~86 lines) and worktrees.ts (~40 lines) with inconsistent logic. This ticket creates a single shared utility that consolidates the resolution logic and implements the new priority order that prefers global installs over packaged binaries.

This utility will be used by both CLI consumers in Phase 2, eliminating code duplication and providing consistent behavior across all maproom commands.

## Acceptance Criteria
- [x] New file created at packages/cli/src/utils/maproom-binary.ts
- [x] findMaproomBinary() function implements correct precedence order
- [x] Function handles Windows platform (uses .exe suffix)
- [x] Function handles Unix platforms (no suffix)
- [x] Function validates config paths and warns if invalid
- [x] Function returns null if no binary found
- [x] Function returns source information for debugging
- [x] TypeScript types exported (MaproomBinaryOptions, BinaryResolutionResult)
- [x] Platform detection uses process.platform and process.arch

## Technical Requirements
- Precedence order: CREWCHIEF_MAPROOM_BIN > configPath > global install > packaged binary
- Platform handling: Windows uses crewchief-maproom.exe, Unix uses crewchief-maproom
- Path validation: Use fs.existsSync() to check binary existence
- Global detection: Use spawnSync('bash', ['-lc', 'command -v crewchief-maproom'])
- Relative path resolution: Resolve from config file location
- Warning emission: Use logger.warn() for invalid config paths
- Packaged paths: Check platform-specific directories (linux-x64, darwin-arm64, etc.)

## Implementation Notes
Create interface and implementation as specified in architecture.md:

```typescript
export interface MaproomBinaryOptions {
  configPath?: string  // from config.repository.maproomBinaryPath
}

export interface BinaryResolutionResult {
  path: string | null
  source: 'env' | 'config' | 'global' | 'packaged' | 'not-found'
}

export function findMaproomBinary(options?: MaproomBinaryOptions): BinaryResolutionResult
```

Reuse existing patterns from maproom.ts and worktrees.ts for platform detection and path resolution. The packaged paths should check:
1. bin/{platform-arch}/crewchief-maproom
2. bin/crewchief-maproom
3. ../maproom-mcp/bin/{platform-arch}/crewchief-maproom

Handle arch mapping: x64 → amd64 on some platforms, handle gracefully.

## Dependencies
- MRBIN-1001 (Config schema must exist)

## Risk Assessment
- **Risk**: Platform detection incorrect for some environments
  - **Mitigation**: Reuse existing platform detection logic exactly, test on multiple platforms
- **Risk**: Packaged path resolution fails on some platforms
  - **Mitigation**: Check multiple path variants, maintain existing packaged paths
- **Risk**: Global detection fails in non-bash shells
  - **Mitigation**: Use same approach as existing code, tested in production

## Files/Packages Affected
- packages/cli/src/utils/maproom-binary.ts (NEW)

## Verification Notes
Verify that:
1. Function signature matches specified interface
2. All four precedence levels are implemented correctly
3. Platform detection works for Windows and Unix
4. Function returns appropriate source information
5. Function handles missing binaries gracefully
6. No external dependencies beyond Node built-ins (fs, path, child_process)
7. TypeScript compilation succeeds
8. Code follows existing utility patterns in packages/cli/src/utils/
