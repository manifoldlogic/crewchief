# Decisions: cli improvements

Running log of key decisions made during this initiative.

---

## [2025-12-05] Default Worktree Location Change

**Context:** Worktrees stored in `.crewchief/worktrees` clutter the repository workspace and interfere with IDE indexing.

**Decision:** Change default to `~/.crewchief/worktrees/<repo-name>/`

**Rationale:**
- Moves worktrees outside repository, reducing clutter
- Repository-specific subdirectory prevents conflicts across projects
- `~` expansion is standard practice in CLI tools
- Centralized location makes disk space management easier

**Alternatives Considered:**
- Keep current default: Rejected - causes ongoing friction
- Use `/tmp` or OS temp directory: Rejected - worktrees should persist across reboots
- No `<repo-name>` subdirectory: Rejected - would cause conflicts between repos

---

## [2025-12-05] Auto-Scan Default Behavior

**Context:** `worktree use` currently auto-scans by default, causing unexpected delays and confusion.

**Decision:** Make auto-scan opt-in with `autoScanOnWorktreeUse` config (default: false)

**Rationale:**
- Scanning can be slow on large codebases
- Automatic behavior is unexpected and not documented
- Users can explicitly scan with `crewchief maproom scan`
- Opt-in provides control without removing capability

**Alternatives Considered:**
- Keep auto-scan default: Rejected - violates principle of least surprise
- Remove auto-scan entirely: Rejected - some users may want this convenience
- Auto-scan only on create, not use: Rejected - still unexpected behavior

---

## [2025-12-05] Maproom Binary Config Location

**Context:** Need config option for maproom binary path, but unclear which config section.

**Decision:** Add `repository.maproomBinaryPath` (not in worktree section)

**Rationale:**
- Maproom binary is a repository-level tool, not worktree-specific
- Consistent with other tool configurations
- `repository` section already exists in schema
- Aligns with environment variable `CREWCHIEF_MAPROOM_BIN` (repository-wide)

**Alternatives Considered:**
- `worktree.maproomBinaryPath`: Rejected - not worktree-specific
- Top-level config field: Rejected - `repository` section more organized
- `maproom.binaryPath`: Rejected - no `maproom` section exists

---

## [2025-12-05] Binary Resolution Order

**Context:** Current order prefers local packaged binary over global install, causing confusion in development.

**Decision:** New order: env var → config → global → packaged

**Previous Order:**
1. `CREWCHIEF_MAPROOM_BIN`
2. Packaged binary
3. Global install

**New Order:**
1. `CREWCHIEF_MAPROOM_BIN`
2. `config.repository.maproomBinaryPath`
3. Global install
4. Packaged binary

**Rationale:**
- Environment variable always takes precedence (explicit override)
- Config file is next most explicit (per-project setting)
- Global install is production default (predictable)
- Packaged binary is fallback (for npm package users)

**Alternatives Considered:**
- Keep current order: Rejected - favors development over production
- Remove packaged binary entirely: Rejected - needed for npm installs

---

## [2025-12-05] Complete Cleanup Strategy

**Context:** `worktree clean` leaves orphaned maproom records and git branches.

**Decision:** Clean command deletes: directory + git worktree + branch + maproom records

**Rationale:**
- Single command for complete cleanup matches user expectations
- Prevents database bloat and stale search results
- Reduces cognitive load (don't need to remember multiple commands)
- Best-effort approach prevents failures from blocking cleanup

**Alternatives Considered:**
- Separate cleanup command: Rejected - more commands to remember
- Clean only on flag: Rejected - complete cleanup should be default
- Fail if maproom unavailable: Rejected - cleanup more important than completeness

**Flags Added:**
- `--keep-branch`: Preserve git branch
- `--keep-maproom`: Skip database cleanup (for testing)

---

## [2025-12-05] Cleanup Implementation Approach

**Context:** Can call maproom cleanup via daemon or direct binary spawn.

**Decision:** Use direct binary spawn for initial implementation

**Rationale:**
- Simpler implementation (no daemon dependency)
- Fewer failure modes (daemon might not be running)
- Cleanup is infrequent operation (performance not critical)
- Can enhance with daemon support later

**Alternatives Considered:**
- Daemon client only: Rejected - adds dependency, requires daemon running
- Hybrid (try daemon, fallback to binary): Rejected - adds complexity for marginal benefit

---

## [2025-12-05] Branch Deletion Default Behavior

**Context:** Should clean command delete branch by default or require flag?

**Decision:** Delete branch by default, warn if unmerged, provide `--keep-branch` flag

**Rationale:**
- Matches user intent when cleaning up (remove everything)
- Warning about unmerged work provides safety
- Opt-out flag available for special cases
- Consistent with `merge` command which also deletes branch

**Alternatives Considered:**
- Require `--delete-branch` flag: Rejected - makes cleanup incomplete by default
- Never delete branch: Rejected - leaves cleanup half-done
- Delete only if merged: Rejected - too conservative, branches often experimental

---

## [2025-12-05] Path Template vs Fixed Default

**Context:** Should default path support templates like `{repo}` and `{branch}`?

**Decision:** Use fixed default with `<repo-name>` placeholder, no runtime templates

**Rationale:**
- Simpler implementation (no template parsing)
- `<repo-name>` is resolved at worktree creation time
- Users who need complex paths can use config with absolute paths
- Avoids introducing template language/syntax

**Alternatives Considered:**
- Full template system: Rejected - over-engineered for simple use case
- No placeholder: Rejected - all repos would share same worktree directory
- Function-based config: Rejected - complexity not justified

---

## [2025-12-05] Backward Compatibility Strategy

**Context:** Changes affect existing users with worktrees in `.crewchief/worktrees`.

**Decision:** Maintain backward compatibility via config, provide migration guide

**Rationale:**
- Existing worktrees continue to work (git tracks absolute paths)
- New worktrees use new default (gradual migration)
- Users can explicitly set old path in config if desired
- Migration is optional, not forced

**Migration Path:**
1. Update config default (new installations get new path)
2. Document migration steps in README
3. No automatic migration (too complex, risky)
4. No deprecation warnings (old path remains valid)

**Config Override:**
```javascript
{
  repository: {
    worktreeBasePath: '.crewchief/worktrees'  // Keep old behavior
  }
}
```
