# Project: CLI GitHub Actions Release Automation

**Slug**: CLIREL
**Status**: Planning Complete
**Timeline**: 3-4 days (realistic estimate)

## Problem

The `@crewchief/cli` package currently uses a manual local release process that:
1. Only builds binaries for the developer's platform (incomplete cross-platform support)
2. Uses an unscoped package name (`crewchief` instead of `@crewchief/cli`)
3. Could conflict with `@crewchief/maproom-mcp` releases (both might use `v*.*.*` tags)
4. Requires manual `pnpm publish` command (error-prone)
5. Lacks validation before publishing (could ship broken binaries)
6. **Race condition**: `git push --follow-tags` can cause workflow trigger failures (tag arrives before commit)

**Impact**: Users on platforms other than the release developer's platform get a broken CLI with missing binaries.

## Solution

Migrate to automated GitHub Actions releases following the proven pattern from `@crewchief/maproom-mcp`:

**Key improvements**:
- ✅ Multi-platform builds (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- ✅ Package rename to `@crewchief/cli` (org convention)
- ✅ Package-scoped tags (`@crewchief/cli@v*.*.*` vs `@crewchief/maproom-mcp@v*.*.*`)
- ✅ Independent versioning (CLI and MCP can release separately)
- ✅ Automated validation (binary existence, size, execution tests)
- ✅ Zero manual steps after tag creation
- ✅ Proper deprecation of old `crewchief` package
- ✅ Fix race condition with two-step push (commits first, then tags separately)

**Breaking change**: Package rename from `crewchief` to `@crewchief/cli` warrants v1.0.0 release.

## Architecture

```
Developer                 GitHub Actions              npm Registry
   │                             │                         │
   │  1. pnpm release:minor      │                         │
   │     (creates tag)            │                         │
   ├────────────────────────────>│                         │
   │                             │                         │
   │  2. git push --tags         │                         │
   ├────────────────────────────>│                         │
   │                             │                         │
   │                        Matrix Build                   │
   │                        (4 platforms)                  │
   │                             │                         │
   │                        Validate                       │
   │                        Package                        │
   │                             │                         │
   │                        Publish                        │
   │                             ├────────────────────────>│
   │                             │                         │
   │                        Verify                         │
   │                             │<────────────────────────│
```

**Workflow structure**:
1. **Matrix build job**: Build Rust binaries for all 4 platforms (parallel)
2. **Validate-and-publish job**: Assemble package, validate, publish to npm

**Validation checks**:
- All 4 platform binaries exist
- Binary sizes in expected range (5-20MB)
- Native platform binary executes successfully
- TypeScript build complete
- Package structure correct

## Phases

1. **Old Package Deprecation** - Publish `crewchief@1.0.0` with deprecation warnings
2. **CLI Package Configuration** - Rename to `@crewchief/cli`, update metadata
3. **Release Script Updates** - Use package-scoped tags, remove manual publish
4. **CLI GitHub Actions Workflow** - Create multi-platform build workflow
5. **MCP Workflow Migration** - Update MCP to use package-scoped tags
6. **Security Hardening** - Tag protection, secrets, SECURITY.md
7. **Dry-Run Validation** - Test workflow without production publish
8. **Production Release** - First `@crewchief/cli@1.0.0` release
9. **Documentation** - Update README, archive project

## Agents

- **general-purpose**: Phases 1-3, 5-9 (configuration, scripts, testing, docs)
- **docker-engineer**: Phase 4 (GitHub Actions workflow - CI/CD expertise)

**Rationale**: docker-engineer specializes in CI/CD workflows, multi-platform builds, and binary distribution - perfect for creating the GitHub Actions workflow.

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem space, current state, research findings
- [architecture.md](planning/architecture.md) - System design, workflow structure, validation
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach, validation gates
- [security-review.md](planning/security-review.md) - Threat model, security baseline, incident response
- [plan.md](planning/plan.md) - Detailed phases, timeline, dependencies

## Success Metrics

**Functional**:
- Package `@crewchief/cli@1.0.0` published with all 4 platform binaries
- Independent tagging works (CLI and MCP workflows don't interfere)
- Old `crewchief` package properly deprecated

**Quality**:
- Zero manual steps after tag creation
- Validation prevents broken releases
- Workflow completes in <15 minutes

**Security**:
- NPM_TOKEN stored as GitHub secret
- Tag protection prevents unauthorized releases
- Security baseline documented

## Key Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Wrong binaries shipped | High | Multi-layered validation (existence, size, execution) |
| Tag triggers wrong workflow | Medium | Package-scoped tags, isolation testing |
| First release fails | Medium | Dry-run validation before production |
| NPM token compromised | Critical | GitHub secrets, 2FA on npm account |

## Dependencies

- Existing maproom-mcp workflow (template to copy)
- NPM_TOKEN secret (configured in repository)
- npm org ownership of `@crewchief` scope
- GitHub repository write access (for tags, workflows)

## Timeline

**Realistic estimate**: 3-4 days

- Configuration and scripts: ~4 hours
- Workflow creation and debugging: ~10 hours
- Security setup: ~2 hours
- Testing and validation: ~4 hours
- Production release and docs: ~3 hours
- Buffer for issues: ~2 hours

**Total**: ~25 hours (3-4 working days)

## Project Completion

Project complete when:
- [x] `@crewchief/cli@1.0.0` published to npm
- [x] All 4 platform binaries functional
- [x] Independent workflows operational
- [x] Security baseline implemented
- [x] Documentation complete
- [x] Old package deprecated
- [x] Project archived to `.crewchief/archive/`
