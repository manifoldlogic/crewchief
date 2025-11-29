# Architecture: npm Package Deprecation

## Solution Overview

Publish a final "tombstone" version of `maproom-mcp` that:
1. Shows clear deprecation message when executed
2. Displays migration README on npm package page
3. Uses `npm deprecate` command for installation warnings
4. Supports `--help` flag as requested

**Design Philosophy:** Keep it simple. This is a one-time operation with no ongoing maintenance. Optimize for clarity and helpfulness, not sophistication.

## Component Design

### 1. Deprecation Package Structure

```
/tmp/maproom-mcp-deprecated/
├── package.json          # Version 2.0.0, minimal metadata
├── README.md             # Full deprecation notice (user-visible on npm)
├── index.js              # Executable that shows migration message
└── .npmignore            # (optional) exclude development files
```

**Why `/tmp`?**
- One-time operation, no need for version control
- Keeps main repository clean
- Easy cleanup after publish

### 2. package.json Design

**Key Decisions:**

```json
{
  "name": "maproom-mcp",
  "version": "2.0.0",
  "description": "DEPRECATED: Use @crewchief/maproom-mcp instead",
  "main": "index.js",
  "bin": {
    "maproom-mcp": "./index.js"
  },
  "deprecated": "This package has been renamed to @crewchief/maproom-mcp",
  "keywords": ["deprecated", "maproom", "mcp"],
  "author": "CrewChief",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/danielbushman/crewchief.git"
  }
}
```

**Design Rationale:**
- **Version 2.0.0**: Major bump signals breaking change (deprecation)
- **`deprecated` field**: npm shows this automatically
- **`bin` entry**: Allows `npx maproom-mcp` to work
- **Repository link**: Points to new location
- **Minimal dependencies**: No dependencies = no vulnerabilities

### 3. Executable (index.js) Design

**Purpose:** Show helpful message when user tries to run the package.

**Requirements:**
- Show deprecation notice
- Link to new package
- Support `--help` flag (user requirement)
- Handle any flags gracefully
- Exit with error code (signal it's deprecated)

**Implementation:**

```javascript
#!/usr/bin/env node

const args = process.argv.slice(2);

console.error('\n⚠️  DEPRECATED: maproom-mcp is no longer maintained\n');
console.error('This package has been replaced by @crewchief/maproom-mcp\n');

// Handle --help flag specifically
if (args.includes('--help') || args.includes('-h')) {
  console.error('For help with the current package, run:');
  console.error('  npx @crewchief/maproom-mcp --help\n');
} else {
  console.error('To use the new package:');
  console.error('  npm install @crewchief/maproom-mcp\n');
  console.error('Or with npx:');
  console.error('  npx @crewchief/maproom-mcp setup --provider=openai\n');
}

console.error('Documentation: https://www.npmjs.com/package/@crewchief/maproom-mcp\n');
process.exit(1);
```

**Design Decisions:**
- **Exit code 1**: Signals error/deprecation
- **stderr output**: Conventional for error/warning messages
- **`--help` detection**: User-requested feature
- **No actual functionality**: Don't partially work, fail clearly
- **Minimal code**: ~15 lines, no dependencies, can't break

### 4. README Design

**Purpose:** Primary deprecation notice users see on npm package page.

**Structure:**
```markdown
# ⚠️ DEPRECATED: maproom-mcp

**This package is no longer maintained.**

## Use @crewchief/maproom-mcp Instead

[Clear migration instructions]
[Links to new package]
[Simple before/after examples]
```

**Already Created:** `/workspace/packages/maproom-mcp/README.deprecated.md`

**Design Decisions:**
- **Big warning emoji**: Immediate visual signal
- **"No longer maintained"**: Clear and honest
- **Positive framing**: "Use this instead" not "Don't use this"
- **Actionable instructions**: Exact commands to run
- **Minimal explanation**: Why it changed (name only)

## Publishing Strategy

### Two-Phase Approach

**Phase 1: Publish New Version**
- Creates v2.0.0 with deprecation content
- Updates README on npm package page
- Makes executable show migration message

**Phase 2: Deprecate All Versions**
- Runs `npm deprecate maproom-mcp "message"`
- Adds warning to installation output
- Affects all versions (past and future)

**Why Two Phases?**
- Each serves different user touchpoint
- Publishing updates package page (browsing users)
- Deprecation updates installation (installing users)
- Both needed for complete coverage

### Deprecation Message Design

User requested:
```bash
npm deprecate maproom-mcp "This package has been replaced by @crewchief/maproom-mcp. Please use the new package: npx @crewchief/maproom-mcp --help"
```

**Analysis:**
- ✅ Clear replacement mentioned
- ✅ Actionable command
- ✅ Includes `--help` as requested
- ✅ Short enough for npm warning display

## Workflow Design

### Automated Script Approach

**Option 1: Manual Steps** (Current recommendation)
```bash
# 1. Create deprecation package
mkdir -p /tmp/maproom-mcp-deprecated
cd /tmp/maproom-mcp-deprecated

# 2. Create files
[copy package.json, README, index.js]

# 3. Test locally
npm pack
tar -xzf maproom-mcp-2.0.0.tgz
cd package
node index.js
node index.js --help

# 4. Publish
npm publish

# 5. Deprecate
npm deprecate maproom-mcp "message"
```

**Option 2: Automated Script** (Future enhancement)
- Could create shell script in `.crewchief/scripts/`
- Automates file creation and publishing
- Adds validation steps
- **Not needed for one-time operation**

**Decision:** Manual steps for MVP. Simple enough to do by hand, and user learns the process.

## Data Flow

```
User Action          →  npm Registry Response
─────────────────────────────────────────────
npm install maproom-mcp
                     →  ⚠️  deprecated This package has been replaced...
                        Installing anyway...

npm view maproom-mcp
                     →  Shows v2.0.0 with deprecation in description
                        README displays deprecation notice

npx maproom-mcp
                     →  Downloads v2.0.0
                        Runs index.js
                        Shows migration message
                        Exits with code 1

npx maproom-mcp --help
                     →  Same as above, but shows --help specific message
```

## Error Handling

### Potential Issues and Solutions

**1. User Lacks Publish Rights**
```
Error: 403 Forbidden - You do not have permission to publish "maproom-mcp"
```
**Solution:** Verify with `npm whoami` and `npm owner ls maproom-mcp`

**2. Version Already Exists**
```
Error: 409 Conflict - Version 2.0.0 already exists
```
**Solution:** Bump to 2.0.1 or 3.0.0

**3. Network Issues**
```
Error: ETIMEDOUT
```
**Solution:** Retry, check npm status page

**4. Invalid Package**
```
Error: Invalid package.json
```
**Solution:** Validate with `npm pack` first

### Validation Steps

**Pre-publish Checklist:**
1. ✅ `npm pack` succeeds
2. ✅ Package size reasonable (<10 KB)
3. ✅ README renders correctly (extract .tgz and view)
4. ✅ `node index.js` shows correct message
5. ✅ `node index.js --help` shows help-specific message
6. ✅ `npm whoami` shows correct user

## Alternative Approaches Considered

### Alternative 1: Just Use `npm deprecate`
**Pros:** Simplest, one command
**Cons:** Doesn't update README or executable
**Verdict:** Insufficient - browsing users won't see deprecation

### Alternative 2: Unpublish Package
**Pros:** Completely removes it
**Cons:** Breaks existing installations, npm discourages this
**Verdict:** Too aggressive, violates npm best practices

### Alternative 3: Redirect Package
**Pros:** Could automatically install new package
**Cons:** Confusing, unconventional, might break automation
**Verdict:** Too clever, prefer explicit deprecation

### Alternative 4: Proxy Package
**Pros:** Could forward to new package transparently
**Cons:** Maintains old package indefinitely, confusing
**Verdict:** Doesn't solve the problem (want to deprecate)

## Selected Approach: Tombstone + Deprecate

**Why This Works:**
- ✅ Industry standard (request, babel-core, istanbul)
- ✅ Clear and explicit
- ✅ Doesn't break existing users
- ✅ Covers all user touchpoints
- ✅ Simple to implement
- ✅ One-time operation, no maintenance

## File Specifications

### package.json (Exact Content)

```json
{
  "name": "maproom-mcp",
  "version": "2.0.0",
  "type": "commonjs",
  "description": "DEPRECATED: Use @crewchief/maproom-mcp instead",
  "main": "index.js",
  "bin": {
    "maproom-mcp": "./index.js"
  },
  "files": [
    "index.js",
    "README.md"
  ],
  "keywords": [
    "deprecated",
    "maproom",
    "mcp",
    "semantic-search"
  ],
  "author": "CrewChief",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/danielbushman/crewchief.git"
  },
  "bugs": {
    "url": "https://github.com/danielbushman/crewchief/issues"
  },
  "homepage": "https://github.com/danielbushman/crewchief/tree/main/packages/maproom-mcp#readme",
  "deprecated": "This package has been renamed to @crewchief/maproom-mcp"
}
```

### index.js (Exact Content)

```javascript
#!/usr/bin/env node

const args = process.argv.slice(2);

console.error('\n⚠️  DEPRECATED: maproom-mcp is no longer maintained\n');
console.error('This package has been replaced by @crewchief/maproom-mcp\n');

if (args.includes('--help') || args.includes('-h')) {
  console.error('For help with the new package, run:');
  console.error('  npx @crewchief/maproom-mcp --help\n');
} else {
  console.error('To use the new package:');
  console.error('  npm install @crewchief/maproom-mcp\n');
  console.error('Or with npx:');
  console.error('  npx @crewchief/maproom-mcp setup --provider=openai\n');
}

console.error('More info: https://www.npmjs.com/package/@crewchief/maproom-mcp\n');
process.exit(1);
```

### README.md

Use existing `/workspace/packages/maproom-mcp/README.deprecated.md`

## Success Metrics

**How We Know It Worked:**

1. ✅ `npm view maproom-mcp` shows version 2.0.0
2. ✅ npm package page shows deprecation README
3. ✅ `npm install maproom-mcp` shows deprecation warning
4. ✅ `npx maproom-mcp` shows migration message
5. ✅ `npx maproom-mcp --help` shows help-specific message
6. ✅ All messages link to new package

**Verification Commands:**
```bash
# Check version
npm view maproom-mcp version

# Check deprecation message
npm view maproom-mcp deprecated

# Test installation warning
npm install maproom-mcp 2>&1 | grep -i deprecat

# Test execution
npx maproom-mcp@2.0.0
npx maproom-mcp@2.0.0 --help
```

## Maintenance Plan

**Short Answer:** None needed.

This is a terminal state. Once published:
- No updates required
- No monitoring needed
- No user support necessary (they should use new package)

**Only Action Needed:** If user contacts about old package, redirect to new one.
