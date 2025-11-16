# Ticket: VSMAP-4004: Package extension as VSIX for distribution

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Configure package.json for publishing, bundle all platform binaries, and create VSIX package. Verify VSIX installs and activates correctly.

## Background
This completes Phase 4 (Polish & Testing) of the VSMAP plan. After all features are implemented, tested, and documented, we need to package the extension for distribution. The VSIX must include all platform-specific binaries, proper metadata, and be installable on fresh VSCode instances.

Reference: VSMAP_PLAN.md Phase 4 "Polish & Testing - VSIX Packaging"

## Acceptance Criteria
- [ ] package.json complete with all required metadata (name, publisher, version, activation events)
- [ ] All Rust binaries bundled in `bin/` directory for 5 platforms
- [ ] VSIX created successfully via `vsce package` or `@vscode/vsce`
- [ ] VSIX file size <50MB (optimized bundle)
- [ ] VSIX installs without errors using `code --install-extension`
- [ ] Extension activates correctly after VSIX install
- [ ] Binary permissions correct (executable) after install
- [ ] No development dependencies included in bundle

## Technical Requirements
- Use `@vscode/vsce` for packaging (modern tooling)
- Include binaries for all platforms:
  - `bin/darwin-x64/crewchief-maproom`
  - `bin/darwin-arm64/crewchief-maproom`
  - `bin/linux-x64/crewchief-maproom`
  - `bin/linux-arm64/crewchief-maproom`
  - `bin/win32-x64/crewchief-maproom.exe`
- Set binary executable permissions in package script
- Create `.vscodeignore` to exclude development files
- Test install on clean VSCode instance (no other extensions)
- Verify all activation events work post-install

## Implementation Notes
package.json configuration:

```json
{
  "name": "vscode-maproom",
  "displayName": "Maproom Semantic Search",
  "description": "Semantic code search powered by embeddings",
  "version": "0.1.0",
  "publisher": "maproom",
  "engines": {
    "vscode": "^1.85.0"
  },
  "categories": ["Other"],
  "activationEvents": [
    "onStartupFinished"
  ],
  "main": "./dist/extension.js",
  "contributes": {
    "commands": [
      {
        "command": "maproom.setup",
        "title": "Maproom: Setup"
      },
      {
        "command": "maproom.search",
        "title": "Maproom: Search"
      },
      {
        "command": "maproom.restartWatchers",
        "title": "Maproom: Restart Watchers"
      }
    ]
  },
  "scripts": {
    "vsce:package": "vsce package",
    "prepare:binaries": "node scripts/prepare-binaries.js"
  },
  "devDependencies": {
    "@vscode/vsce": "^2.22.0"
  }
}
```

.vscodeignore configuration:
```
.vscode/**
.vscode-test/**
src/**
.gitignore
.eslintrc.json
tsconfig.json
node_modules/**/.bin/**
**/*.map
**/*.ts
!dist/**/*.js
.github/**
test/**
*.vsix
.agents/**
docs/**
```

prepare-binaries.js script:
```javascript
const fs = require('fs');
const path = require('path');

// Copy binaries from CrewChief build
const platforms = [
  'darwin-x64',
  'darwin-arm64',
  'linux-x64',
  'linux-arm64',
  'win32-x64'
];

for (const platform of platforms) {
  const sourceDir = path.join(__dirname, '../../packages/cli/bin', platform);
  const targetDir = path.join(__dirname, '../bin', platform);

  fs.mkdirSync(targetDir, { recursive: true });

  const binaryName = platform.startsWith('win') ?
    'crewchief-maproom.exe' : 'crewchief-maproom';

  const source = path.join(sourceDir, binaryName);
  const target = path.join(targetDir, binaryName);

  fs.copyFileSync(source, target);

  // Set executable on Unix platforms
  if (!platform.startsWith('win')) {
    fs.chmodSync(target, 0o755);
  }
}

console.log('✅ Binaries prepared for packaging');
```

Package and verify workflow:
```bash
# 1. Prepare binaries
npm run prepare:binaries

# 2. Build extension
npm run compile

# 3. Package VSIX
npm run vsce:package

# 4. Verify file size
ls -lh *.vsix

# 5. Test install
code --install-extension vscode-maproom-0.1.0.vsix

# 6. Launch new VSCode window
code --new-window

# 7. Check extension activated
# View → Output → Maproom
```

Binary detection logic (in extension):
```typescript
function getBinaryPath(): string {
  const platform = process.platform;
  const arch = process.arch;

  const platformKey = `${platform}-${arch}`;
  const binaryName = platform === 'win32' ?
    'crewchief-maproom.exe' : 'crewchief-maproom';

  return path.join(
    __dirname,
    '..',
    'bin',
    platformKey,
    binaryName
  );
}
```

Verification checklist:
- [ ] VSIX installs without error
- [ ] Extension appears in Extensions list
- [ ] Extension activates (check Output)
- [ ] Docker services start
- [ ] Setup wizard appears
- [ ] Binary executes successfully
- [ ] No "module not found" errors
- [ ] File size reasonable (<50MB)

VSIX metadata verification:
```bash
# Extract and inspect
unzip -l vscode-maproom-0.1.0.vsix

# Should contain:
# - extension.js (compiled)
# - bin/darwin-x64/crewchief-maproom
# - bin/darwin-arm64/crewchief-maproom
# - bin/linux-x64/crewchief-maproom
# - bin/linux-arm64/crewchief-maproom
# - bin/win32-x64/crewchief-maproom.exe
# - package.json
# - README.md
```

## Dependencies
- VSMAP-4002 (manual testing) should be complete to ensure quality
- All platform binaries built in CrewChief main repo
- @vscode/vsce installed

## Risk Assessment
- **Risk**: Binaries may lose executable permissions in VSIX
  - **Mitigation**: Test on Unix systems, include chmod in post-install if needed
- **Risk**: Bundle may be too large (>50MB)
  - **Mitigation**: Exclude source maps, dev dependencies, compress binaries
- **Risk**: Missing platform binary causes runtime error
  - **Mitigation**: Show friendly error message suggesting platform unsupported

## Files/Packages Affected
- `package.json` (update with publishing metadata)
- `.vscodeignore` (new file, exclude dev files)
- `scripts/prepare-binaries.js` (new file, copy binaries)
- `bin/` directory (new, contains all platform binaries)
- `README.md` (update with install instructions)
- Root directory (VSIX file created: `vscode-maproom-0.1.0.vsix`)
