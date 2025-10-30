# Ticket: CFGVER-1001: Implement `.maproom-version` file schema and creation logic

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create the version tracking file schema and logic for creating, reading, and hashing configuration files. This establishes the foundation for detecting config updates by tracking package version, config version, and file integrity hashes.

## Background
The root cause of config drift in Maproom MCP is that the CLI doesn't know what version of config it has cached. Pattern-based detection (checking for specific strings) is fragile and misses architectural changes. We need explicit version tracking using a `.maproom-version` file that stores:
- Package version from package.json
- Config version (for schema evolution)
- SHA-256 hashes of all config files
- Timestamps for audit trail

Reference: `analysis.md` lines 155-158: "The CLI doesn't know what version of the config it has. Without version tracking: Can't detect updates automatically, Can't show meaningful error messages, Can't decide whether to update or not, Must rely on brittle pattern matching."

## Acceptance Criteria
- [ ] TypeScript interface or JSON schema defines version file structure matching `architecture.md` lines 46-62
- [ ] Function `createVersionFile()` creates version file with current package version from `packages/maproom-mcp/package.json`
- [ ] Function `computeFileHash()` computes SHA-256 hash for config files (docker-compose.yml, init.sql, Dockerfile.mcp-server)
- [ ] Function `readVersionFile()` reads and validates version file, returns null on corruption
- [ ] Function `writeVersionFile()` writes version file to `~/.maproom-mcp/.maproom-version` with 0o600 permissions
- [ ] Cache directory is created with 0o700 permissions if it doesn't exist
- [ ] All timestamps use ISO 8601 format (YYYY-MM-DDTHH:mm:ss.sssZ)

## Technical Requirements
- Use Node.js `crypto.createHash('sha256')` for file hashing
- Use Node.js `fs.writeFileSync()` with `mode: 0o600` for version file creation
- Read package version from `packages/maproom-mcp/package.json` dynamically
- Handle missing cache directory by creating it with `fs.mkdirSync(dir, { recursive: true, mode: 0o700 })`
- Files to hash: `docker-compose.yml`, `init.sql`, `Dockerfile.mcp-server`
- Version file location: `~/.maproom-mcp/.maproom-version`
- JSON structure must match schema:
  ```json
  {
    "package_version": "1.2.3",
    "config_version": "1.2.3",
    "last_updated": "2024-10-30T15:30:00.000Z",
    "files": {
      "docker-compose.yml": {
        "hash": "sha256:abc123...",
        "size": 2048,
        "last_modified": "2024-10-30T15:30:00.000Z"
      }
    }
  }
  ```

## Implementation Notes
**Module Structure:**
- Create module: `packages/maproom-mcp/src/config-manager.ts`
- Export functions: `createVersionFile()`, `computeFileHash()`, `readVersionFile()`, `writeVersionFile()`
- Define TypeScript interfaces for version file structure
- Module will be imported from compiled output in `dist/` by CLI (`bin/cli.cjs`)

**TypeScript Interfaces:**
```typescript
export interface VersionFileMetadata {
  package_version: string;
  config_version: string;
  last_updated: string;
  files: Record<string, FileMetadata>;
}

export interface FileMetadata {
  hash: string;
  size: number;
  last_modified: string;
}
```

**Security (from `security-review.md`):**
- Use `sanitizePath()` function to prevent path traversal (lines 41-56):
  ```typescript
  function sanitizePath(filename: string): string {
    if (filename.includes('/') || filename.includes('\\')) {
      throw new Error('Invalid filename: must not contain path separators');
    }
    const resolved = path.resolve(CACHE_DIR, filename);
    if (!resolved.startsWith(CACHE_DIR)) {
      throw new Error('Invalid path: outside cache directory');
    }
    return resolved;
  }
  ```
- Validate JSON schema before parsing (lines 65-92)
- File permissions: 0o600 for files, 0o700 for directories (lines 261-270)

**Architecture Reference:**
- Version file schema: `architecture.md` lines 41-62
- File hashing logic: `architecture.md` lines 325-347

## Dependencies
None (first ticket in Phase 1)

## Risk Assessment
- **Risk**: Path traversal attacks allowing write outside cache directory
  - **Mitigation**: Use `sanitizePath()` function to validate all file paths stay within `~/.maproom-mcp/`

- **Risk**: JSON injection or prototype pollution from malformed version file
  - **Mitigation**: Validate JSON schema with `isValidVersionFile()` before using parsed data

- **Risk**: Hash collision allowing integrity bypass
  - **Mitigation**: Use SHA-256 (industry standard, collision-resistant), acceptable risk for config files

## Files/Packages Affected
- **Create**: `packages/maproom-mcp/src/config-manager.ts`
- **Read**: `packages/maproom-mcp/package.json` (to get current version)
- **Read**: Config files in `~/.maproom-mcp/` (docker-compose.yml, init.sql, Dockerfile.mcp-server)
- **Write**: `~/.maproom-mcp/.maproom-version`

**Build Note**: TypeScript source is compiled to `dist/config-manager.js` which is imported by `bin/cli.cjs` using `require('../dist/config-manager.js')`
