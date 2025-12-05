# Security Review: Maproom Binary Configuration

## Security Assessment

### Overview

This project adds configuration-based binary path resolution, which introduces a potential attack vector: **arbitrary binary execution**. The primary security concern is preventing malicious configuration files from executing untrusted binaries.

**Risk Level:** Medium (local execution context, user controls config files)

**Threat Model:**
- Attacker modifies crewchief.config.js to point to malicious binary
- User runs crewchief command
- Malicious binary executes with user's permissions

**Mitigations:**
- Config files are local (user must have write access to repository)
- No remote config loading
- Environment variable override exists for emergency recovery
- Explicit user action required (running CLI command)

### Authentication & Authorization

**Not Applicable** - This is a local CLI tool with no authentication or user accounts.

**Access Control:**
- Binary execution runs with user's file system permissions
- No privilege escalation
- No sudo/root required

**Config File Permissions:**
- crewchief.config.js must be writable by user (normal file permissions)
- crewchief.config.local.js is gitignored (not shared across team)
- Malicious config requires local write access (already compromised)

### Data Protection

**Sensitive Data:** None

**Config File Contents:**
- Binary paths (file system locations)
- No credentials or secrets
- No personally identifiable information (PII)
- No API keys or tokens

**Environment Variables:**
- CREWCHIEF_MAPROOM_BIN contains file path (not sensitive)
- No secrets in environment variables

### Input Validation

#### Config File Validation

**Zod Schema Validation:**
```typescript
maproomBinaryPath: z.string().optional()
```

**Current Validation:**
- Type check: Must be string (Zod enforces)
- Optional: Default is undefined (safe)
- No format validation (intentional - allows flexibility)

**Missing Validation:**
- No path sanitization (accepts any string)
- No allowlist of approved binaries
- No signature verification

**Risk Assessment:**
- **Medium Risk**: Malicious config can specify any binary path
- **Acceptable for MVP**: User controls config file (trusted input)
- **Future Enhancement**: Add optional path allowlist

#### Path Resolution Security

**Relative Path Handling:**
```typescript
const resolved = path.resolve(options.configPath)
```

**Security Properties:**
- `path.resolve()` normalizes paths (prevents ../ traversal)
- Absolute paths used as-is (no normalization needed)
- No URL parsing (no protocol injection)

**Potential Issues:**
- Symlink following (resolved path might point elsewhere)
- Relative paths could escape repository (e.g., ../../../usr/bin/malicious)

**Risk Assessment:**
- **Low Risk**: User must create malicious symlink/config themselves
- **Acceptable**: Matches existing file system behavior

#### Binary Execution Security

**Execution Pattern:**
```typescript
spawnSync(result.path, args, { stdio: 'inherit' })
```

**Security Properties:**
- Direct execution (no shell interpretation)
- Arguments passed as array (no injection)
- stdio inheritance (no output manipulation)
- Synchronous (blocks until completion)

**Potential Issues:**
- No binary signature verification
- No hash checking
- Trust on first use (no pinning)

**Risk Assessment:**
- **Low Risk**: Same as user running the binary directly
- **Acceptable**: CLI tools typically trust binaries on PATH

### Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| No binary signature verification | Medium | User controls config file, manual verification required | Accepted for MVP |
| No path allowlist | Medium | Document trusted binary locations in security docs | Accepted for MVP |
| Symlink following | Low | Standard file system behavior, user responsibility | Accepted |
| No hash pinning | Low | Binary updates are expected, hash would break | Accepted |
| Config file tampering | High | Require code review for config changes in PRs | Accepted |
| Environment variable injection | Low | User controls environment, expected behavior | Accepted |

### Additional Security Considerations

#### Supply Chain Security

**Risk:** Malicious binary distributed via npm package

**Mitigation:**
- Binaries built from source in CI (GitHub Actions)
- Release process requires code review
- npm package signing (npm provenance)

**Not in scope for this project** - Existing release process handles this

#### File System Security

**Risk:** Unauthorized write access to config file

**Mitigation:**
- Standard file system permissions
- crewchief.config.local.js is gitignored (not committed)
- Code review for crewchief.config.js changes

**Best Practice:**
```bash
# Prevent accidental commits
echo 'crewchief.config.local.js' >> .gitignore

# Restrict permissions (optional)
chmod 600 crewchief.config.local.js
```

#### Command Injection

**Risk:** Config path contains shell metacharacters

**Assessment:** **Not vulnerable**
- `spawnSync` does not invoke shell
- Path passed as string argument (not interpolated)
- No eval or exec usage

**Example Safe Handling:**
```typescript
// Safe - no shell interpretation
spawnSync('/path/with spaces/binary', ['arg1', 'arg2'])

// Unsafe (NOT used in this project)
exec(`${configPath} arg1 arg2`)  // DON'T DO THIS
```

#### Path Traversal

**Risk:** Config path escapes intended directory

**Assessment:** **Low risk, acceptable**
- Relative paths are allowed (intentional)
- User controls config file (trusted input)
- No remote config loading

**Example:**
```javascript
// Allowed (user may want this)
export default {
  repository: {
    maproomBinaryPath: '../../../usr/local/bin/crewchief-maproom'
  }
}
```

## MVP Security Scope

### In Scope for MVP

- [x] Zod validation of config schema
- [x] Path resolution (normalize paths)
- [x] Safe binary execution (no shell)
- [x] Warning on invalid paths
- [x] Environment variable override
- [x] Documentation of security considerations

### Out of Scope for MVP

- [ ] Binary signature verification
- [ ] Path allowlist/denylist
- [ ] Hash pinning
- [ ] Config file signing
- [ ] Audit logging of binary execution
- [ ] Sandboxed execution

### Future Enhancements (Post-MVP)

**Optional Path Allowlist:**
```typescript
export const RepositorySchema = z.object({
  maproomBinaryPath: z.string().optional(),
  allowedBinaryPaths: z.array(z.string()).optional(), // Future
})

// Validate path is in allowlist
if (config.allowedBinaryPaths && !config.allowedBinaryPaths.includes(resolved)) {
  throw new Error('Binary path not in allowlist')
}
```

**Binary Hash Verification:**
```typescript
export const RepositorySchema = z.object({
  maproomBinaryPath: z.string().optional(),
  maproomBinarySha256: z.string().optional(), // Future
})

// Verify hash before execution
if (config.maproomBinarySha256) {
  const actualHash = await hashFile(resolved)
  if (actualHash !== config.maproomBinarySha256) {
    throw new Error('Binary hash mismatch - possible tampering')
  }
}
```

## Security Checklist

### Code Security

- [x] No hardcoded secrets
- [x] No credentials in config files
- [x] No SQL injection vulnerabilities (not applicable)
- [x] No XSS vulnerabilities (not applicable)
- [x] No command injection (using spawnSync safely)
- [x] No path traversal vulnerabilities (user controls config)
- [x] Input validation on config fields (Zod schema)
- [x] Proper error handling (no stack traces to users)

### Configuration Security

- [x] Config files are local (not fetched remotely)
- [x] Zod validation prevents type confusion
- [x] Optional fields have safe defaults
- [x] Invalid paths trigger warnings
- [x] Environment variable override exists

### Execution Security

- [x] Binary execution uses spawnSync (no shell)
- [x] Arguments passed as array (no injection)
- [x] stdio inherited (no output manipulation)
- [x] Exit codes propagated correctly
- [x] Error messages don't leak sensitive info

### Documentation Security

- [x] Document security considerations
- [x] Warn about malicious config files
- [x] Document trusted binary sources
- [x] Include example secure configuration
- [ ] Add security section to README (Phase 3)

## Security Best Practices for Users

### Recommended Configuration

**For Development (local only):**
```javascript
// crewchief.config.local.js (gitignored)
export default {
  repository: {
    maproomBinaryPath: './target/release/crewchief-maproom'
  }
}
```

**For Team Use (committed):**
```javascript
// crewchief.config.js (code reviewed)
export default {
  repository: {
    // Use global install, not custom path
    // maproomBinaryPath: undefined
  }
}
```

### Code Review Checklist

When reviewing PRs that change crewchief.config.js:

- [ ] Verify maproomBinaryPath is not set (prefer global install)
- [ ] If set, verify path points to trusted binary
- [ ] Check for suspicious paths (/tmp, /dev, unusual locations)
- [ ] Verify no absolute paths from untrusted sources
- [ ] Confirm binary exists and is expected version

### Emergency Recovery

If malicious config is detected:

```bash
# Override with environment variable
CREWCHIEF_MAPROOM_BIN=/usr/local/bin/crewchief-maproom crewchief maproom scan

# Or remove config
rm crewchief.config.local.js

# Or use global install (unset config)
git checkout crewchief.config.js
```

## Risk Acceptance

**Accepted Risks for MVP:**

1. **No binary signature verification**
   - **Rationale:** User controls config file (trusted input source)
   - **Compensating Control:** Code review for config changes
   - **Future:** Add optional signature verification

2. **No path allowlist**
   - **Rationale:** Developer flexibility is priority for MVP
   - **Compensating Control:** Documentation of security best practices
   - **Future:** Add optional allowlist configuration

3. **Symlink following**
   - **Rationale:** Standard file system behavior
   - **Compensating Control:** User creates symlinks (requires write access)
   - **Future:** Add option to disable symlink following

4. **Config file tampering**
   - **Rationale:** Attacker with write access to repository is already compromised
   - **Compensating Control:** File system permissions, code review
   - **Future:** Config file signing

## Conclusion

**Security Posture:** Acceptable for MVP

**Key Security Properties:**
- No remote code execution (local only)
- No privilege escalation
- No secrets handling
- Safe binary execution (no shell)
- User controls all inputs (trusted context)

**Residual Risks:**
- Malicious config file can execute arbitrary binary (mitigated by local trust model)
- No verification of binary authenticity (acceptable for MVP)

**Recommendation:** Ship this feature with documentation of security considerations.
