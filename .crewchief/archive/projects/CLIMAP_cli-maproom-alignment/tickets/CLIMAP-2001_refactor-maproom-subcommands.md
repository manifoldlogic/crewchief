# Ticket: CLIMAP-2001: Refactor maproom commands from colon-separated to subcommand pattern

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
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Convert maproom command registration from `maproom:scan` colon-separated pattern to `maproom scan` subcommand pattern to align with other CLI features (worktree, agent) and industry standards (git, docker, kubectl). Remove all old `maproom:*` commands and register 8 new subcommands including 3 previously unregistered commands (branch-watch, cache, generate-embeddings).

## Background
Currently, maproom uses the `maproom:scan`, `maproom:search` pattern which is inconsistent with other CLI features that use `worktree create`, `agent list` patterns. This inconsistency makes maproom feel like a separate tool rather than an integrated feature. The tool has no existing users, so a clean break to industry-standard subcommand patterns is the right architectural choice.

This ticket implements **Phase 2: Command Structure Refactoring** from the CLIMAP plan (section 2.1-2.3). This phase establishes the foundation for consistent CLI patterns that will scale as the tool grows. The refactoring aligns with industry standards used by git, docker, and kubectl.

**Dependency Context**: This work depends on CLIMAP-1001 (documentation updates) being completed first to ensure consistency between code and documentation.

## Acceptance Criteria
- [x] Parent `maproom` command registered with Commander.js
- [x] All 5 existing commands converted to subcommands (scan, search, upsert, watch, db)
- [x] Three new commands registered (branch-watch, cache, generate-embeddings)
- [x] Nested `db migrate` subcommand properly structured
- [x] All old `maproom:*` commands completely removed from codebase
- [x] Help text updated to show new syntax in examples
- [x] Arguments forward correctly to Rust binary via `runMaproomForward()`
- [x] `crewchief maproom --help` displays all subcommands

## Technical Requirements
- File to modify: `packages/cli/src/cli/maproom.ts`
- Use Commander.js nested command pattern
- Maintain pure forwarding model (no changes to forwarding logic)
- Commands to convert:
  - `maproom:scan` → `maproom scan`
  - `maproom:search` → `maproom search`
  - `maproom:upsert` → `maproom upsert`
  - `maproom:watch` → `maproom watch`
  - `maproom:db` → `maproom db migrate` (nested)
- New commands to add:
  - `maproom branch-watch` (auto-index on branch switch)
  - `maproom cache` (cache management)
  - `maproom generate-embeddings` (manual embedding generation)
- Each command must have `.allowUnknownOption(true)` for flag forwarding
- Each command must have `.argument('[args...]')` for argument forwarding
- Each command must include appropriate `.description()` text

## Implementation Notes

**Code Structure Reference:**
```typescript
export function registerMaproomCommands(program: Command) {
  const maproom = program.command('maproom')
    .description('Semantic code indexing and search')

  maproom.command('scan')
    .description('Scan and index repository files')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['scan', ...(args || [])]))

  maproom.command('search')
    .description('Semantic search across indexed code')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['search', ...(args || [])]))

  maproom.command('upsert')
    .description('Update specific files in the index')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['upsert', ...(args || [])]))

  maproom.command('watch')
    .description('Watch repository for changes')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['watch', ...(args || [])]))

  // Nested subcommand for database operations
  const db = maproom
    .command('db')
    .description('Database operations')

  db
    .command('migrate')
    .description('Run database migrations')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['db', 'migrate', ...(args || [])]))

  // New commands
  maproom
    .command('branch-watch')
    .description('Auto-index worktrees on branch switch')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['branch-watch', ...(args || [])]))

  maproom
    .command('cache')
    .description('Manage maproom caches')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['cache', ...(args || [])]))

  maproom
    .command('generate-embeddings')
    .description('Generate embeddings for indexed chunks')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['generate-embeddings', ...(args || [])]))
}
```

**Implementation Steps:**
1. Create parent command: `const maproom = program.command('maproom')`
2. Register each subcommand: `maproom.command('scan').action(...)`
3. For db, create nested structure: `maproom.command('db').command('migrate')`
4. Delete all old `program.command('maproom:*')` registrations
5. Update help text in `.addHelpText()` to use new syntax (if present)
6. Add descriptions for new commands (branch-watch, cache, generate-embeddings)
7. Verify forwarding works: args array should be `['scan', ...additionalArgs]`

**Key Implementation Details:**
- The `runMaproomForward()` function should receive the subcommand as the first element in the args array
- For nested commands like `db migrate`, the array should be `['db', 'migrate', ...args]`
- All existing forwarding logic remains unchanged - this is purely a routing refactor
- Help text should demonstrate the new command patterns

## Dependencies
- CLIMAP-1001 (Update CLI README environment variables) - Documentation should be updated first for consistency

## Risk Assessment
- **Risk**: Breaking change to command interface
  - **Mitigation**: Tool has no existing users, clean break is acceptable and preferred
- **Risk**: Arguments might not forward correctly to Rust binary
  - **Mitigation**: Integration tests verify argument forwarding, manual testing with `--help` flag
- **Risk**: Help text might be unclear or incomplete
  - **Mitigation**: Comprehensive descriptions and examples, manual verification of help output
- **Risk**: Nested subcommands (db migrate) might not work correctly
  - **Mitigation**: Follow Commander.js nested command pattern precisely, test help output

## Files/Packages Affected
- `/workspace/packages/cli/src/cli/maproom.ts` (primary changes - refactor command registration)

## Testing Requirements

**Manual Testing:**
- Verify `crewchief maproom --help` shows all 8 subcommands
- Verify arguments forward correctly (test with `--help` flag on each subcommand)
- Verify old `maproom:*` commands no longer exist (should show "unknown command" error)
- Manual test: `crewchief maproom scan --help` (should forward to Rust binary and show Rust help)
- Manual test: `crewchief maproom db migrate --help` (should forward nested subcommand)

**Integration Testing (if applicable):**
- Verify subcommand registration in CLI tests
- Verify argument forwarding to Rust binary
- Verify help text displays correctly

**Expected Test Output:**
```bash
# Should show maproom subcommands
$ crewchief maproom --help

# Should forward to Rust binary help
$ crewchief maproom scan --help

# Should show error for old syntax
$ crewchief maproom:scan
error: unknown command 'maproom:scan'
```

## Planning References
- Architecture doc: `.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/architecture.md` (Section 1: Command Structure Refactoring)
- Quality strategy: `.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/quality-strategy.md` (Integration test requirements)
- Plan: `.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/plan.md` (Phase 2, tasks 2.1-2.3)

## Estimated Effort
2-3 hours
