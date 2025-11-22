# Security Review: Open Tool Path Resolution Fix

**Date:** 2025-11-18
**Project:** OPNFIX - Open Tool Path Resolution Fix
**Reviewer:** Architecture Analysis
**Risk Level:** Medium (File system access, path traversal potential)

## Security Objectives

1. **Prevent path traversal attacks** - Users cannot read files outside repository
2. **Validate all paths** - No blind trust of database data
3. **Fail safely** - Security failures are explicit errors, not silent bypasses
4. **No information leakage** - Error messages don't reveal sensitive paths

## Threat Model

### Threat 1: Malicious Database Injection

**Scenario:** Attacker gains database write access and injects malicious paths.

**Attack Vector:**
```sql
-- Attacker inserts worktree with traversal path
INSERT INTO maproom.worktrees (repo_id, name, abs_path)
VALUES (1, 'main', '/etc')

-- Attacker inserts file with sensitive target
INSERT INTO maproom.files (worktree_id, relpath)
VALUES (1, 'passwd')
```

**Result:** Open tool reads `/etc/passwd`

**Likelihood:** Low (requires database compromise)
**Impact:** High (arbitrary file read)
**Overall Risk:** Medium

### Threat 2: Path Traversal via relpath Parameter

**Scenario:** User/attacker provides crafted relpath parameter.

**Attack Vector:**
```typescript
openTool({
  relpath: '../../../etc/passwd',
  worktree: 'main'
})
```

**Result:** If validation fails, reads `/workspace/../../../etc/passwd` = `/etc/passwd`

**Likelihood:** Medium (public API)
**Impact:** High (arbitrary file read)
**Overall Risk:** High

### Threat 3: Symlink Escape

**Scenario:** Repository contains symlink pointing outside repository.

**Attack Vector:**
```bash
# In repository
ln -s /etc/passwd evil.txt

# Index it
maproom scan /workspace

# Read via open tool
openTool({ relpath: 'evil.txt', worktree: 'main' })
```

**Result:** Reads `/etc/passwd` via symlink

**Likelihood:** Low (requires repo write access)
**Impact:** High (arbitrary file read)
**Overall Risk:** Medium

### Threat 4: Database Pollution as Attack

**Scenario:** Database pollution could be malicious, not accidental.

**Attack Vector:**
```sql
-- Insert worktree with path to sensitive directory
INSERT INTO maproom.worktrees (repo_id, name, abs_path)
VALUES (1, 'main', '/home/user/.ssh')

-- Insert file targeting private key
INSERT INTO maproom.files (worktree_id, relpath)
VALUES (1, 'id_rsa')
```

**Result:** Open tool reads `/home/user/.ssh/id_rsa`

**Likelihood:** Low (requires database compromise)
**Impact:** Critical (credential theft)
**Overall Risk:** High if database is compromised

## Current Security Controls

### Existing: validatePath() Function

**Location:** `packages/maproom-mcp/src/utils/validation.ts`

**What it does:**
```typescript
export function validatePath(relpath: string): string {
  // 1. Reject empty paths
  if (!relpath || relpath.trim() === '') {
    throw new ValidationError('Path cannot be empty', 'INVALID_PATH')
  }

  // 2. Normalize path (resolve ./ and ../)
  const normalized = path.normalize(relpath)

  // 3. Reject absolute paths
  if (path.isAbsolute(normalized)) {
    throw new ValidationError('Absolute paths not allowed', 'INVALID_PATH')
  }

  // 4. Reject paths that traverse up (../)
  if (normalized.startsWith('..') || normalized.includes(path.sep + '..')) {
    throw new ValidationError('Path traversal detected', 'INVALID_PATH')
  }

  // 5. Reject null bytes (path injection)
  if (normalized.includes('\0')) {
    throw new ValidationError('Invalid characters in path', 'INVALID_PATH')
  }

  return normalized
}
```

**Strengths:**
- ✅ Rejects `../` traversal
- ✅ Rejects absolute paths
- ✅ Rejects null byte injection
- ✅ Normalizes paths

**Weaknesses:**
- ⚠️ Runs BEFORE database query (validates parameter, not database data)
- ⚠️ Doesn't validate final joined path
- ⚠️ Doesn't detect symlink escapes

### Existing: validateWithinRepo() Function

**Location:** `packages/maproom-mcp/src/utils/validation.ts`

**What it does:**
```typescript
export function validateWithinRepo(
  absolutePath: string,
  repoRoot: string
): void {
  const normalizedPath = path.normalize(absolutePath)
  const normalizedRoot = path.normalize(repoRoot)

  // Check if path starts with repo root
  if (!normalizedPath.startsWith(normalizedRoot)) {
    throw new ValidationError(
      `Path escapes repository boundary: ${absolutePath}`,
      'INVALID_PATH'
    )
  }
}
```

**Strengths:**
- ✅ Validates final path is within repository
- ✅ Runs AFTER path construction
- ✅ Uses normalized paths

**Weaknesses:**
- ⚠️ String-based check (can be bypassed with symlinks)
- ⚠️ Doesn't validate repoRoot itself is safe

**Critical:** This is called in `readFileFromFilesystem()` line 104, BEFORE reading the file.

## Security Improvements in This Project

### Improvement 1: Validate ALL Candidate Paths

**Current:** Only validates the returned path
**Proposed:** Validate EVERY candidate before trying it

```typescript
for (const row of rows) {
  const candidatePath = path.join(row.abs_path, relpath)

  // Validate BEFORE filesystem check
  validateWithinRepo(candidatePath, row.abs_path)

  if (await fileExists(candidatePath)) {
    return row.abs_path
  }
}
```

**Benefit:** Even if database is compromised, traversal is blocked.

### Improvement 2: Validate abs_path is Within Expected Boundaries

**Problem:** Database could contain `/etc` as abs_path.

**Solution:** Add repository root validation:

```typescript
async function getWorktreePath(
  client: Client,
  worktreeName: string,
  relpath: string,
  expectedRoot?: string  // Optional: validate abs_path is under this
): Promise<string> {
  const { rows } = await client.query(...)

  for (const row of rows) {
    // NEW: Validate abs_path itself is within expected bounds
    if (expectedRoot && !row.abs_path.startsWith(expectedRoot)) {
      log.warn({ abs_path: row.abs_path, expectedRoot },
        'Worktree abs_path outside expected root - skipping')
      continue
    }

    const candidatePath = path.join(row.abs_path, relpath)
    validateWithinRepo(candidatePath, row.abs_path)

    if (await fileExists(candidatePath)) {
      return row.abs_path
    }
  }
}
```

**Trade-off:**
- ✅ Pro: Blocks database-level attacks
- ⚠️ Con: Requires knowing expected root (may not always be available)

**Decision:** Implement as optional parameter for extra security when available.

### Improvement 3: Symlink Detection

**Problem:** `fs.readFile()` follows symlinks automatically.

**Solution:** Check if path is a symlink before reading:

```typescript
async function readFileFromFilesystem(
  worktreePath: string,
  relpath: string,
  config: OpenToolConfig
): Promise<string> {
  const absolutePath = path.join(worktreePath, relpath)

  // Validate within repo
  validateWithinRepo(absolutePath, worktreePath)

  // NEW: Check if it's a symlink
  const stats = await fs.lstat(absolutePath)
  if (stats.isSymbolicLink()) {
    // Resolve symlink and validate target
    const realPath = await fs.realpath(absolutePath)
    validateWithinRepo(realPath, worktreePath)

    // Log warning but allow (symlinks within repo are OK)
    log.debug({ path: absolutePath, target: realPath },
      'Following symlink within repository')
  }

  // Check file size
  await validateFileSize(absolutePath, config.maxFileSize)

  // Read file
  const content = await fs.readFile(absolutePath, 'utf8')
  return content
}
```

**Benefit:** Detects and validates symlink targets before following.

**Trade-off:**
- ✅ Pro: Prevents symlink escape attacks
- ⚠️ Con: Extra filesystem call (lstat + realpath)
- ⚠️ Con: May break legitimate symlinks in repository

**Decision:** Implement symlink validation, allow symlinks within repo boundaries.

## Security Testing Requirements

### Test 1: Path Traversal in relpath Parameter

```typescript
it('should reject path traversal in relpath', async () => {
  await expect(
    openTool({ relpath: '../../../etc/passwd', worktree: 'main' })
  ).rejects.toThrow('Path traversal detected')
})
```

**Validates:** validatePath() catches user-supplied traversal.

### Test 2: Path Traversal in Database

```typescript
it('should reject traversal from database abs_path', async () => {
  // Insert malicious worktree
  await client.query(
    `INSERT INTO maproom.worktrees (repo_id, name, abs_path)
     VALUES (1, 'evil', '/etc')`
  )
  await client.query(
    `INSERT INTO maproom.files (worktree_id, relpath)
     VALUES (1, 'passwd')`
  )

  // Should reject
  await expect(
    openTool({ relpath: 'passwd', worktree: 'evil' })
  ).rejects.toThrow('escapes repository boundary')
})
```

**Validates:** validateWithinRepo() catches database-level attacks.

### Test 3: Symlink Escape

```typescript
it('should reject symlink pointing outside repository', async () => {
  // Create repository with evil symlink
  await fs.symlink('/etc/passwd', '/workspace/evil.txt')

  // Index it
  await indexFile('/workspace/evil.txt')

  // Should detect and reject
  await expect(
    openTool({ relpath: 'evil.txt', worktree: 'main' })
  ).rejects.toThrow('escapes repository boundary')
})
```

**Validates:** Symlink validation works.

### Test 4: Absolute Path in relpath

```typescript
it('should reject absolute path in relpath', async () => {
  await expect(
    openTool({ relpath: '/etc/passwd', worktree: 'main' })
  ).rejects.toThrow('Absolute paths not allowed')
})
```

**Validates:** validatePath() catches absolute paths.

### Test 5: Null Byte Injection

```typescript
it('should reject null bytes in relpath', async () => {
  await expect(
    openTool({ relpath: 'test\0.txt', worktree: 'main' })
  ).rejects.toThrow('Invalid characters in path')
})
```

**Validates:** validatePath() catches injection attempts.

## Security Best Practices

### 1. Defense in Depth

**Layers of protection:**
```
1. Parameter validation (validatePath) → Reject bad user input
2. Database query filtering → Only plausible candidates
3. Path construction validation (validateWithinRepo) → Verify joined path
4. Symlink validation → Verify link targets
5. Filesystem access check → Final permission check
```

**Principle:** Even if one layer fails, others prevent attack.

### 2. Fail Securely

**Insecure pattern:**
```typescript
try {
  validatePath(relpath)
} catch (error) {
  // Ignore error, proceed anyway ❌
}
```

**Secure pattern:**
```typescript
try {
  validatePath(relpath)
} catch (error) {
  // Propagate error, halt execution ✅
  throw error
}
```

**Rule:** Security failures MUST stop execution.

### 3. Minimal Information in Errors

**Insecure:**
```typescript
throw new Error(`File not found: ${absolutePath}`)
// Reveals internal path structure to attacker
```

**Secure:**
```typescript
throw new ValidationError(
  `File '${relpath}' not accessible in worktree '${worktreeName}'`,
  'FILE_NOT_FOUND'
)
// Only reveals user-provided parameters
```

**Rule:** Error messages must not leak sensitive system information.

### 4. Validate Early and Often

**Best Practice:**
- ✅ Validate parameters at API boundary
- ✅ Validate database data before use
- ✅ Validate constructed paths before filesystem access
- ✅ Validate symlink targets if followed

**Rule:** Trust nothing, validate everything.

## Risk Mitigation Summary

| Threat | Current Risk | Mitigated Risk | Mitigation |
|--------|--------------|----------------|------------|
| Path traversal via parameter | High | Low | validatePath() rejects `../` |
| Database injection | Medium | Low | validateWithinRepo() on all candidates |
| Symlink escape | Medium | Low | Symlink target validation (new) |
| Absolute path injection | High | Low | validatePath() rejects absolute paths |
| Null byte injection | Medium | Low | validatePath() rejects null bytes |
| Information leakage | Low | Low | Minimal error messages |

**Overall Security Posture:** Strong (with proposed improvements)

## Security Requirements for Ship

Before this project ships, ALL of the following MUST be true:

- ✅ Parameter validation (validatePath) works for all inputs
- ✅ All candidate paths validated (validateWithinRepo)
- ✅ Symlink targets validated before following
- ✅ Security tests pass (5 tests listed above)
- ✅ Error messages don't leak sensitive paths
- ✅ Code review by security-aware developer
- ✅ Manual penetration testing completed

## Open Security Questions

### Q1: Should we block all symlinks?

**Options:**
1. Block all symlinks (most secure, may break workflows)
2. Allow symlinks within repository (balanced)
3. Allow all symlinks (insecure)

**Recommendation:** Option 2 - Allow symlinks if target is within repository.

### Q2: Should we validate abs_path is under /workspace?

**Trade-offs:**
- ✅ Pro: Prevents database attacks pointing to /etc
- ❌ Con: Hardcodes deployment path assumption
- ❌ Con: Breaks local development with different paths

**Recommendation:** Make it configurable via environment variable:
```typescript
const ALLOWED_ROOT = process.env.MAPROOM_ALLOWED_ROOT || '/workspace'
```

### Q3: Should we log security violations?

**Yes, but carefully:**
- ✅ Log at WARN level (not ERROR - not application error)
- ✅ Include attempt details for forensics
- ❌ Don't log sensitive paths
- ✅ Rate-limit logging (prevent log flooding)

**Recommendation:** Implement security event logging.

## Conclusion

**Security Assessment:** This fix maintains strong security posture.

**Key Strengths:**
- Multiple validation layers
- Existing security functions are robust
- New code preserves security guarantees
- Symlink handling added

**Remaining Risks:**
- Database compromise still enables some attacks (low likelihood)
- Symlinks within repository are trusted (acceptable risk)

**Ship Decision:** Safe to ship with proposed improvements.

**Next:** Create execution plan with implementation phases.
