# Worktree Merge Command Specification

## Overview

The `crewchief worktree merge <name>` command provides a streamlined way to incorporate changes from a worktree back into its source branch, including both tracked and ignored files, and then clean up the worktree completely.

## Command Syntax

```bash
crewchief worktree merge <name> [options]
```

### Arguments
- `<name>`: The worktree selector (branch name, directory name, or path)

### Options
- `--no-copy-ignored`: Skip copying ignored files back to source
- `--dry-run`: Show what would be done without making changes
- `--strategy <type>`: Merge strategy (default: 'squash')
  - `squash`: Squash all commits into one (default)
  - `cherry-pick`: Cherry-pick individual commits
  - `ff`: Fast-forward merge if possible
- `--message <msg>`: Custom commit message (auto-generated if not provided)
- `--no-delete`: Keep the worktree after merging (for inspection)

## Implementation Plan

### 1. Core Workflow

```typescript
interface MergeWorktreeOptions {
  selector: string          // Worktree selector
  copyIgnored?: boolean      // Whether to copy ignored files back
  dryRun?: boolean          // Dry run mode
  strategy?: MergeStrategy  // Merge strategy
  message?: string          // Custom commit message
  noDelete?: boolean        // Keep worktree after merge
}
```

### 2. Implementation Steps

#### Phase 1: Validation and Preparation
1. **Resolve Worktree**
   - Use existing worktree resolution logic from `worktree cd` command
   - Ensure worktree exists and is valid
   - Get worktree path and branch name

2. **Check Prerequisites**
   - Verify source branch is clean (no uncommitted changes)
   - Ensure we're not currently inside the worktree being merged
   - Check that worktree has commits to merge

3. **Determine Source Branch**
   - Read worktree metadata to find which branch it was created from
   - Store this information when creating worktrees (enhancement needed)
   - Fallback: use config's mainBranch if metadata unavailable

#### Phase 2: Copy Ignored Files Back
1. **Reverse Copy Operation**
   - Use modified version of `copyIgnoredFiles` function
   - Copy FROM worktree TO source location
   - Respect same patterns and overwrite strategies
   - Track which files were copied for commit message

2. **Implementation Details**
   ```typescript
   async function copyIgnoredFilesBack(options: {
     worktreeRoot: string
     sourceRoot: string  
     config: CrewChiefConfig
     dryRun?: boolean
   }): Promise<CopyResult>
   ```

#### Phase 3: Git Merge Operations
1. **Generate Commit Message**
   - Auto-generate detailed message if not provided
   - Include:
     - Summary of changes
     - Number of commits being merged
     - Files changed statistics
     - List of ignored files copied back
   - Format:
     ```
     Merge worktree 'branch-name'
     
     Changes from worktree:
     - X commits merged
     - Y files changed, Z insertions(+), W deletions(-)
     
     Ignored files updated:
     - .env (modified)
     - .claude/config.json (added)
     
     Source branch: main
     Worktree branch: cc-feature-20250117123456
     ```

2. **Perform Merge**
   - Switch to source branch
   - Apply selected merge strategy
   - Use GitMergeService with enhancements
   - Handle merge conflicts gracefully

3. **Cleanup Operations**
   - Delete worktree branch (local and remote if exists)
   - Remove worktree using `git worktree remove`
   - Clean up directory
   - Run `git worktree prune` for good measure

### 3. File Structure Changes

```
packages/cli/src/
├── cli/
│   └── worktree.ts          # Add merge command
├── git/
│   ├── merge.ts             # Enhance merge service
│   ├── worktrees.ts         # Add source branch tracking
│   └── copy-ignored-files.ts # Add reverse copy functionality
└── utils/
    └── worktree-metadata.ts  # New: Store/retrieve worktree metadata
```

### 4. Metadata Storage

Store worktree metadata in `.crewchief/worktrees/<name>/.crewchief-meta.json`:
```json
{
  "sourceBranch": "main",
  "createdAt": "2025-01-17T12:34:56Z",
  "createdFrom": "/path/to/source",
  "baseBranch": "main",
  "purpose": "agent" | "manual"
}
```

### 5. Error Handling

1. **Pre-merge Checks**
   - Dirty working tree → Prompt to stash or commit
   - No commits to merge → Exit gracefully
   - Merge conflicts → Provide clear instructions

2. **File Copy Errors**
   - Permission issues → Warn but continue
   - Missing files → Log and continue
   - Pattern errors → Validate early

3. **Git Operation Errors**
   - Network issues → Retry with timeout
   - Branch protection → Clear error message
   - Missing permissions → Helpful guidance

### 6. Testing Strategy

1. **Unit Tests**
   - Worktree resolution
   - Metadata storage/retrieval
   - Reverse file copy logic
   - Commit message generation

2. **Integration Tests**
   - Full merge workflow
   - Different merge strategies
   - Ignored files handling
   - Error scenarios

3. **Manual Test Cases**
   - Merge with modified .env files
   - Merge with no changes
   - Merge with conflicts
   - Dry run mode verification

### 7. Usage Examples

```bash
# Basic merge with squash (default)
crewchief worktree merge feature-branch

# Merge without copying ignored files back
crewchief worktree merge feature-branch --no-copy-ignored

# Dry run to see what would happen
crewchief worktree merge feature-branch --dry-run

# Cherry-pick commits with custom message
crewchief worktree merge feature-branch --strategy cherry-pick --message "Add new feature"

# Merge but keep worktree for inspection
crewchief worktree merge feature-branch --no-delete
```

### 8. Integration with Existing Commands

- **worktree create**: Store metadata about source branch
- **worktree list**: Show source branch info
- **agent commands**: Use merge for agent completions
- **competition merge**: Reuse core merge logic

### 9. Future Enhancements

1. **Interactive Mode**
   - Preview changes before merging
   - Select which ignored files to copy
   - Choose commits to include

2. **Conflict Resolution**
   - Built-in merge conflict UI
   - AI-assisted conflict resolution
   - Automatic resolution strategies

3. **Audit Trail**
   - Log all merge operations
   - Track ignored file changes
   - Generate merge reports

### 10. Dependencies

- `simple-git`: Git operations
- `glob`: File pattern matching
- `ignore`: Gitignore parsing
- `chalk`: Terminal output formatting
- `commander`: CLI framework

## Implementation Priority

1. **Phase 1** (Core Functionality)
   - Basic merge command
   - Squash strategy only
   - Manual ignored files copy

2. **Phase 2** (Enhanced Features)
   - All merge strategies
   - Automatic ignored files handling
   - Metadata storage

3. **Phase 3** (Polish)
   - Dry run mode
   - Better error messages
   - Interactive features