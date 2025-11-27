# Ticket: VSCEXT-4001: Remove Docker code

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (N/A - deletion only)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- verify-ticket
- commit-ticket

## Summary
Delete all Docker-related code from the extension, including the DockerManager class, related tests, and any imports/references throughout the codebase.

## Background
With the move to SQLite-only database and host Ollama, Docker is no longer needed. The DockerManager class and related code should be completely removed to reduce maintenance burden and eliminate confusion.

Reference: planning/plan.md - Phase 4, Ticket 4001
Reference: planning/analysis.md - Cruft from Previous Architecture

## Acceptance Criteria
- [x] `src/docker/` directory deleted entirely
- [x] No imports from docker module anywhere in codebase
- [x] No references to Docker in any TypeScript files
- [x] TypeScript compiles without errors
- [x] No docker-related configuration in package.json

## Technical Requirements

**Files to Delete**:
- `src/docker/manager.ts` - DockerManager class
- `src/docker/index.ts` - Docker exports
- `src/docker/example-usage.ts` - Dead example code
- `src/docker/manager.test.ts` - Docker tests
- Entire `src/docker/` directory

**Files to Update** (remove Docker imports/references):
- `src/extension.ts` - Remove `ensureDockerRunning()` calls if any remain
- `src/services/index.ts` - Remove docker exports (if exists)
- Any other files with docker imports

**Verification Commands**:
```bash
# Find any remaining Docker references
grep -r "docker" packages/vscode-maproom/src/ --include="*.ts"
grep -r "DockerManager" packages/vscode-maproom/src/ --include="*.ts"
grep -r "ensureDockerRunning" packages/vscode-maproom/src/ --include="*.ts"

# Verify TypeScript compilation
cd packages/vscode-maproom && pnpm build
```

## Implementation Notes
1. First, search for all Docker references to understand scope
2. Delete the `src/docker/` directory
3. Fix any import errors that result
4. Remove any Docker-related code paths in extension.ts
5. Verify compilation succeeds
6. Run grep to confirm no Docker references remain

## Dependencies
- VSCEXT-3002 (Rewritten activation no longer uses Docker)

## Risk Assessment
- **Risk**: Hidden Docker dependencies break compilation
  - **Mitigation**: Grep verification before and after deletion
- **Risk**: Users with existing Docker setup confused
  - **Mitigation**: Clear changelog noting Docker removal

## Files/Packages Affected
- `packages/vscode-maproom/src/docker/` - Delete entire directory
- `packages/vscode-maproom/src/extension.ts` - Remove Docker imports/calls
- `packages/vscode-maproom/src/services/index.ts` - Remove exports (if applicable)
