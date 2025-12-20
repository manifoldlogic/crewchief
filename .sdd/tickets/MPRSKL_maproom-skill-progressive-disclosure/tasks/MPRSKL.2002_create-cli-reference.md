# Task: [MPRSKL.2002]: Create cli-reference.md

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only task)
- [x] **Verified** - all 8 acceptance criteria met

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general
- verify-task
- commit-task

## Summary
Create comprehensive CLI reference documentation covering all crewchief-maproom commands, flags, and options with examples, organized by command category (search, index, manage).

## Background
As part of the progressive disclosure restructure, detailed CLI documentation moves from SKILL.md to a dedicated reference file. This allows agents to find complete command details when needed without cluttering the main skill overview.

The CLI reference serves as the authoritative documentation for all crewchief-maproom command-line usage, accessible via link from SKILL.md.

**References:** plan.md Phase 2, Task 4; architecture.md Progressive Disclosure Structure

## Acceptance Criteria
- [x] File created at `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/cli-reference.md`
- [x] Documents all crewchief-maproom commands (status, scan, search, vector-search, context, daemon, etc.)
- [x] All command flags documented with type, default value, and description
- [x] Examples provided for each command showing common usage patterns
- [x] Commands organized by category: Search, Indexing, Management, Daemon
- [x] Flag documentation includes both long and short forms where applicable
- [x] Environment variables documented (MAPROOM_EMBEDDING_*, MAPROOM_DATABASE_URL)
- [x] All content verified against current CLI (`crewchief-maproom --help` and subcommand help)

## Technical Requirements
- Create new file at `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/cli-reference.md`
- Document all commands from `crewchief-maproom --help`
- Include flag syntax: `--flag-name <TYPE>` or `--flag-name` for boolean
- Show default values where relevant
- Use code blocks for all command examples
- Organize with clear section headings by command category
- Include introduction explaining purpose and scope

## Implementation Notes
**Document structure:**

```markdown
# Maproom CLI Reference

Complete reference for all crewchief-maproom commands and options.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| MAPROOM_DATABASE_URL | Path to SQLite database | ~/.maproom/maproom.db |
| MAPROOM_EMBEDDING_PROVIDER | Embedding provider (openai, ollama, google) | openai |
| MAPROOM_EMBEDDING_MODEL | Model name | (provider-specific) |
| MAPROOM_EMBEDDING_DIMENSION | Vector dimension | (inferred from model) |

## Search Commands

### search
Full-text search across indexed code.

**Usage:**
```bash
crewchief-maproom search --repo <REPO> --query "<QUERY>" [OPTIONS]
```

**Flags:**
- `--repo <REPO>` - Repository name to search
- `--query <QUERY>` - Search query string
- `--limit <N>` - Maximum results (default: 10)
- `--min-score <SCORE>` - Minimum relevance score (default: 0.0)
- `--json` - Output as JSON

**Examples:**
```bash
# Basic search
crewchief-maproom search --repo myproject --query "authentication"

# Limited results
crewchief-maproom search --repo myproject --query "error handling" --limit 5

# JSON output for scripting
crewchief-maproom search --repo myproject --query "validation" --json
```

### vector-search
Semantic search using embeddings.

[Continue for all commands...]

## Indexing Commands

### scan
Scan repository and update index.

[Full documentation...]

## Management Commands

### status
Show indexed repositories and statistics.

[Full documentation...]

## Daemon Commands

### daemon start
Start maproom background daemon.

[Full documentation...]
```

**Commands to document (from crewchief-maproom --help):**
- status
- scan
- search
- vector-search
- context
- daemon start/stop/status
- index
- (any other commands in current CLI)

**Critical accuracy requirement:** Run `crewchief-maproom --help` and each subcommand help to ensure all flags and descriptions match the actual CLI.

## Dependencies
- None (can be created independently)

## Risk Assessment
- **Risk**: Documentation becomes stale as CLI evolves
  - **Mitigation**: Verify against current CLI during creation; recommend periodic review process
- **Risk**: Missing newly added commands
  - **Mitigation**: Use `--help` output as source of truth; document all listed commands
- **Risk**: Examples don't work
  - **Mitigation**: Test example commands against actual CLI where possible

## Files/Packages Affected
- .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/cli-reference.md (new file)

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes
The verify-task agent should specifically check:

- [ ] File created in correct location (references/ subdirectory)
- [ ] All commands from `crewchief-maproom --help` are documented
- [ ] Each command section includes: usage syntax, flags, examples
- [ ] Flag descriptions match `--help` output
- [ ] Default values documented where applicable
- [ ] Environment variables section is complete
- [ ] Examples use consistent placeholder format (`<REPO>`, `<QUERY>`)
- [ ] Code blocks properly formatted
- [ ] Organization is logical (grouped by command purpose)
- [ ] Cross-references to other docs where appropriate

**Completeness verification:**
```bash
# List all commands
crewchief-maproom --help
# For each command, check subcommand help
crewchief-maproom status --help
crewchief-maproom scan --help
crewchief-maproom search --help
crewchief-maproom vector-search --help
crewchief-maproom context --help
crewchief-maproom daemon --help
# Verify all commands and flags appear in cli-reference.md
```

**Content accuracy check:**
- Compare flag names and types against help output
- Verify default values match CLI behavior
- Test at least one example per command category

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-20 | verify-task | FAIL | 7/8 acceptance criteria met, missing 3 cache subcommands (warm, invalidate, maintenance) and 4 migrate subcommands (rollback, list-backups, delete-backup, verify) |
| 2025-12-20 | verify-task | FAIL | 7/8 acceptance criteria met, missing cache warm subcommand (cache invalidate, maintenance, and all migrate subcommands have been added) |
| 2025-12-20 | verify-task | PASS | All 8 acceptance criteria met, cache warm subcommand documented (lines 449-473) |
