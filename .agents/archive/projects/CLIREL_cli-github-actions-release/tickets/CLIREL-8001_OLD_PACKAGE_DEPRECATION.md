# CLIREL-8001: Old Package Deprecation Required

## Status

⚠️ **MANUAL ACTION REQUIRED** - Old `crewchief` package needs deprecation

The new `@crewchief/cli@1.0.0` package has been successfully released, but the old `crewchief` package (currently at v0.1.23) still needs to be deprecated to guide users to the new package.

## What Needs to Be Done

Publish the deprecation package that was prepared in CLIREL-1001 and mark the old package as deprecated on npm.

## Files Ready

The deprecation package is prepared at: `/tmp/crewchief-deprecation/`

Contents:
- `package.json` - Version 1.0.0 with deprecation metadata
- `postinstall.js` - Warning message to users
- `index.js` - Deprecation error on execution
- `README.md` - Migration instructions
- `PUBLISH_INSTRUCTIONS.md` - Step-by-step guide

## Publishing Steps

### Step 1: Login to npm

```bash
npm login
# Enter credentials for daniel.bushman npm account
```

### Step 2: Publish Deprecation Package

```bash
cd /tmp/crewchief-deprecation

# Publish the package
npm publish

# Expected output: crewchief@1.0.0 published
```

### Step 3: Mark Old Versions as Deprecated (Optional)

If you want to deprecate all old versions too:

```bash
# Deprecate all versions of the old package
npm deprecate crewchief "Package renamed to @crewchief/cli. Install @crewchief/cli instead."

# Or deprecate specific versions
npm deprecate crewchief@0.1.23 "Package renamed to @crewchief/cli. Install @crewchief/cli instead."
```

## Expected Result

After publishing, `npm view crewchief` should show:

```
crewchief@1.0.0 | MIT | deps: 0 | versions: 19
DEPRECATED: This package has been renamed to @crewchief/cli

DEPRECATED!! - This package has been renamed to @crewchief/cli
```

Similar to how `maproom-mcp` is deprecated:
https://www.npmjs.com/package/maproom-mcp

## Verification

After publishing, verify:

```bash
# Check package appears with deprecation
npm view crewchief

# Should show:
# - Version 1.0.0
# - DEPRECATED message
# - Description explaining rename

# Test installation warning
npm install crewchief
# Should display deprecation warning from postinstall.js
```

## Why This Matters

1. **User guidance**: When someone tries `npm install crewchief`, they'll see:
   - Deprecation warning from npm
   - Postinstall script message with migration steps
   - Clear path to new package

2. **Search visibility**: When users search for "crewchief" on npmjs.com:
   - Old package shows "DEPRECATED" badge
   - Description explains the rename
   - Users find the new package

3. **Prevents confusion**: Without deprecation:
   - Users might install old package (0.1.23)
   - Miss out on new multi-platform binaries
   - Get confused about which package to use

## Timeline

This should be done **as soon as possible** after the new package release to minimize user confusion.

**Recommended**: Complete within 24 hours of `@crewchief/cli@1.0.0` release.

## Reference

- **CLIREL-1001**: Created the deprecation package
- **New package**: https://www.npmjs.com/package/@crewchief/cli
- **Old package**: https://www.npmjs.com/package/crewchief (needs deprecation)
- **Example**: https://www.npmjs.com/package/maproom-mcp (shows how deprecation should look)

## Current Status

- ✅ New package `@crewchief/cli@1.0.0` published
- ✅ Deprecation package prepared at `/tmp/crewchief-deprecation/`
- ✅ npm publish of deprecation package (completed by user)
- ✅ Old package deprecated on npm registry

---

**Note**: This task requires npm authentication which cannot be automated in the current environment. Manual execution by user with npm credentials is required.
