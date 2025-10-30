# Ticket: CFGVER-1003: Implement SHA-256 file integrity verification

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
Implement file integrity verification using SHA-256 hashes to detect manual edits or corruption of cached config files. This prevents using tampered or corrupted configs by comparing stored hashes with actual file hashes.

## Background
Users may manually edit cached configs at `~/.maproom-mcp/`, or files may become corrupted. Without integrity checking, the CLI would use these modified/corrupted files, leading to:
- Docker container startup failures
- Database initialization errors
- Security vulnerabilities from tampered configs

Integrity verification ensures all files match their expected state before use. If corruption is detected, the CLI will trigger an update to restore clean configs.

Reference: `architecture.md` lines 101-112: File integrity verification detects missing files and hash mismatches.

Reference: `security-review.md` lines 107-132: TOCTOU attack mitigation by reading file once and computing hash of read content.

## Acceptance Criteria
- [ ] Function `verifyFileIntegrity(versionFileData)` accepts version file data object
- [ ] Returns detailed result object: `{ valid: boolean, corruptedFiles: Array<{filename: string, reason: string}> }`
- [ ] Detects missing files with reason code: `'missing'`
- [ ] Detects hash mismatches with reason code: `'hash_mismatch'`
- [ ] Verifies all files listed in version file's `files` object
- [ ] Uses SHA-256 hash computation (same algorithm as file creation)
- [ ] Reads each file only once to prevent TOCTOU attacks
- [ ] Returns `{ valid: true, corruptedFiles: [] }` when all files are valid

## Technical Requirements
- Use Node.js `crypto.createHash('sha256')` for hash computation
- Use `fs.existsSync()` to check file existence (or `fs.lstatSync()` with try/catch)
- Use `fs.readFileSync()` to read file content once
- Compute hash of read content before using it
- Use `fs.lstatSync()` to check file type (prevent symlink attacks)
- Handle missing files gracefully (don't crash, add to corruptedFiles array)
- Handle file read errors gracefully (permissions, etc.)
- Compare hashes using strict equality (`===`)

## Implementation Notes
**Function Location:**
- Add to module: `packages/maproom-mcp/src/config-manager.ts`
- Export function: `verifyFileIntegrity(versionFileData)`

**TypeScript Interfaces:**
```typescript
export interface IntegrityCheckResult {
  valid: boolean;
  corruptedFiles: CorruptedFile[];
}

export interface CorruptedFile {
  filename: string;
  reason: 'missing' | 'hash_mismatch' | 'not_regular_file';
}
```

**Implementation Pattern (from `security-review.md` lines 113-132):**
```typescript
export function verifyFileIntegrity(versionFileData: VersionFileMetadata): IntegrityCheckResult {
  const corruptedFiles: CorruptedFile[] = [];

  for (const [filename, metadata] of Object.entries(versionFileData.files)) {
    const filepath = path.join(CACHE_DIR, filename);

    // Check file type (prevent symlink attacks)
    try {
      const stats = fs.lstatSync(filepath);
      if (!stats.isFile()) {
        corruptedFiles.push({ filename, reason: 'not_regular_file' });
        continue;
      }
    } catch (error) {
      corruptedFiles.push({ filename, reason: 'missing' });
      continue;
    }

    // Read file once and compute hash
    const content = fs.readFileSync(filepath);
    const hash = crypto.createHash('sha256')
      .update(content)
      .digest('hex');
    const hashWithPrefix = `sha256:${hash}`;

    // Compare with stored hash
    if (hashWithPrefix !== metadata.hash) {
      corruptedFiles.push({ filename, reason: 'hash_mismatch' });
    }
  }

  return {
    valid: corruptedFiles.length === 0,
    corruptedFiles
  };
}
```

**Security Considerations:**
- **TOCTOU Prevention**: Read file once, compute hash, then use that content (lines 107-132 of security-review.md)
- **Symlink Attack Prevention**: Use `lstatSync()` to check file type before reading (lines 213-233)
- **Hash Format**: Store as "sha256:abc123..." for explicit algorithm identification
- **Error Handling**: Treat unreadable files as corrupted (safe default)

**Integration with Update Detection:**
- Called from `needsConfigUpdate()` function (CFGVER-1002)
- If integrity check fails, return `{ needsUpdate: true, reason: 'integrity_failure', corruptedFiles: [...] }`

## Dependencies
- **CFGVER-1001** - Requires `readVersionFile()` to get file metadata
- **CFGVER-1001** - Uses same hash algorithm as `computeFileHash()`

## Risk Assessment
- **Risk**: Hash collision allowing corrupted files to pass verification
  - **Mitigation**: Use SHA-256 (no practical collision attacks known), acceptable risk

- **Risk**: TOCTOU attack (modify file between hash check and use)
  - **Mitigation**: Read file once, compute hash of read content, use that verified content

- **Risk**: Symlink attack (replace config with symlink to sensitive file)
  - **Mitigation**: Check file type with `lstatSync()`, reject non-regular files

- **Risk**: Uncaught exceptions from file operations crashing CLI
  - **Mitigation**: Wrap file operations in try/catch, add to corruptedFiles array

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `verifyFileIntegrity()` function)
- **Read**: Config files in `~/.maproom-mcp/` (docker-compose.yml, init.sql, Dockerfile.mcp-server)
- **Read**: `~/.maproom-mcp/.maproom-version` (for file metadata with hashes)
