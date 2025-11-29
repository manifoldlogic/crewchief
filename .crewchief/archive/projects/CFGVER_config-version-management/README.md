# CFGVER: Config Version Management (Simplified)

## Problem

The Maproom MCP CLI uses cached configs at `~/.maproom-mcp/`. When users run `npx -y @crewchief/maproom-mcp@latest`, stale configs cause connection failures.

**Real incident (Oct 30, 2024)**: docker-compose.yml changed from local builds to published images. Users with cached configs failed because pattern-based detection missed the change.

## Solution (Simplified)

**Goal**: Detect version changes and update configs automatically, preserving user customizations.

**Approach**:
1. Store package version in `~/.maproom-mcp/.version` file
2. On CLI startup, compare package version to cached version
3. If mismatch: copy all configs from package, preserve user `.env` if exists
4. Write new version number

**What We're NOT Doing** (yet):
- No backups/rollback (if update fails, user can re-run)
- No file integrity verification (trust package contents)
- No Docker cleanup (containers restart automatically)
- Minimal testing (just ensure it works)

## Implementation

**Timeline**: 1-2 days
**Tickets**: 4 tickets
**Risk**: Low (worst case: user re-runs `npx` command)

### Tickets

1. **CFGVER-001**: Version tracking (`.version` file)
2. **CFGVER-002**: Version comparison logic
3. **CFGVER-003**: Config update with `.env` preservation
4. **CFGVER-004**: CLI integration

## Success Criteria

- ✅ Detects version changes automatically
- ✅ Updates configs when version changes
- ✅ Preserves user `.env` customizations
- ✅ No more config drift incidents

## Future Enhancements

If needed, we have a comprehensive plan archived at:
`.crewchief/archive/projects/CFGVER_config-version-management-comprehensive/`

That plan includes:
- Backup/rollback mechanism
- File integrity verification
- Docker volume cleanup
- Comprehensive testing (80% coverage)
- Security hardening
