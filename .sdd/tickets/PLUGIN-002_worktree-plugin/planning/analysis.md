# Analysis: Worktree Plugin

## Problem Definition

Claude Code currently has no way to discover or use crewchief's git worktree management capabilities. Users who want to:
- Create isolated branches for parallel development
- Safely merge worktree changes back to source branches
- Clean up worktrees without losing work
- Copy environment files between worktrees

...must manually invoke CLI commands without guidance on proper sequencing, safety checks, or best practices. The worktree lifecycle (create -> use -> work -> merge -> clean) has specific ordering requirements and safety considerations that are easy to get wrong.

## Context

Git worktrees enable parallel development by allowing multiple checkouts of the same repository. The crewchief CLI provides a robust worktree management layer that:
- Creates worktrees with metadata tracking (source branch, creation time)
- Copies ignored files (.env, secrets) to new worktrees
- Supports multiple merge strategies (fast-forward, squash, cherry-pick)
- Integrates with maproom database cleanup
- Prevents dangerous operations (deleting current worktree, merging from inside worktree)

This plugin makes these capabilities discoverable to Claude Code through the claude-code-plugins marketplace.

## Existing Solutions

### In Codebase

The crewchief CLI already provides complete worktree management:

**Command: `crewchief worktree create <name>`**
- Creates new branch and worktree directory
- Options: `--branch <base>`, `--base-path <dir>`, `--shell`, `--no-copy-ignored`
- Saves metadata (source branch, creation time, purpose)
- Optionally copies ignored files from source
- Source: `/packages/cli/src/cli/worktree.ts` lines 67-126

**Command: `crewchief worktree list`**
- Lists all active worktrees with branch names
- Shows running agents per worktree
- Source: `/packages/cli/src/cli/worktree.ts` lines 128-156

**Command: `crewchief worktree use <name>`**
- Switches to existing worktree
- Options: `--shell` (interactive subshell), `--print` (path output)
- Source: `/packages/cli/src/cli/worktree.ts` lines 432-568

**Command: `crewchief worktree clean <selector>`**
- Removes worktree directory and git metadata
- Options: `--all`, `--stale`, `--keep-dir`, `--keep-branch`, `--keep-maproom`
- Cleans maproom database records
- Deletes associated git branch (with safety checks)
- Source: `/packages/cli/src/cli/worktree.ts` lines 158-430

**Command: `crewchief worktree merge <name>`**
- Merges worktree back to source branch
- Options: `--strategy <ff|squash|cherry-pick>`, `--no-delete`, `--dry-run`, `--message`, `--yes`
- Copies ignored files back, performs merge, cleans up worktree
- Source: `/packages/cli/src/cli/worktree.ts` lines 641-872

**Command: `crewchief worktree copy-ignored <selector>`**
- Copies ignored files (.env, etc.) to existing worktree
- Options: `--dry-run`
- Source: `/packages/cli/src/cli/worktree.ts` lines 571-639

### In Industry

Git's native `git worktree` commands provide low-level worktree management but lack:
- Metadata tracking (which branch was the source)
- Ignored file copying
- Merge integration with cleanup
- Safety guardrails for common mistakes

Tools like `git-town` and `git-flow` focus on branching models rather than worktree lifecycle management.

## Current State

- **CLI Implementation**: Complete and stable (see source files above)
- **Plugin Infrastructure**: Maproom plugin provides proven pattern to follow
- **Marketplace**: Exists at `.crewchief/claude-code-plugins/` as git submodule
- **Documentation Gap**: No plugin exposes worktree capabilities to Claude Code

## Research Findings

### CLI Command Analysis

Analyzed `/packages/cli/src/cli/worktree.ts` (875 lines) and supporting files:

1. **Safety Mechanisms**:
   - Cannot delete current worktree (explicit check with `path.relative()`)
   - Cannot merge from inside worktree being merged
   - Branch deletion handles multiple edge cases (not merged, checked out elsewhere, protected)
   - Confirmation prompts for destructive operations

2. **Metadata Service** (`WorktreeMetadataService`):
   - Stores: sourceBranch, createdAt, createdFrom, baseBranch, purpose
   - Enables merge back to correct source branch
   - Located at: `/packages/cli/src/utils/worktree-metadata.ts`

3. **Integration Points**:
   - Maproom: `cleanMaproomRecords()` cleans stale database entries
   - Config: `copyIgnoredFiles` setting in `crewchief.config.js`
   - Git: Uses `simple-git` library for operations

### Maproom Plugin Pattern

Analyzed existing plugin at `.crewchief/claude-code-plugins/plugins/maproom/`:

1. **Structure**:
   ```
   plugins/maproom/
   ├── .claude-plugin/plugin.json
   ├── README.md
   └── skills/maproom-search/
       ├── SKILL.md
       └── references/search-best-practices.md
   ```

2. **plugin.json Requirements**:
   - name, version, description (required)
   - author object with name, email, url
   - repository URL
   - keywords array for discovery

3. **SKILL.md Requirements**:
   - YAML frontmatter with `name` (lowercase, hyphens) and `description` (<1024 chars)
   - Decision tree (when to use this vs alternatives)
   - Command reference with examples
   - Error handling guidance

## Constraints

### Technical Constraints

1. **Plugin Format**: Must follow Claude Code plugin specification
2. **SKILL.md Frontmatter**: Name must be lowercase with hyphens, description under 1024 characters
3. **CLI Dependencies**: Users must have crewchief CLI installed
4. **Git Repository**: Worktree commands only work in git repositories

### Business Constraints

1. **Documentation Only**: No code execution - plugin teaches usage patterns
2. **Safety Focus**: Must emphasize correct lifecycle ordering and safety checks
3. **Consistency**: Must match maproom plugin style and structure

### Resource Constraints

1. **Effort Estimate**: Medium (2-3 days) per epic summary
2. **Single Skill**: One skill covering complete worktree lifecycle

## Success Criteria

### Required

- [ ] Plugin directory structure matches specification:
  ```
  plugins/worktree/
  ├── .claude-plugin/plugin.json
  ├── README.md
  └── skills/worktree-management/SKILL.md
  ```
- [ ] plugin.json validates with name "worktree", version "0.1.0"
- [ ] SKILL.md frontmatter has valid name ("worktree-management") and description (<1024 chars)
- [ ] Description clearly states when to use this skill (worktree lifecycle, parallel development)
- [ ] Complete worktree lifecycle documented: create -> use -> work -> merge -> clean
- [ ] All 6 CLI commands documented with correct syntax and examples:
  - `crewchief worktree create`
  - `crewchief worktree list`
  - `crewchief worktree use`
  - `crewchief worktree clean`
  - `crewchief worktree merge`
  - `crewchief worktree copy-ignored`
- [ ] Safety checks documented:
  - Cannot delete current worktree
  - Cannot merge from inside worktree
  - Unmerged branches require `git branch -D`
  - Check for uncommitted changes before merge
- [ ] Common workflows have step-by-step examples:
  - Feature development workflow
  - Quick experiment workflow
  - Merge strategies comparison

### Quality

- [ ] Instructions use imperative form (verb-first)
- [ ] CLI examples are copy-paste ready
- [ ] No placeholder content remains
- [ ] Consistent with maproom plugin style
