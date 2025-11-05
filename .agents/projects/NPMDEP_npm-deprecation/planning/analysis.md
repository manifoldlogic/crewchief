# Analysis: npm Package Deprecation

## Problem Space

### Current Situation

The `maproom-mcp` package exists as an unscoped npm package but has been superseded by the scoped `@crewchief/maproom-mcp` package. The old package:

1. **No longer receives updates** - Development has moved to the scoped package
2. **May confuse users** - Users might install the old package thinking it's current
3. **No clear migration path** - Users installing the old package don't know about the new one
4. **Takes up namespace** - The unscoped name prevents others from using it

### User Impact

**Existing Users:**
- Users with `maproom-mcp` in their dependencies won't know to upgrade
- No automated way to discover the new package
- May continue using outdated version indefinitely

**New Users:**
- May discover old package first in npm search
- Will install deprecated version unknowingly
- Will miss out on new features and bug fixes

**npm Ecosystem:**
- Orphaned packages create noise in search results
- Unclear which package is canonical
- No standard deprecation message visible

## Industry Standards for Package Deprecation

### npm Best Practices

**1. Use `npm deprecate` Command**
- Adds warning message shown during installation
- Appears on npm website package page
- Doesn't prevent installation (users can still use if needed)
- Standard approach recommended by npm

**2. Publish Final "Tombstone" Version**
- Major version bump (signals significant change)
- README explaining deprecation
- Minimal executable that shows migration message
- Links to replacement package

**3. Clear Migration Path**
- Exact commands to install new package
- Link to new package documentation
- Explanation of what changed (just name vs. breaking changes)
- Timeline (if any) for when old package becomes unavailable

### Examples from Major Packages

**`request` → `got`/`axios`:**
- Published final version with deprecation notice
- Clear README explaining alternatives
- Used `npm deprecate` for all versions

**`babel-core` → `@babel/core`:**
- Published under new scoped namespace
- Deprecated old unscoped package
- Clear migration documentation

**`istanbul` → `nyc`:**
- Deprecated with explicit replacement recommendation
- README with migration guide
- Used npm deprecate command

## Current Project State

### Existing Setup

**Old Package (`maproom-mcp`):**
- Status: Published to npm, but not actively maintained
- Last version: Unknown (need to check npm)
- Users: Unknown count

**New Package (`@crewchief/maproom-mcp`):**
- Status: Actively developed
- Current version: 1.3.5
- Location: `/workspace/packages/maproom-mcp`
- Published as scoped package

### Assets Available

**Deprecation README:**
- Already created at `/workspace/packages/maproom-mcp/README.deprecated.md`
- Contains clear migration instructions
- Links to new package

**package.json:**
- Current package.json is for `@crewchief/maproom-mcp`
- Need separate package.json for deprecation publish

**npm Account Access:**
- Assumed: User has npm credentials
- Assumed: User has publish rights to `maproom-mcp`

## Research: npm Publishing Process

### Authentication

**Login Methods:**
1. `npm login` - Interactive, prompts for credentials
2. `npm adduser` - Alias for login
3. Environment variable: `NPM_TOKEN` for CI/CD
4. `.npmrc` file with auth token

### Publishing Steps

**Standard Flow:**
1. Create/update package.json
2. Create/update README.md
3. Test package locally: `npm pack`
4. Publish: `npm publish`
5. Deprecate: `npm deprecate <pkg> "<message>"`

**Scoped Package Considerations:**
- `@crewchief/maproom-mcp` is scoped (already handled)
- `maproom-mcp` is unscoped (target for deprecation)
- Different packages in npm registry

### Version Strategy

**Options:**
1. **Patch bump (e.g., 1.3.6)** - Minimal change signal
2. **Minor bump (e.g., 1.4.0)** - Feature-level change
3. **Major bump (e.g., 2.0.0)** - Breaking change signal ✅ **Recommended**

**Rationale for Major Version:**
- Signals significant change (deprecation)
- npm convention for "tombstone" versions
- Makes it clear this isn't a normal update
- Users expect breaking changes in major versions

## Key Constraints

### Technical Constraints

1. **Cannot modify old versions** - Already published versions are immutable
2. **Must publish new version** - Only way to update README on npm page
3. **npm deprecate affects all versions** - Deprecation message applies to every version
4. **Package name collision** - Cannot publish under both names simultaneously

### Process Constraints

1. **npm authentication required** - User must have valid npm credentials
2. **Publish rights needed** - User must own or have access to `maproom-mcp` package
3. **One-time operation** - Cannot easily undo publish (though can unpublish within 72 hours)

### User Experience Constraints

1. **Must be helpful, not punitive** - Users need clear path forward
2. **Should include `--help` flag** - User specifically requested this
3. **Shouldn't break existing installations** - Deprecation, not removal
4. **Should work with both npm and npx** - Cover all usage patterns

## Success Criteria

### Must Have

1. ✅ Old package shows deprecation warning during `npm install`
2. ✅ npm package page displays deprecation README
3. ✅ Running `npx maproom-mcp` shows migration message
4. ✅ Running `npx maproom-mcp --help` shows migration message
5. ✅ Clear link to new package visible in all contexts

### Nice to Have

1. Migration message includes version comparison
2. Graceful handling of any flags/arguments
3. Analytics/metrics on deprecation message views (npm provides this)

## Risk Assessment

### Low Risk

- **Publishing wrong content** - Can review before publish
- **Breaking existing users** - Deprecation doesn't break anything
- **Lost credentials** - npm provides recovery process

### Medium Risk

- **User doesn't have publish rights** - Would need to contact npm support
- **Package already deprecated by someone else** - Unlikely, but possible
- **npm registry downtime** - Temporary issue, retry later

### Mitigations

1. **Test locally first** - Use `npm pack` to verify package contents
2. **Review README rendering** - Check with `npm pack` → extract → view
3. **Verify credentials** - Run `npm whoami` before attempting publish
4. **Document process** - Create clear instructions for repeating if needed

## Open Questions

1. **Current version of `maproom-mcp`** - Need to check npm registry
2. **Existing users count** - Nice to know impact (npm provides download stats)
3. **Previous deprecation attempts** - Check if already deprecated
4. **Publish rights** - Confirm user has access

These can be answered during implementation.
