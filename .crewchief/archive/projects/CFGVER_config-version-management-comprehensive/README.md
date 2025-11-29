# CFGVER: Config Version Management

## Problem

The Maproom MCP CLI uses cached configuration files at `~/.maproom-mcp/`. When users run `npx -y @crewchief/maproom-mcp@latest`, the CLI may use stale cached configs even though the npm package has updated. This causes **config drift** leading to:

- MCP server connection failures
- Docker containers failing to start
- Users must manually delete cache directory to recover

### Real-World Impact (October 30, 2024)

A config drift incident occurred when docker-compose.yml changed from local builds to published Docker images. Users with cached configs from before the change experienced connection failures. The CLI's pattern-based update detection missed this architectural change.

## Solution

Implement **version-based configuration management** using explicit version tracking instead of brittle pattern detection:

1. **Version Markers** - Add version comments to docker-compose.yml
2. **Version File** - Track package version in `.maproom-version` file
3. **Automatic Detection** - Compare versions on every CLI run
4. **Safe Updates** - Backup old config, replace with new, cleanup containers
5. **Rollback Mechanism** - Restore from backup if update fails

## Architecture

The solution uses three key components:

### 1. Version Tracking
- `.maproom-version` file tracks package version and file hashes
- Version comment in docker-compose.yml for manual verification
- SHA-256 hashes verify file integrity

### 2. Update Detection
- Compare package version to cached version on every run
- Verify file integrity via hashes
- Trigger update if version mismatch or corruption detected

### 3. Safe Update Process
- **Step 1:** Backup existing configs
- **Step 2:** Stop Docker containers
- **Step 3:** Copy new configs from package
- **Step 4:** Update version file
- **Step 5:** Cleanup old resources
- **Rollback:** Restore backup if any step fails

## Project Status

**Phase:** Planning Complete
**Next:** Implementation Phase 1 - Core Version Management

## Planning Documents

- **[Analysis](planning/analysis.md)** - Problem space, industry solutions, root cause analysis
- **[Architecture](planning/architecture.md)** - Technical design, components, integration points
- **[Quality Strategy](planning/quality-strategy.md)** - Testing approach, coverage goals, acceptance criteria
- **[Security Review](planning/security-review.md)** - Threat model, mitigations, security checklist
- **[Plan](planning/plan.md)** - Implementation phases, timeline, success metrics

## Implementation Plan

### Phase 1: Core Version Management (2-3 days)
- Version file schema and creation
- Version comparison logic
- File integrity checking
- Unit tests (80% coverage)

### Phase 2: Safe Update Process (3-4 days)
- Backup creation
- Config update logic
- Rollback mechanism
- Integration tests

### Phase 3: Docker Integration (2-3 days)
- Container stop logic
- Volume cleanup
- Error handling
- Docker tests

### Phase 4: CLI Integration (1-2 days)
- CLI entry point updates
- Progress messages
- Error messages
- Manual testing

### Phase 5: Testing & Validation (3-4 days)
- Complete test suite
- Manual testing
- Documentation
- CI/CD updates

### Phase 6: Release & Monitoring (1 day)
- npm publish
- Release notes
- User feedback

**Total Estimated Time:** 12-17 days (3 weeks)

## Success Criteria

### Functional
- Zero config drift incidents after release
- 100% success rate for first-run config creation
- 95%+ success rate for version updates
- 100% success rate for rollback

### Quality
- 80%+ code coverage for config-manager
- All critical paths covered by integration tests
- Zero high-severity security issues
- All manual test cases passing

### User Experience
- Clear progress messages during update
- Actionable error messages
- No user intervention required for normal updates
- Positive user feedback

## Relevant Agents

### Primary Implementers
- **database-engineer** - Core logic, file operations, version management
- **docker-engineer** - Docker integration, container management
- **mcp-tools-engineer** - CLI integration, user experience

### Quality Assurance
- **unit-test-runner** - Unit tests with vitest
- **integration-tester** - End-to-end scenarios
- **code-reviewer** - Code review, edge cases

### Support
- **documentation-engineer** - User documentation, release notes
- **security-specialist** - Security review, vulnerability assessment

## Technical Stack

- **Language:** JavaScript (Node.js)
- **Testing:** Vitest (unit), manual (integration)
- **Dependencies:** Node.js built-ins only (fs, path, crypto, child_process)
- **CI/CD:** GitHub Actions
- **Distribution:** npm registry

## Key Design Decisions

### Why Explicit Versions vs Pattern Detection?

Pattern detection (`includes('EMBEDDING_PROVIDER: ollama')`) is fragile:
- Requires maintaining list of patterns for each breaking change
- Misses architectural changes (build → image)
- False positives on unrelated content
- Tightly couples config structure to detection logic

Explicit versions are robust:
- Detects ALL changes automatically
- No maintenance burden
- Industry standard (npm, Docker, Kubernetes)
- Future-proof

### Why Backup Before Update?

Config updates can fail (Docker not running, permission errors, disk full). Backups enable:
- Safe rollback on failure
- User confidence (can recover)
- Debug aid (compare old vs new)
- Audit trail (last 5 updates)

### Why SHA-256 Hashing?

File integrity checking detects:
- Corrupted configs (bit rot)
- Manual edits (user changed file)
- Partial updates (update failed mid-way)

SHA-256 provides:
- Collision resistance (no practical attacks)
- Fast computation (config files are small)
- Standard in Node.js crypto module

## Related Issues

- **Config Drift Incident:** October 30, 2024 - docker-compose.yml change from build to image
- **Pattern Detection:** Current logic in `packages/maproom-mcp/bin/cli.cjs` lines 209-223

## Quick Links

- **Project Directory:** `.crewchief/projects/CFGVER_config-version-management/`
- **Planning:** `planning/`
- **Tickets:** `tickets/` (created after plan approval)

## Timeline

| Milestone | Target Date | Status |
|-----------|------------|--------|
| Planning Complete | ✅ Complete | Done |
| Phase 1: Core Logic | Week 1 | Pending |
| Phase 2: Update Process | Week 1-2 | Pending |
| Phase 3: Docker Integration | Week 2 | Pending |
| Phase 4: CLI Integration | Week 2 | Pending |
| Phase 5: Testing | Week 3 | Pending |
| Phase 6: Release | Week 3 | Pending |

**Target Ship Date:** 3 weeks from start

## Getting Started

Ready to start implementation?

1. Review all planning documents in `planning/`
2. Proceed with Phase 1 implementation
3. Follow ticket-driven development workflow
4. Mark acceptance criteria as complete
5. Request review before moving to next phase

---

*This project prevents config drift and ensures users always have up-to-date configurations when running `npx -y @crewchief/maproom-mcp@latest`.*
