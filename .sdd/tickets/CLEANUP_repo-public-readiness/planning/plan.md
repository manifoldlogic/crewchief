# Plan: Repository Public Readiness

## Overview

This plan organizes cleanup into four phases: Security Audit, Artifact Removal, Documentation Consolidation, and Final Verification. Each phase is designed to be independently valuable while building toward a public-ready repository.

## Phases

### Phase 1: Security Audit and Sensitive Data Removal

**Objective:** Identify and remove any credentials, secrets, or sensitive paths before public exposure.

**Deliverables:**
- `deliverables/security-audit-report.md` - Complete scan results
- Updated `.gitignore` with security-focused additions
- Removed `.env` file (keep `.env.example` files)
- Sanitized any MCP config files with local paths
- Verification that no API keys or tokens exist in codebase

**Agent Assignments:**
- `file-operations-agent`: Scan for secret patterns, generate report
- `file-operations-agent`: Remove/sanitize identified files

**Tasks:**
1. Grep for SECRET, PASSWORD, API_KEY, TOKEN patterns (non-test files)
2. Check all `.json` and `.yaml` for hardcoded credentials
3. Audit `.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`
4. Remove `.env` from repo (it's empty but shouldn't exist)
5. Update `.gitignore` to prevent sensitive files

---

### Phase 2: Bulk Artifact Removal

**Objective:** Remove accumulated development artifacts that bloat the repository.

**Deliverables:**
- Archive branch `archive/project-history` with historical project data
- Cleaned `.crewchief/` directory (archive folder removed from main)
- Removed `packages/cli/.crewchief/genetic-iterations/`
- Removed all `.log` files from repository
- Removed `.bak` files
- Removed legacy `.claude/` directory

**Agent Assignments:**
- `file-operations-agent`: Create archive branch, remove directories
- `file-operations-agent`: Delete log and backup files

**Tasks:**
1. Create `archive/project-history` branch from current state
2. On main: Delete `.crewchief/archive/projects/` (100+ project directories)
3. Delete `packages/cli/.crewchief/genetic-iterations/` entirely
4. Delete all `.log` files: `.sdd/logs/`, `packages/cli/.crewchief/**/*.log`
5. Delete `.crewchief/archive/projects/DAEMIGR_*/README.md.bak`
6. Evaluate and remove `.claude/` directory (deprecated by `.agent/`)
7. Update `.gitignore` to prevent re-accumulation

---

### Phase 3: Documentation and Structure Cleanup

**Objective:** Organize documentation and remove obsolete files.

**Deliverables:**
- Updated `/docs/README.md` with comprehensive index
- Consolidated or removed duplicate documentation
- Audit of `/scripts/` with removal of obsolete scripts
- Cleaned PostgreSQL-specific SQL scripts (SQLite is now primary)
- Updated project root documentation

**Agent Assignments:**
- `file-operations-agent`: Audit and organize documentation
- `file-operations-agent`: Clean scripts directory

**Tasks:**
1. Create documentation index in `/docs/README.md`
2. Identify and remove PostgreSQL-specific docs (deprecated)
3. Audit `/scripts/` - categorize as: active, deprecated, testing-only
4. Remove deprecated scripts (e.g., PostgreSQL validation scripts)
5. Update `/docs/CLAUDE.md` with new directory structure
6. Clean up `.crewchief/` structure documentation
7. Evaluate `/tests/manual/` - move or remove stale test reports

---

### Phase 4: Final Verification and Polish

**Objective:** Verify repository works correctly and is ready for public viewing.

**Deliverables:**
- `deliverables/final-verification-report.md` - Full test results
- Passing CI/CD pipeline
- Clean root directory listing
- Updated CONTRIBUTING.md with cleanup guidelines

**Agent Assignments:**
- `verify-task`: Run full test suite, report results
- `file-operations-agent`: Final documentation polish

**Tasks:**
1. Run `pnpm install && pnpm build` - verify build succeeds
2. Run `pnpm test` - verify all tests pass
3. Run `cargo build && cargo test` - verify Rust builds
4. Manual review of root directory structure
5. Security rescan - verify no regressions
6. Create/update CONTRIBUTING.md with contribution guidelines
7. Final commit message summarizing cleanup

---

## Dependencies

```
Phase 1 (Security)
    ↓
Phase 2 (Artifacts) - Depends on security being clean
    ↓
Phase 3 (Documentation) - Can partially overlap with Phase 2
    ↓
Phase 4 (Verification) - Must be last
```

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Accidentally delete needed file | Medium | High | Create archive branch first; all changes on feature branch |
| Break build/tests | Low | High | Run full test suite after each phase |
| Miss sensitive data | Low | High | Multiple grep scans with different patterns |
| Remove documentation still needed | Medium | Medium | Archive branch preserves everything; can restore |
| Merge conflicts with active work | Medium | Medium | Communicate cleanup window; minimize duration |

## Success Metrics

- [ ] Zero security findings in final scan
- [ ] Repository size reduced by >50MB
- [ ] All tests pass (`pnpm test`, `cargo test`)
- [ ] Build succeeds (`pnpm build`, `cargo build`)
- [ ] Root directory has <20 visible items
- [ ] No `.log`, `.bak`, `.tmp` files in repo
- [ ] Single project management system active
- [ ] Documentation index exists at `/docs/README.md`
- [ ] Archive branch created with historical data
- [ ] CONTRIBUTING.md exists with cleanup guidelines
