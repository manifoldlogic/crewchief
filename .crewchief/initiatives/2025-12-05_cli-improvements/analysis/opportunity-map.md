# Opportunity Map: cli improvements

## Problem Spaces

### Problem 1: Worktree Storage Location Inflexibility
**Description:** Worktrees are currently stored in `.crewchief/worktrees` within the repository, which clutters the workspace and can cause issues with IDE indexing, file watchers, and disk space management.

**Impact:** Developers working on multiple branches simultaneously face repo bloat, slow IDE performance, and difficulty managing worktree disk usage. Repos with `.crewchief` in `.gitignore` still show untracked files.

**Current State:** Hardcoded default in `RepositorySchema` (`worktreeBasePath: '.crewchief/worktrees'`). No support for `~` expansion or centralized storage across projects.

### Problem 2: Uncontrolled Auto-Scanning
**Description:** `worktree use` automatically triggers maproom scans (line 143 in `worktrees.ts`), which can be slow and unexpected. Users have no way to disable this behavior.

**Impact:** Developers experience delays when switching between worktrees, especially on large codebases. Creates confusion when scan is not wanted.

**Current State:** Auto-scan is hardcoded in `createWorktree()` and `runMaproomScan()` is called unconditionally. No config option to control this behavior.

### Problem 3: Incomplete Cleanup
**Description:** `crewchief worktree clean` removes the directory and git worktree metadata but leaves orphaned records in maproom database. Users must manually clean maproom separately.

**Impact:** Database bloat over time, stale search results referencing non-existent worktrees, inconsistent state between git and maproom.

**Current State:** The `clean` command calls `removeWorktree()` and `removeDirSync()` but has no integration with maproom's `delete_worktree_data()` functionality.

### Problem 4: Maproom Binary Resolution Confusion
**Description:** Binary resolution logic prefers local build (`packages/cli/bin/`) over global installation, causing confusion when developers switch between development and production contexts.

**Impact:** Developers building locally for testing accidentally use old binaries. No explicit way to configure binary path beyond `CREWCHIEF_MAPROOM_BIN` environment variable.

**Current State:** `worktrees.ts` line 40-53 searches local paths before global. No config file option for binary path.

## Goals

### Goal 1: Flexible Worktree Storage
**Outcome:** Developers can store worktrees in a centralized location outside the repository, with support for `~` expansion and repository-specific subdirectories.

**Measurement:** Config validation passes for paths like `~/.crewchief/worktrees/<repo-name>/`, worktrees are created in expected locations, existing users can maintain current paths via config.

### Goal 2: Controlled Indexing
**Outcome:** Developers explicitly control when maproom scans happen, with auto-scan disabled by default and configurable per-project.

**Measurement:** `worktree use` completes without scanning unless enabled, config option `autoScanOnWorktreeUse` controls behavior, users can manually trigger scans with `maproom scan`.

### Goal 3: Complete Cleanup
**Outcome:** `crewchief worktree clean` removes all traces of a worktree: directory, git branch, and maproom database records.

**Measurement:** After cleanup, `git worktree list` shows no entry, `git branch` shows no branch, maproom database has no worktree record, search doesn't return results from deleted worktree.

### Goal 4: Explicit Binary Configuration
**Outcome:** Developers can configure maproom binary path in config file, with predictable resolution order that doesn't prefer local builds.

**Measurement:** Config option `maproomBinaryPath` is respected, resolution order is documented and consistent, development workflow docs show how to use local builds.

## Constraints

- Must maintain backward compatibility for existing worktrees in `.crewchief/worktrees`
- Path expansion must work cross-platform (Windows, macOS, Linux)
- Maproom cleanup should be best-effort (don't fail if maproom not available)
- Config schema changes must validate with Zod
- Must not break existing tests
- CLI commands should remain fast and responsive

## Opportunities

### Opportunity 1: Repository Name in Default Path
**Value:** Using `~/.crewchief/worktrees/<repo-name>/` as default allows developers to work on multiple repos without worktree name conflicts.

**Feasibility:** High - repo name is already available via git config, simple string interpolation.

### Opportunity 2: Unified Cleanup Command
**Value:** Single command that cleans all resources reduces cognitive load and prevents inconsistent state.

**Feasibility:** Medium - requires integration with maproom CLI/daemon, needs graceful error handling.

### Opportunity 3: Configuration File Precedence
**Value:** Layered config (global → project → local) lets teams share defaults while individuals override.

**Feasibility:** High - config loader already supports this pattern.

### Opportunity 4: Migration Helper
**Value:** Future enhancement could provide `crewchief worktree migrate` to move existing worktrees to new location.

**Feasibility:** Low priority - manual migration is straightforward, automated version requires complex git worktree repair logic.

### Opportunity 5: Pre-creation Path Validation
**Value:** Validate worktree paths before creation to catch permission errors, disk space issues early.

**Feasibility:** Medium - Node.js provides filesystem APIs, adds minimal overhead to create command.

### Opportunity 6: Smart Auto-Scan Defaults
**Value:** Future: enable auto-scan only for main/feature branches, disable for experimental branches.

**Feasibility:** Low priority - requires branch pattern matching, adds configuration complexity.
