# Analysis: Repository Public Readiness

## Problem Definition

The CrewChief repository has accumulated numerous artifacts from development that need cleanup before making the codebase publicly shareable. These include:

1. **Temporary/experimental files** that were committed but never cleaned up
2. **Documentation scattered** across multiple directories rather than consolidated in `/docs`
3. **Duplicate agent/workflow systems** (`.claude/`, `.agent/`, `.crewchief/`, `.sdd/`)
4. **Genetic optimization run artifacts** stored in package directories
5. **Sensitive configuration files** that may contain or reference credentials
6. **Outdated scripts and SQL files** that are no longer relevant

## Context

CrewChief is a CLI tool combining git worktree management and semantic code search (Maproom). The codebase has evolved through many iterations with different project management and agent systems:

- **`.crewchief/`** - Current project management system with ~100+ archived projects
- **`.agent/`** - "Antigravity" workflow system (migration from `.claude/`)
- **`.claude/`** - Legacy Claude-specific agents and commands
- **`.sdd/`** - SDD-based ticket system (newest)

This creates confusion and bloat that would confuse external contributors.

## Current State Assessment

### 1. Duplicate Management Systems

| Directory | Purpose | Status |
|-----------|---------|--------|
| `.crewchief/` | Project planning & tickets | Active, but has massive archive |
| `.agent/` | Antigravity workflows | Active (migrated from .claude) |
| `.claude/` | Claude agents/commands | Legacy, should be removed |
| `.sdd/` | SDD ticket system | Newest, appears active |

### 2. Genetic Optimization Artifacts

Found extensive run artifacts in `packages/cli/.crewchief/genetic-iterations/`:
- `premium-run-1762492225141/` - Multi-generation optimization data
- `ultra-run-1762742953256/` - 7 generations of variant data
- `ultra-run-1763154816350/` - 10 generations of variant data

These contain hundreds of `.log`, `.json`, and `.txt` files - valuable for research but inappropriate for a public repo.

### 3. Log Files in Repository

Found `.log` files committed to repo:
- `.sdd/logs/workflow.log`
- `.crewchief/archive/projects/COMPFIX_*/validation-results/*.log`
- `packages/cli/.crewchief/genetic-iterations/**/tool-usage.log`

### 4. Documentation Scattered

Documentation exists in multiple locations:
- `/docs/` - Main documentation (proper location)
- `/crates/maproom/docs/` - Rust component docs (29 files)
- `/packages/*/docs/` - Package-specific docs
- `/.crewchief/archive/projects/*/` - Historical project docs (should be cleaned/archived)

### 5. Potential Sensitive Files

- `.env` - Empty but exists in repo
- `.env.example` files - Contain placeholders (safe)
- `.mcp.json` - MCP configuration (may have local paths)
- `.cursor/mcp.json` - Cursor IDE config
- `.vscode/mcp.json` - VSCode config

### 6. Backup/Temporary Files

- `.crewchief/archive/projects/DAEMIGR_daemon-client-migration/README.md.bak`
- `.crewchief/scratchpad/TODO.md` - Active TODO items

### 7. Scripts and SQL Files Audit Needed

- `/scripts/` - 25+ scripts, need to verify if all are current
- `/crates/maproom/scripts/` - Database and analysis scripts
- Various `.sql` files for PostgreSQL (no longer primary DB)

## Research Findings

### Industry Best Practices for Open-Source Repos

1. **Minimal committed state** - Only essential files in version control
2. **Clear `.gitignore`** - Current `.gitignore` is extensive but genetic iterations should be added
3. **Consolidated docs** - Single `/docs` directory with clear structure
4. **No credentials** - No `.env` files with values, only `.env.example`
5. **README at root** - Clear getting started guide

### Existing Cleanup Patterns in Codebase

The codebase has cleanup scripts already:
- `scripts/cleanup-test-branches.sh`
- `crates/maproom/src/db/cleanup.rs`

And documentation about cleanup:
- `docs/admin-guide-cleanup.md`
- `docs/deployment-cleanup.md`

## Constraints

1. **Must not break functionality** - Build, test, and deploy must still work
2. **Must preserve valuable history** - Archive rather than delete completed project docs
3. **Must be reversible** - Use git branches so changes can be undone
4. **Time-sensitive** - Public release is desired

## Success Criteria

1. **Single project management system** - Decide on `.crewchief/` OR `.sdd/`, consolidate
2. **No committed log files** - All `.log` files in `.gitignore`
3. **No genetic run artifacts** - Move to external storage or delete
4. **Consolidated documentation** - Merge scattered docs into `/docs/`
5. **No empty/vestigial files** - Remove `.env`, unused scripts
6. **Security scan passes** - No credentials, API keys, or sensitive paths
7. **Clean repository root** - Minimal config files, clear structure
8. **Build and tests pass** - No regressions from cleanup
