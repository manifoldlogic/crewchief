# CLIREL-1001 Implementation Notes

## Ticket: Deprecate old crewchief package with migration warnings

**Status**: Ready for manual publish
**Date**: 2025-11-08

## What Was Done

Created a deprecation package structure at `/tmp/crewchief-deprecation/` with the following files:

### 1. package.json
- Package name: `crewchief`
- Version: `1.0.0` (semantic versioning for breaking change)
- Includes deprecation notice in metadata
- Postinstall script configured

### 2. postinstall.js
Clear deprecation warning that displays on installation:
- Shows package has been renamed
- Provides step-by-step migration instructions
- Formatted for visibility

### 3. index.js
Minimal entry point that shows deprecation error if executed

### 4. README.md
Documentation explaining:
- Package deprecation
- Migration steps
- Why the change was made
- Link to new package

### 5. PUBLISH_INSTRUCTIONS.md
Comprehensive publishing guide including:
- Prerequisites checklist
- Step-by-step publish process
- Deprecation command
- Verification steps
- Troubleshooting

## Manual Steps Required

This ticket requires **manual npm publish** by someone with npm credentials:

### Quick Publish Steps

```bash
# 1. Login to npm
npm login

# 2. Navigate to package
cd /tmp/crewchief-deprecation

# 3. Publish
npm publish

# 4. Deprecate
npm deprecate crewchief "Package renamed to @crewchief/cli. Install @crewchief/cli instead."

# 5. Verify
npm view crewchief
```

Full instructions: `/tmp/crewchief-deprecation/PUBLISH_INSTRUCTIONS.md`

## Why This Couldn't Be Automated

1. **npm authentication**: Requires user's npm credentials
2. **One-time operation**: Not worth CI/CD setup
3. **Low risk**: Simple publish with no production impact
4. **Manual verification**: Better to have human verification for deprecation

## Verification Checklist

After manual publish, verify:
- [ ] `crewchief@1.0.0` appears on npm
- [ ] Package shows deprecation warning: `npm view crewchief`
- [ ] Postinstall script runs on install
- [ ] README visible on npmjs.com/package/crewchief
- [ ] Deprecation message clear and actionable

## Package Location

**Temporary directory**: `/tmp/crewchief-deprecation/`

**Files created**:
- package.json
- postinstall.js
- index.js
- README.md
- PUBLISH_INSTRUCTIONS.md

## Next Steps

1. User with npm credentials publishes the package
2. Verify publication successful
3. Mark ticket as complete
4. Proceed to CLIREL-2001 (Package Configuration)

## Notes

- Package is minimal by design - only contains deprecation warnings
- No actual CLI functionality included
- Safe to publish - only warns users about new package name
- Old package (0.1.23) will remain available but marked deprecated
