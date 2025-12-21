# Ticket: Repository Public Readiness

**Ticket ID:** CLEANUP
**Status:** Planning Complete
**Created:** 2025-12-20

## Summary

Systematically clean up the CrewChief repository to prepare it for public release. This involves removing accumulated development artifacts, consolidating documentation, eliminating duplicate project management systems, and ensuring no sensitive data is exposed.

## Problem Statement

The repository has accumulated significant development artifacts that would confuse external contributors and potentially expose internal project context:

- **100+ archived project directories** in `.crewchief/archive/`
- **Genetic optimization run data** with hundreds of log/JSON files
- **Multiple overlapping systems**: `.crewchief/`, `.sdd/`, `.agent/`, `.claude/`
- **Scattered documentation** across multiple directories
- **Configuration files** with local paths (`.mcp.json`)
- **Log and backup files** committed to the repository

## Proposed Solution

A four-phase cleanup approach:

1. **Security Audit** - Scan for and remove any sensitive data before public exposure
2. **Artifact Removal** - Delete development artifacts, archive project history to separate branch
3. **Documentation Cleanup** - Consolidate and organize documentation
4. **Final Verification** - Ensure all tests pass and repository is clean

Key decisions:
- Retain `.crewchief/` as primary project system, but remove archive from main
- Create `archive/project-history` branch to preserve historical project data
- Delete genetic optimization run data entirely
- Keep component docs near code, but improve `/docs` index

## Relevant Agents

- `file-operations-agent` - File operations, scanning, removal
- `verify-task` - Test suite execution, validation
- `commit-task` - Commit changes with proper messages

## Deliverables

Work products created during ticket execution:

See [deliverables/](deliverables/) for:
- `security-audit-report.md` - Security scan results
- `final-verification-report.md` - Test and validation results

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis and findings
- [architecture.md](planning/architecture.md) - Cleanup approach and decisions
- [plan.md](planning/plan.md) - Four-phase execution plan
- [quality-strategy.md](planning/quality-strategy.md) - Testing and verification approach
- [security-review.md](planning/security-review.md) - Security assessment

## Key Cleanup Targets

### High Priority (Security)
- Remove `.env` file (empty but shouldn't exist)
- Remove/sanitize `.mcp.json` configs
- Verify no API keys or credentials exposed

### High Priority (Size)
- Remove `.crewchief/archive/projects/` (~100 directories)
- Remove `packages/cli/.crewchief/genetic-iterations/`
- Remove all `.log` files

### Medium Priority (Organization)
- Remove `.claude/` directory (deprecated by `.agent/`)
- Remove `.bak` files
- Update `.gitignore` to prevent re-accumulation

### Low Priority (Polish)
- Create `/docs/README.md` index
- Audit and clean `/scripts/` directory
- Add CONTRIBUTING.md with cleanup guidelines

## Tasks

See [tasks/](tasks/) for all ticket tasks (to be created by task-creator).

## Success Criteria

- [ ] Zero security findings in final scan
- [ ] Repository size reduced by >50MB
- [ ] All tests pass (`pnpm test`, `cargo test`)
- [ ] Build succeeds (`pnpm build`, `cargo build`)
- [ ] No `.log`, `.bak`, `.tmp` files in repo
- [ ] Archive branch created with historical data
- [ ] Documentation index exists

## Next Steps

**Recommended:** Run `/sdd:review CLEANUP` before creating tasks to validate planning documents.
