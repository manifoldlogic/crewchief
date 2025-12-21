# Architecture: Repository Public Readiness

## Overview

This cleanup effort follows a systematic approach: **Audit -> Categorize -> Remove/Relocate -> Verify**. The goal is to transform the repository from a development-focused state with accumulated artifacts into a clean, public-ready codebase while preserving functionality.

## Design Decisions

### Decision 1: Retain `.crewchief/` as Primary Project System

**Context:** Multiple overlapping systems exist (`.crewchief/`, `.sdd/`, `.agent/`, `.claude/`).

**Decision:** Keep `.crewchief/` for project/ticket management, but significantly reduce its footprint.

**Rationale:**
- `.crewchief/` has the most mature structure with clear conventions
- Massive archive can be compressed or moved to separate branch
- `.sdd/` is newest but less complete - can coexist for SDD-specific workflows
- `.claude/` is legacy and duplicates `.agent/` functionality
- `.agent/` is valuable for Antigravity workflows

### Decision 2: Archive-Branch Strategy for Historical Data

**Context:** Need to remove ~100 archived project directories but preserve history.

**Decision:** Create `archive/project-history` branch containing full `.crewchief/archive/` before removing from main.

**Rationale:**
- Git history alone makes recovery difficult
- Branch provides easy access if needed
- Main branch stays clean for public consumption
- Preserves valuable project context for future reference

### Decision 3: Delete Genetic Optimization Run Data

**Context:** `packages/cli/.crewchief/genetic-iterations/` contains hundreds of files from optimization runs.

**Decision:** Delete entirely from git history (optional) or just from working tree, add to `.gitignore`.

**Rationale:**
- Data is experiment-specific, not needed for repo function
- Contains no code, only run artifacts
- Significant size reduction (~1000+ files)
- Can be regenerated if optimization features are used again

### Decision 4: Consolidate Docs to `/docs`

**Context:** Documentation scattered across `/docs`, `/crates/*/docs/`, `/packages/*/docs/`.

**Decision:** Keep component docs in their packages but ensure `/docs` has comprehensive index.

**Rationale:**
- Component docs near code aids maintenance
- Central `/docs` for user-facing documentation
- Avoid massive merge that could cause conflicts
- Just need better discoverability

## Component Design

### 1. Cleanup Audit Script

**Purpose:** Generate comprehensive report of cleanup candidates.

**Responsibilities:**
- Identify all `.log`, `.bak`, `.tmp` files
- List genetic iteration directories
- Find empty files
- Detect potential secrets (grep for patterns)
- Output actionable cleanup list

**Interface:**
```bash
./scripts/audit-cleanup.sh > cleanup-report.md
```

### 2. .gitignore Enhancement

**Purpose:** Prevent future accumulation of cleanup targets.

**Additions needed:**
```gitignore
# Genetic optimization runs (already partially covered)
**/genetic-iterations/

# Log files
**/*.log
.sdd/logs/

# Editor/IDE local configs
.cursor/
.mcp.json

# Environment files
.env
!.env.example
```

### 3. Directory Structure (Post-Cleanup)

```
/
├── .crewchief/
│   ├── projects/          # Active projects only
│   ├── scratchpad/        # Temporary notes
│   └── README.md          # How to use
├── .sdd/                   # SDD workflows (if kept)
├── crates/
│   └── maproom/
│       └── docs/           # Keep component-specific docs
├── docs/                   # Main documentation hub
├── packages/
│   ├── cli/                # No .crewchief subdirs
│   ├── daemon-client/
│   ├── maproom-mcp/
│   └── vscode-maproom/
├── scripts/                # Only active, documented scripts
├── .gitignore              # Enhanced
├── CLAUDE.md               # AI guidance
├── README.md               # Main readme
└── ...config files...
```

## Data Flow

### Cleanup Workflow

```
1. Audit Phase
   ├── Run audit script
   ├── Generate cleanup-report.md
   └── Review with owner

2. Archive Phase
   ├── Create archive/project-history branch
   ├── Push archive content
   └── Document in README

3. Remove Phase
   ├── Delete archived projects from main
   ├── Delete genetic iterations
   ├── Delete log files
   ├── Delete backup files
   └── Remove legacy directories

4. Update Phase
   ├── Enhance .gitignore
   ├── Update docs index
   ├── Clean scripts directory
   └── Security scan

5. Verify Phase
   ├── Run full test suite
   ├── Build all packages
   ├── Manual smoke test
   └── Final security scan
```

## Integration Points

### Build System Integration

Cleanup must not affect:
- `pnpm install && pnpm build`
- `pnpm test`
- `cargo build` / `cargo test`
- GitHub Actions workflows
- Docker builds

### Security Scanning Integration

Post-cleanup, run:
- `git secrets --scan` (if available)
- grep for API key patterns
- Check all `.json` and `.yaml` files for hardcoded values

## Performance Considerations

### Repository Size Impact

Expected reductions:
- `.crewchief/archive/` removal: ~50-100MB of text files
- `genetic-iterations/` removal: ~10-50MB of JSON/logs
- Log files: ~1-5MB

### Clone Time

Public users will benefit from:
- Faster initial clone
- Reduced disk usage
- Cleaner file tree to navigate

## Maintainability

### Future Prevention

1. **Enhanced .gitignore** - Prevent re-accumulation
2. **CONTRIBUTING.md** - Document cleanup expectations
3. **CI check** - Optional: add script to detect cleanup violations
4. **Regular audits** - Quarterly cleanup review

### Documentation Updates Needed

- `/docs/CLAUDE.md` - Update guidance about project directories
- `/.crewchief/README.md` - Slim down, reference archive branch
- `/CONTRIBUTING.md` - Add cleanup guidelines for contributors
