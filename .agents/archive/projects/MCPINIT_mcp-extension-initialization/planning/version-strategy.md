# Version Strategy: Extension and MCP Alignment

## Problem

The VSCode extension and MCP server must use compatible versions to avoid schema mismatches. Currently:
- `@crewchief/maproom-mcp`: v2.2.1
- `vscode-maproom`: v0.1.1

When the extension invokes `npx @crewchief/maproom-mcp setup`, we need to ensure:
1. The MCP version matches what the extension expects
2. Both use the same underlying Maproom binary/schema
3. Updates are synchronized across both packages

## Version Alignment Strategy

### Option 1: Pinned Version (RECOMMENDED)

**Approach**: Extension embeds exact MCP version it's compatible with

**Implementation**:
```typescript
// src/constants.ts
export const MAPROOM_MCP_VERSION = '2.2.1' // Synchronized with maproom-mcp package.json

// src/process/setup-manager.ts
const args = [
  `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`,
  'setup',
  `--provider=${options.provider}`
]
```

**MCP Configuration**:
```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": [
        "-y",
        "@crewchief/maproom-mcp@2.2.1"
      ],
      "env": { ... }
    }
  }
}
```

**Pros**:
- ✅ Guaranteed compatibility
- ✅ Predictable behavior
- ✅ No surprises from upstream changes
- ✅ Users can test new versions explicitly

**Cons**:
- ⚠️ Requires extension update to get MCP updates
- ⚠️ Slightly slower first run (npx downloads specific version)

**When to Update**:
- When maproom-mcp has breaking changes
- When maproom-mcp has significant features extension should expose
- When maproom-mcp has critical bug fixes

---

### Option 2: Latest Version

**Approach**: Always use `@latest` tag

**Implementation**:
```typescript
const args = [
  '@crewchief/maproom-mcp@latest',
  'setup',
  `--provider=${options.provider}`
]
```

**Pros**:
- ✅ Users always get latest features
- ✅ Bug fixes automatically available
- ✅ Less coordination needed

**Cons**:
- ❌ Breaking changes can break extension
- ❌ Unpredictable behavior
- ❌ Hard to debug version-specific issues

**Not Recommended**: Too risky for production use

---

### Option 3: Semver Range

**Approach**: Use semver range for minor updates

**Implementation**:
```typescript
const MAPROOM_MCP_VERSION = '^2.2.0' // Minor updates OK, major needs explicit update
```

**Pros**:
- ✅ Get bug fixes automatically
- ✅ Compatible minor features
- ⚠️ Major versions protected

**Cons**:
- ⚠️ Still some unpredictability
- ⚠️ CI tests might pass but user installs fail

**Not Recommended**: Adds complexity without clear benefit

## Recommended Approach: Pinned Version

Use **Option 1** with explicit version pinning for predictable, testable behavior.

### Implementation Details

#### 1. Version Constant

**File**: `src/constants.ts`
```typescript
/**
 * MCP server version compatible with this extension
 *
 * IMPORTANT: This must match the version tested during extension release.
 * Update when:
 * - maproom-mcp has breaking schema changes
 * - maproom-mcp has features this extension exposes
 * - maproom-mcp has critical security fixes
 */
export const MAPROOM_MCP_VERSION = '2.2.1'
```

#### 2. Setup Manager

**File**: `src/process/setup-manager.ts`
```typescript
import { MAPROOM_MCP_VERSION } from '../constants'

export class SetupManager {
  async runSetup(options: SetupOptions): Promise<SetupResult> {
    const args = [
      '-y', // Auto-accept npx prompt
      `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`,
      'setup',
      `--provider=${options.provider}`
    ]

    outputChannel.appendLine(`[Setup] Using @crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`)

    // ... rest of implementation
  }
}
```

#### 3. MCP Config Writer

**File**: `src/config/mcp-writer.ts`
```typescript
import { MAPROOM_MCP_VERSION } from '../constants'

export class MCPConfigWriter {
  buildMaproomConfig(provider: ProviderConfig): MCPServerConfig {
    return {
      command: 'npx',
      args: [
        '-y',
        `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`
      ],
      env: this.buildEnvironment(provider)
    }
  }
}
```

## Release Coordination

### Synchronized Versioning

Both packages should follow semantic versioning with coordinated releases:

| Change Type | maproom-mcp | vscode-maproom | Example |
|-------------|-------------|----------------|---------|
| **Breaking Schema** | MAJOR bump | MAJOR bump | 2.2.1 → 3.0.0 (both) |
| **New MCP Tool** | MINOR bump | MINOR bump (if exposed) | 2.2.1 → 2.3.0 (both) |
| **Bug Fix** | PATCH bump | No change* | 2.2.1 → 2.2.2 (MCP only) |
| **Extension Feature** | No change | MINOR/PATCH | 0.1.1 → 0.2.0 (ext only) |

\* Extension can be updated to reference new patch version, but not required

### Release Workflow

#### When maproom-mcp Updates

**Patch Release** (2.2.1 → 2.2.2):
```bash
# maproom-mcp
pnpm version patch
git tag @crewchief/maproom-mcp@2.2.2
git push && git push --tags

# vscode-maproom (optional, for bug fixes)
# Update src/constants.ts: MAPROOM_MCP_VERSION = '2.2.2'
# Run tests to verify compatibility
# If tests pass, release new extension version
```

**Minor Release** (2.2.1 → 2.3.0):
```bash
# maproom-mcp
pnpm version minor
git tag @crewchief/maproom-mcp@2.3.0
git push && git push --tags

# vscode-maproom (required if exposing new features)
# Update src/constants.ts: MAPROOM_MCP_VERSION = '2.3.0'
# Add UI for new features (if any)
# Update README with new capabilities
# Run full test suite
# Release: 0.1.1 → 0.2.0
```

**Major Release** (2.2.1 → 3.0.0):
```bash
# maproom-mcp
pnpm version major
git tag @crewchief/maproom-mcp@3.0.0
git push && git push --tags

# vscode-maproom (required, breaking changes)
# Update src/constants.ts: MAPROOM_MCP_VERSION = '3.0.0'
# Fix breaking changes in extension code
# Update all references to MCP schema
# Run comprehensive test suite
# Document migration steps
# Release: 0.1.1 → 1.0.0 (major extension change)
```

### Automated Version Sync Script

**File**: `scripts/sync-mcp-version.js`
```javascript
#!/usr/bin/env node
/**
 * Sync MCP version between packages
 * Usage: node scripts/sync-mcp-version.js
 */
const fs = require('fs')
const path = require('path')

const mcpPackageJson = require('../packages/maproom-mcp/package.json')
const constantsPath = path.join(__dirname, '../packages/vscode-maproom/src/constants.ts')

const mcpVersion = mcpPackageJson.version

// Read current constants file
let constantsContent = fs.readFileSync(constantsPath, 'utf-8')

// Replace version
const versionPattern = /export const MAPROOM_MCP_VERSION = '[\d.]+'/
constantsContent = constantsContent.replace(
  versionPattern,
  `export const MAPROOM_MCP_VERSION = '${mcpVersion}'`
)

// Write back
fs.writeFileSync(constantsPath, constantsContent)

console.log(`✓ Updated vscode-maproom to use @crewchief/maproom-mcp@${mcpVersion}`)
```

**Add to package.json**:
```json
{
  "scripts": {
    "sync:versions": "node scripts/sync-mcp-version.js"
  }
}
```

**Usage**:
```bash
# After updating maproom-mcp version
cd packages/maproom-mcp
pnpm version minor

# Sync to extension
cd ../..
pnpm sync:versions

# Verify
cd packages/vscode-maproom
grep MAPROOM_MCP_VERSION src/constants.ts
```

## Version Checking

### Runtime Version Detection (Optional Enhancement)

**File**: `src/process/version-checker.ts`
```typescript
/**
 * Check installed MCP version matches expected version
 */
export async function checkMCPVersion(): Promise<VersionCheckResult> {
  try {
    const { stdout } = await execAsync('npx @crewchief/maproom-mcp --version')
    const installedVersion = stdout.trim()

    if (installedVersion !== MAPROOM_MCP_VERSION) {
      return {
        compatible: false,
        expected: MAPROOM_MCP_VERSION,
        actual: installedVersion,
        message: `MCP version mismatch. Expected ${MAPROOM_MCP_VERSION}, found ${installedVersion}`
      }
    }

    return { compatible: true }
  } catch (error) {
    return { compatible: false, message: 'Could not determine MCP version' }
  }
}
```

**Use Case**: Show warning in status bar if user manually installed different version

## Testing Strategy

### Version Compatibility Tests

**File**: `test/integration/version-compatibility.test.ts`
```typescript
import { MAPROOM_MCP_VERSION } from '../../src/constants'

describe('Version Compatibility', () => {
  it('should match maproom-mcp package.json version', () => {
    const mcpPackage = require('../../../maproom-mcp/package.json')

    expect(MAPROOM_MCP_VERSION).toBe(mcpPackage.version)
  })

  it('should use versioned package in setup command', () => {
    const manager = new SetupManager(mockOutputChannel)
    const args = manager.buildSetupArgs({ provider: 'ollama' })

    expect(args).toContain(`@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`)
  })

  it('should use versioned package in MCP config', () => {
    const writer = new MCPConfigWriter()
    const config = writer.buildMaproomConfig({ provider: 'openai' })

    expect(config.args).toContain(`@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`)
  })
})
```

### CI Version Check

**File**: `.github/workflows/version-check.yml`
```yaml
name: Version Sync Check

on:
  pull_request:
    paths:
      - 'packages/maproom-mcp/package.json'
      - 'packages/vscode-maproom/src/constants.ts'

jobs:
  check-version-sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check versions match
        run: |
          MCP_VERSION=$(jq -r .version packages/maproom-mcp/package.json)
          EXT_VERSION=$(grep MAPROOM_MCP_VERSION packages/vscode-maproom/src/constants.ts | cut -d "'" -f2)

          if [ "$MCP_VERSION" != "$EXT_VERSION" ]; then
            echo "❌ Version mismatch!"
            echo "   maproom-mcp: $MCP_VERSION"
            echo "   vscode-maproom: $EXT_VERSION"
            echo ""
            echo "Run: pnpm sync:versions"
            exit 1
          fi

          echo "✓ Versions synchronized: $MCP_VERSION"
```

## Migration Path

### For Users with Existing Installations

**Problem**: User has `@crewchief/maproom-mcp@2.1.0` globally installed, extension expects 2.2.1

**Solution**: `npx` with version pin will download correct version:
```bash
# npx -y downloads specific version to cache
npx -y @crewchief/maproom-mcp@2.2.1 setup --provider=ollama
```

**MCP Config**: Similarly uses versioned package:
```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp@2.2.1"],
      "env": { ... }
    }
  }
}
```

**Outcome**: Users get correct version automatically, no manual intervention needed.

## Documentation Requirements

### README Update

**Section to Add**:
```markdown
## Version Compatibility

This extension is compatible with `@crewchief/maproom-mcp@2.2.1`.

The setup wizard automatically installs the correct version. If you previously
installed a different version globally, the extension will use its own pinned
version to ensure compatibility.

### Manual Installation

If you need to use the MCP server outside the extension:

```bash
npx @crewchief/maproom-mcp@2.2.1 setup --provider=ollama
```

### Updating

Both the extension and MCP server receive updates together:
- Extension updates: Via VS Code Marketplace
- MCP server: Automatically used via version pin in extension

No manual coordination needed!
```

### CHANGELOG

**For Each Release**:
```markdown
## [0.2.0] - 2025-01-XX

### Changed
- Updated to `@crewchief/maproom-mcp@2.2.1`
  - Improved setup reliability
  - Better error messages
  - Security fixes
```

## Conclusion

**Recommended Strategy**: **Pinned Version**

**Implementation**:
1. Create `src/constants.ts` with `MAPROOM_MCP_VERSION = '2.2.1'`
2. Use `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}` in all npx calls
3. Add version sync script: `pnpm sync:versions`
4. Add CI check to ensure versions match
5. Document version compatibility in README

**Benefits**:
- ✅ Guaranteed schema compatibility
- ✅ Predictable behavior for users
- ✅ Easy to test and verify
- ✅ Clear upgrade path
- ✅ No surprises from upstream changes

**Maintenance**:
- Update `MAPROOM_MCP_VERSION` when maproom-mcp releases
- Run test suite to verify compatibility
- Release extension with updated version constant
- Document changes in CHANGELOG

This approach balances stability (pinned versions) with maintainability (easy sync script) while keeping users' installations working predictably.
