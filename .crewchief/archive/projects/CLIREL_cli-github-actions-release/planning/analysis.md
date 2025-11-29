# Analysis: CLI GitHub Actions Release Automation

## Problem Space

### Current State

The CrewChief CLI package (`crewchief` on npm) uses a manual release process:

1. Developer runs `pnpm release:minor` locally
2. Script bumps version in package.json
3. Script commits, tags as `crewchief@v{version}`, and pushes
4. Script runs `pnpm publish --access public`
5. The `prepublishOnly` hook runs `pnpm build:all`:
   - Builds TypeScript with tsup
   - Runs `scripts/build-and-package.sh` to build Rust maproom binary
6. Package published to npm with binaries **only for the developer's platform**

**Critical limitation**: The CLI package ships with Rust binaries for only one platform (whoever ran the release). Users on other platforms get a broken CLI.

**Additional issues**:
- Package name `crewchief` doesn't follow org convention (`@crewchief/*`)
- No separation between CLI and MCP releases (both could accidentally use same simple `v*.*.*` tags)
- Manual process is error-prone
- No cross-platform binary validation before publish
- No dry-run or testing capability
- **Race condition in maproom-mcp**: Using `git push --follow-tags` can cause workflow failures when tag arrives before commit is fully registered on GitHub

### Industry Solutions

**Standard monorepo release patterns**:

1. **Lerna/Changesets**: Version management tools for monorepos
   - Handle independent package versioning
   - Generate changelogs
   - Coordinate releases
   - **Limitation**: Don't handle native binary compilation

2. **GitHub Actions matrix builds**: Multi-platform CI/CD
   - Build on multiple OS/architecture combinations
   - Collect artifacts from different runners
   - Package together before publish
   - **Example**: Rust projects commonly use this pattern

3. **Package-scoped tags**: Monorepo convention
   - Format: `@scope/package@v1.0.0`
   - Clearly identifies which package a tag belongs to
   - Prevents tag conflicts between packages
   - **Example**: React monorepo uses `react@18.0.0`, `react-dom@18.0.0`

4. **Cross-compilation**: Single-platform builds for multiple targets
   - Use cross-compilation toolchains
   - Faster than matrix builds (one runner)
   - **Limitation**: Complex setup, platform-specific issues common

**Existing implementation in this repo**: The `@crewchief/maproom-mcp` package already has this solved:
- GitHub Actions workflow: `.github/workflows/build-and-publish-maproom-mcp.yml`
- Triggers on `v*.*.*` tags
- Matrix builds for 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- Uses cross-compilation for linux targets on ubuntu runner
- Uses native builds for darwin targets on macOS runners
- Collects all binaries, validates them, packages together
- Publishes to npm as `@crewchief/maproom-mcp`

### Research Findings

**GitHub Actions limitations**:
- No official linux-arm64 hosted runners (uses cross-compilation instead)
- macOS runners cost 10x linux runners (use sparingly)
- Artifact upload/download between jobs adds ~30s overhead
- Matrix strategy can run jobs in parallel (4 platforms build simultaneously)

**npm scoped packages**:
- Require `@orgname/package` format
- Need org membership to publish
- Can have public or private access
- Breaking change from unscoped to scoped name

**Git tag patterns**:
- Simple tags: `v1.0.0` (good for single-package repos)
- Package-scoped: `@scope/pkg@v1.0.0` (monorepo best practice)
- Prefixed: `cli-v1.0.0` (less conventional, but simpler)
- GitHub tag triggers support glob patterns: `@crewchief/cli@v*.*.*`

**Rust binary cross-compilation**:
- `cross` tool simplifies cross-compilation
- MCP workflow uses manual cross-compilation setup
- Target triples: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`
- Binary stripping reduces size 30-40%

**Deprecation best practices**:
- Publish final version with deprecation warnings
- Use `deprecated` field in package.json
- Redirect users to new package in postinstall script
- Mark as deprecated on npm registry (`npm deprecate <pkg>@<version> <message>`)

## Current Project State

**Repository structure**:
```
packages/
├── cli/                    # CLI package (needs migration)
│   ├── bin/
│   │   ├── crewchief       # Shell wrapper
│   │   ├── darwin-arm64/   # Platform binaries (incomplete)
│   │   └── linux-arm64/
│   ├── src/                # TypeScript source
│   ├── dist/               # Built JS (from tsup)
│   ├── scripts/
│   │   └── release.mjs     # Manual release script
│   └── package.json        # Name: "crewchief", v0.1.23
│
└── maproom-mcp/            # MCP package (already automated)
    ├── bin/                # Platform binaries (complete)
    │   ├── darwin-arm64/
    │   ├── darwin-x64/
    │   ├── linux-arm64/
    │   ├── linux-x64/
    │   └── cli.cjs
    ├── src/
    ├── dist/
    ├── scripts/
    │   └── release.js      # Creates tag, pushes (GH Actions publishes)
    └── package.json        # Name: "@crewchief/maproom-mcp", v1.3.5

crates/maproom/             # Rust indexer (shared)
scripts/build-and-package.sh # Local build script (being replaced)
.github/workflows/
└── build-and-publish-maproom-mcp.yml  # Template to copy
```

**Key files**:
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - 354 lines, comprehensive workflow
- `/workspace/packages/cli/scripts/release.mjs` - 65 lines, manual release
- `/workspace/packages/cli/package.json` - Current config
- `/workspace/scripts/build-and-package.sh` - 159 lines, local-only build

**Current CLI bin/ state**:
- `darwin-arm64/crewchief-maproom` - 9.7MB, from Aug 26 (stale)
- `linux-arm64/crewchief-maproom` - 16MB, from Nov 6 (recent)
- Missing: darwin-x64, linux-x64
- Symlink points to linux-arm64 (dev platform)

**Current versioning**:
- CLI: 0.1.23 (unscoped package)
- MCP: 1.3.5 (scoped package)
- Independent versions (no sync requirement)

**User decision input**:
- Rename: `crewchief` → `@crewchief/cli`
- Tagging: Package-scoped tags (`@crewchief/cli@v*.*.*`)
- Versioning: Independent (CLI and MCP can have different versions)
- Deprecation: Publish final `crewchief@1.0.0` with warnings, mark as deprecated
- Breaking change: Use v1.0.0 to signal package rename

## Problem Analysis

**Core challenge**: Replicating the existing maproom-mcp automation pattern for the CLI package while maintaining independent releases.

**Key differences between CLI and MCP**:
1. **Entry point**: CLI has shell wrapper (`bin/crewchief`), MCP has Node script (`bin/cli.cjs`)
2. **Build process**: CLI builds TypeScript + Rust, MCP builds only TypeScript (uses same Rust binary)
3. **Distribution**: Both need all 4 platform binaries
4. **Usage**: CLI is a command-line tool, MCP is an MCP server

**Technical constraints**:
- GitHub Actions doesn't have native linux-arm64 runners (must cross-compile)
- macOS builds are expensive (10x cost of linux)
- Rust build times are significant (5-10 minutes per platform)
- Binaries are large (10-16MB each, ~60MB total)
- npm has 10GB package size limit (well within bounds)

**Migration risks**:
1. **Breaking change**: Package rename breaks existing users
   - **Mitigation**: Deprecation path, clear communication
   - **Low risk**: Likely no production users yet

2. **Tag conflict**: Both packages use `v*.*.*` pattern currently
   - **Mitigation**: Migrate both to package-scoped tags
   - **Medium risk**: Requires coordinated update to MCP workflow

3. **Binary validation**: Incorrect binaries could ship
   - **Mitigation**: Copy MCP's validation logic (size checks, execution tests)
   - **Low risk**: Well-tested pattern exists

4. **Workflow maintenance**: Two similar workflows could drift
   - **Mitigation**: Extract common steps to reusable workflow (future enhancement)
   - **Low risk**: Accept duplication for now

**Success criteria**:
1. CLI releases publish with binaries for all 4 platforms
2. Independent tagging: `@crewchief/cli@v*` triggers CLI workflow only
3. Independent versioning: CLI and MCP can release separately
4. Old package deprecated with clear migration path
5. Automated validation prevents broken releases
6. Process matches MCP workflow quality

## Domain Concepts

1. **Package**: npm package with name, version, dependencies
2. **Scoped package**: npm package with `@org/name` format
3. **Release**: Publishing a new version to npm registry
4. **Tag**: Git tag marking a specific commit for release
5. **Package-scoped tag**: Tag format `@org/pkg@v1.0.0`
6. **Workflow**: GitHub Actions YAML file defining CI/CD pipeline
7. **Matrix build**: Parallel jobs for multiple platforms
8. **Platform**: OS + architecture combination (e.g., linux-x64)
9. **Binary**: Compiled Rust executable for specific platform
10. **Artifact**: File uploaded/downloaded between GitHub Actions jobs
11. **Cross-compilation**: Building binary for different platform than build host
12. **Validation**: Automated checks before publish (size, execution, content)
13. **Deprecation**: Marking old package version as obsolete
14. **Breaking change**: Incompatible change requiring major version bump
15. **Monorepo**: Repository with multiple packages

**Total: 15 concepts** - within coherence bounds ✓

## Architectural Touchpoints

**Modified files**:
- `.github/workflows/build-and-publish-cli.yml` (new)
- `.github/workflows/build-and-publish-maproom-mcp.yml` (update trigger)
- `packages/cli/package.json` (rename, config)
- `packages/cli/.npmignore` (new)
- `packages/cli/scripts/release.mjs` (update tags, remove publish)
- `packages/maproom-mcp/scripts/release.js` (update tags)

**Unchanged files**:
- `crates/maproom/` - Rust code unchanged
- `packages/cli/src/` - TypeScript code unchanged
- `scripts/build-and-package.sh` - Keep for local dev builds

**External interfaces** (all stable):
- GitHub Actions API - stable, well-documented
- npm registry API - stable, semantic versioning
- Git tagging - stable protocol
- Rust toolchain - stable 1.x series
- Node.js - LTS versions

## Validation Approach

**Pre-publish validation** (automated in workflow):
1. Binary existence: All 4 platform binaries present
2. Binary size: Within expected range (5-20MB)
3. Binary execution: Native platform binary runs `--version`
4. TypeScript build: dist/ contains expected files
5. Package contents: Tarball inspection
6. Package installation: `npm pack` and test install

**Post-publish validation** (automated in workflow):
1. npm registry check: Package version exists
2. Tarball download: Can retrieve published package
3. Installation test: `npm install @crewchief/cli@{version}`
4. Execution test: Installed binary runs

**Deprecation validation** (manual + automated):
1. Old package marked deprecated on npm
2. Deprecation message points to new package
3. Final version (1.0.0) displays migration warning

## Research Summary

**Key insights**:
1. The maproom-mcp workflow provides a proven template - copy and adapt
2. Package-scoped tags are the standard monorepo pattern - use them for both packages
3. Cross-compilation works well for Linux targets - copy MCP's approach
4. Validation catches most issues - invest in comprehensive checks
5. Breaking change (rename) is acceptable given low user count - v1.0.0 signals this clearly

**Best practices identified**:
- Matrix builds for parallelism and efficiency
- Artifact passing between jobs for clean separation
- Comprehensive validation before publish (fail fast)
- Dry-run support for testing workflows
- Size checks prevent bloated binaries
- Execution tests catch runtime issues

**Potential improvements** (out of scope for MVP):
- Reusable workflow to reduce duplication
- Automated changelog generation
- GitHub Releases with release notes
- Artifact checksums for security
- Signing binaries

**Decision rationale**:
- **Package rename**: Aligns with org convention, worth breaking change
- **v1.0.0**: Semantic versioning - breaking change deserves major bump
- **Package-scoped tags**: Standard pattern, clearer than prefixes
- **Independent versioning**: Packages serve different purposes, no coupling needed
- **Deprecation warning**: Responsible migration path for existing users
