# Ticket: VSCEXT-3001: Implement startup reconciliation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement TypeScript-based startup reconciliation that runs before the watch process starts. This catches up on any file changes that occurred while the extension was inactive.

## Background
When VSCode reopens, files may have changed since the extension last indexed. The reconciliation process uses `git diff` to find changed files and the existing `upsert` CLI command to index them before starting watch mode.

Reference: planning/plan.md - Phase 3, Ticket 3001
Reference: planning/architecture.md - TypeScript-Based Startup Reconciliation

## Acceptance Criteria
- [ ] `reconcileChanges(context)` function created in `src/process/reconcile.ts`
- [ ] Reads last indexed commit from VSCode workspace state
- [ ] Uses `git diff --name-only` to find changed files
- [ ] Spawns `crewchief-maproom upsert` with correct arguments
- [ ] Updates last indexed commit in workspace state after success
- [ ] Gracefully handles first run (no last commit stored)
- [ ] Gracefully handles no changed files

## Technical Requirements
- Store last commit in: `context.workspaceState.get<string>('maproom.lastIndexedCommit')`
- Git commands via `child_process.exec`:
  - `git rev-parse HEAD` - Get current commit
  - `git diff --name-only <last-commit>..HEAD` - Get changed files
- Upsert CLI: `crewchief-maproom upsert --commit <COMMIT> --repo <REPO> --worktree <WORKTREE> --root <ROOT> --paths <PATHS>`
- Reuse existing git utilities: `getRepoName()`, `getBranchName()` from `utils/git.ts`

## Implementation Notes

```typescript
// src/process/reconcile.ts
import { exec } from 'child_process'
import { promisify } from 'util'
import * as vscode from 'vscode'
import { getRepoName, getBranchName } from '../utils/git'
import { spawn } from 'child_process'

const execAsync = promisify(exec)

export async function reconcileChanges(context: vscode.ExtensionContext): Promise<void> {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) return

  const repoName = await getRepoName(workspaceRoot)
  const branchName = await getBranchName(workspaceRoot)

  // 1. Get last indexed commit (stored in extension state)
  const lastCommit = context.workspaceState.get<string>('maproom.lastIndexedCommit')

  if (!lastCommit) {
    // First run - skip reconciliation, watch will handle initial indexing
    return
  }

  // 2. Get current HEAD
  const { stdout: headCommit } = await execAsync('git rev-parse HEAD', { cwd: workspaceRoot })
  const currentHead = headCommit.trim()

  if (lastCommit === currentHead) {
    // No changes since last run
    return
  }

  // 3. Get changed files
  const { stdout: diffResult } = await execAsync(
    `git diff --name-only ${lastCommit}..HEAD`,
    { cwd: workspaceRoot }
  )
  const changedFiles = diffResult.split('\n').filter(Boolean)

  if (changedFiles.length === 0) {
    // Update commit even if no files changed (e.g., merge commits)
    await context.workspaceState.update('maproom.lastIndexedCommit', currentHead)
    return
  }

  // 4. Run upsert for changed files
  await spawnUpsert(changedFiles, repoName, branchName, workspaceRoot, currentHead)

  // 5. Update last indexed commit
  await context.workspaceState.update('maproom.lastIndexedCommit', currentHead)
}

async function spawnUpsert(
  files: string[],
  repo: string,
  worktree: string,
  root: string,
  commit: string
): Promise<void> {
  // Get binary path (reuse existing logic from orchestrator)
  const binaryPath = getBinaryPath()

  return new Promise((resolve, reject) => {
    const proc = spawn(binaryPath, [
      'upsert',
      '--commit', commit,
      '--repo', repo,
      '--worktree', worktree,
      '--root', root,
      '--paths', files.join(','),
    ], {
      env: {
        ...process.env,
        MAPROOM_DATABASE_URL: getDatabaseUrl(),
      }
    })

    proc.on('close', (code) => {
      if (code === 0) resolve()
      else reject(new Error(`upsert exited with code ${code}`))
    })
  })
}
```

## Dependencies
- VSCEXT-2001 (Refactored ProcessOrchestrator for consistent binary path resolution)

## Risk Assessment
- **Risk**: Git diff returns too many files
  - **Mitigation**: Upsert handles batch processing; worst case is slow startup
- **Risk**: Git commands fail (not a git repo)
  - **Mitigation**: Graceful error handling, skip reconciliation

## Files/Packages Affected
- `packages/vscode-maproom/src/process/reconcile.ts` - New file
- `packages/vscode-maproom/src/process/reconcile.test.ts` - Unit tests
- `packages/vscode-maproom/src/process/index.ts` - Export reconcile function
