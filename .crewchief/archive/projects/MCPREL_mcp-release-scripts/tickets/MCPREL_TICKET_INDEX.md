# MCPREL Ticket Index

Project: MCP Release Scripts Update
Status: Ready for Implementation

## Overview
Update `@crewchief/maproom-mcp` release scripts to create git commits and tags that trigger GitHub Actions workflows, replacing direct npm publishing.

## Tickets by Phase

### Phase 1: Script Implementation (Core)
**Goal**: Create functional release script

| Ticket | Title | Agent | Time | Status | Depends On |
|--------|-------|-------|------|--------|------------|
| MCPREL-1001 | Create release.js script for git-based releases | general-purpose | 30-45m | ⏳ Pending | - |
| MCPREL-1002 | Update package.json release scripts to use new release.js | general-purpose | 10-15m | ⏳ Pending | MCPREL-1001 |

**Deliverables**:
- `/workspace/packages/maproom-mcp/scripts/release.js` (new file)
- `/workspace/packages/maproom-mcp/package.json` (modified scripts section)

---

### Phase 2: Testing & Validation (Quality Assurance)
**Goal**: Verify scripts work correctly and trigger GitHub Actions

| Ticket | Title | Agent | Time | Status | Depends On |
|--------|-------|-------|------|--------|------------|
| MCPREL-2001 | Manual testing and validation of release scripts | general-purpose, unit-test-runner | 20-30m | ⏳ Pending | MCPREL-1001, MCPREL-1002 |

**Deliverables**:
- Test results documented in ticket
- Confirmation of GitHub Actions triggering
- Verification of published artifacts

**Important**: This triggers real GitHub Actions that publish to npm and Docker Hub. Test carefully.

---

### Phase 3: Documentation (Polish - Optional)
**Goal**: Document new workflow for developers

| Ticket | Title | Agent | Time | Status | Depends On |
|--------|-------|-------|------|--------|------------|
| MCPREL-3001 | Update documentation for new release workflow | general-purpose | 10-15m | ⏳ Optional | MCPREL-2001 |

**Note**: Evaluate during implementation. Skip if no developer documentation exists or process is self-explanatory.

**Deliverables**:
- Updated README (if applicable)
- Or documentation that ticket was evaluated and skipped

---

## Execution Order

### Sequential Path
```
MCPREL-1001 (Create release.js)
     ↓
MCPREL-1002 (Update package.json)
     ↓
MCPREL-2001 (Manual testing)
     ↓
MCPREL-3001 (Documentation - optional)
```

### Parallel Opportunities
None - all tickets are sequential dependencies.

---

## Total Project Estimate
- **Phase 1**: 45-60 minutes
- **Phase 2**: 20-30 minutes
- **Phase 3**: 10-15 minutes (if needed)
- **Total**: 1.5-2 hours

---

## Success Criteria

Project complete when:
1. ✅ `pnpm release:patch/minor/major` bumps version
2. ✅ Git commit created: `chore(release): bump version to X.Y.Z`
3. ✅ Git tag created: `vX.Y.Z`
4. ✅ Both pushed to origin
5. ✅ GitHub Actions workflows trigger:
   - `build-and-publish-maproom-mcp.yml` completes
   - `publish-maproom-mcp-image.yml` completes
6. ✅ Clear error messages on failure
7. ✅ Tested and verified working

---

## Release Flow

When developer runs `pnpm release:patch`:

```
Developer → release.js → Bump version → Git commit → Git tag → Git push
                                                                    ↓
                                                           GitHub Actions
                                                                 ↙     ↘
                                                    Build npm pkg    Build Docker
                                                         ↓                ↓
                                                    Publish npm     Push Docker Hub
                                                         ↓                ↓
                                                      4 platforms    2 platforms
```

---

## Planning References

- [README](../README.md) - Project overview and context
- [Analysis](../planning/analysis.md) - Problem space and GitHub Actions status
- [Architecture](../planning/architecture.md) - Script design and flow
- [Quality Strategy](../planning/quality-strategy.md) - Testing approach
- [Security Review](../planning/security-review.md) - Risk assessment
- [Implementation Plan](../planning/plan.md) - Detailed phases and tickets

---

## Files Affected

| File | Operation | Phase |
|------|-----------|-------|
| `packages/maproom-mcp/scripts/release.js` | CREATE | Phase 1 |
| `packages/maproom-mcp/package.json` | MODIFY (scripts) | Phase 1 |
| `packages/maproom-mcp/README.md` | MAYBE MODIFY | Phase 3 (optional) |

---

## Risk Summary

| Risk | Level | Mitigation |
|------|-------|------------|
| Breaking existing workflow | Low | Test on branch, easy rollback |
| Test triggers real publish | Medium | Coordinate with owner, use test tags |
| Git operation failures | Low | Clear errors, git handles recovery |
| JSON syntax errors | Very Low | Use Edit tool, verify after changes |

**Overall Risk**: Very Low

---

## Ticket Files

- `MCPREL-1001_create-release-script.md` - Core implementation
- `MCPREL-1002_update-package-json-scripts.md` - Configuration update
- `MCPREL-2001_manual-testing-and-validation.md` - Quality assurance
- `MCPREL-3001_update-documentation.md` - Documentation (optional)
- `MCPREL_TICKET_INDEX.md` - This index

---

## Next Steps

1. Review all tickets for clarity and completeness
2. Start with MCPREL-1001: Create release.js script
3. Execute tickets sequentially through phases
4. Test thoroughly before merging (MCPREL-2001)
5. Evaluate documentation needs (MCPREL-3001)
6. Monitor first real release after merge
