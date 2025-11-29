# MCPDB Security Review - MCP Server SQLite Support

## Overview

This security review assesses the risks introduced by adding SQLite backend support to the MCP server. The changes involve URL parsing, file path handling, and test infrastructure.

## Threat Model

### Attack Surface

| Component | Attack Vector | Risk Level |
|-----------|---------------|------------|
| URL Parser | Malicious URL injection | Medium |
| Path Expansion | Path traversal attacks | Medium |
| File Validation | Symlink attacks | Low |
| Test Fixtures | Test data poisoning | Low |

## Security Analysis

### 1. URL Parsing Security

#### Risk: URL Injection

**Scenario**: Attacker controls `MAPROOM_DATABASE_URL` and injects malicious path

**Mitigation**:
```typescript
// Sanitize before passing to daemon
function parseSqliteUrl(url: string): DatabaseConfig {
  // Only accept sqlite:// prefix
  if (!url.startsWith('sqlite://')) {
    throw new Error('Invalid SQLite URL scheme')
  }

  // Extract path portion
  const path = url.slice('sqlite://'.length)

  // Reject empty paths
  if (!path || path.trim() === '') {
    throw new Error('SQLite URL must include a path')
  }

  // Reject URLs that look like other schemes
  if (path.includes('://')) {
    throw new Error('Invalid SQLite URL: contains embedded scheme')
  }

  return { type: 'sqlite', url, path: resolvePath(path) }
}
```

**Assessment**: Low risk - environment variables require system access

### 2. Path Traversal Security

#### Risk: Directory Traversal

**Scenario**: Path like `sqlite://../../../etc/passwd` attempts to access sensitive files

**Mitigation**:
```typescript
function validateSqlitePath(path: string): void {
  const resolved = resolvePath(path)

  // SQLite database must end with .db or .sqlite extension
  const validExtensions = ['.db', '.sqlite', '.sqlite3']
  if (!validExtensions.some(ext => resolved.endsWith(ext))) {
    throw new Error(
      `Invalid SQLite database path: must end with ${validExtensions.join(', ')}`
    )
  }

  // Additional validation: file should be in expected locations
  // This is informational, not enforced (users may have custom paths)
  const expectedPaths = [
    homedir(),           // ~/.maproom/
    process.cwd(),       // Project directory
    '/tmp',              // Temporary files
  ]

  const isExpected = expectedPaths.some(base =>
    resolved.startsWith(base)
  )

  if (!isExpected) {
    console.warn(`SQLite path outside expected directories: ${resolved}`)
  }
}
```

**Assessment**: Low risk - daemon only reads database, doesn't execute arbitrary paths

### 3. Symlink Attacks

#### Risk: Symlink Following

**Scenario**: Symlink at expected path points to sensitive file

**Mitigation**:
- SQLite opens files in database mode, not arbitrary read
- Node.js `existsSync` follows symlinks (by design for usability)
- Daemon validates SQLite file format before operations

**Assessment**: Low risk - SQLite format validation prevents arbitrary file access

### 4. Environment Variable Security

#### Current State

```typescript
// daemon.ts
daemonClient = new DaemonClient({
  env: {
    MAPROOM_DATABASE_URL: process.env.MAPROOM_DATABASE_URL,
    OPENAI_API_KEY: process.env.OPENAI_API_KEY,
    ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY,
    // ...
  }
})
```

**Analysis**:
- Only whitelisted environment variables passed to daemon
- No user-controlled variables passed directly
- SQLite URL comes from trusted `MAPROOM_DATABASE_URL`

**Assessment**: Good - existing whitelist approach maintained

### 5. Test Data Security

#### Risk: Malicious Fixture

**Scenario**: Pre-indexed SQLite fixture contains malicious data

**Mitigation**:
- Fixture generated from known test code in repository
- Fixture checked into git, subject to code review
- Test isolation prevents production impact

**Assessment**: Low risk - fixture is code-reviewed artifact

## Security Recommendations

### Must Implement (MVP)

1. **Validate URL scheme strictly**
   ```typescript
   if (!url.startsWith('sqlite://')) throw new Error('Invalid scheme')
   ```

2. **Sanitize path before logging**
   ```typescript
   log.info({ path: path.replace(homedir(), '~') }, 'SQLite path')
   ```

3. **Validate file extension**
   ```typescript
   if (!path.endsWith('.db') && !path.endsWith('.sqlite')) {
     throw new Error('Invalid SQLite file extension')
   }
   ```

### Should Implement (Quality)

1. **Log unusual paths with warning level**
   ```typescript
   if (!isExpectedPath(resolved)) {
     log.warn({ path: resolved }, 'SQLite path outside expected directories')
   }
   ```

2. **Document security considerations**
   - Note that SQLite files are read/write
   - Recommend appropriate file permissions (600)

### Nice to Have (Enterprise)

1. **Path allowlist configuration**
   - Environment variable for allowed base paths
   - Reject paths outside allowlist

2. **File permission validation**
   - Warn if SQLite file is world-readable
   - Recommend secure permissions

## Comparison with PostgreSQL Path

| Security Aspect | PostgreSQL | SQLite |
|-----------------|------------|--------|
| Connection string | Network URL | File path |
| Credential exposure | In URL | None |
| Network attack surface | Yes | No |
| Local file access | No | Yes |
| Privilege escalation | Via SQL | Via file system |

**Summary**: SQLite reduces network attack surface but introduces local file access considerations. Overall security posture is equivalent for typical deployments.

## Known Gaps

### Gap 1: No Runtime Path Validation

**Description**: Path is validated at startup but not monitored for changes

**Risk**: Low - environment variables don't change at runtime

**Deferred**: Enterprise feature, not MVP

### Gap 2: No Audit Logging

**Description**: Database access not logged with user context

**Risk**: Low - MCP server runs in user context

**Deferred**: Enterprise feature, not MVP

## Conclusion

The SQLite backend support introduces minimal security risk:

1. **URL parsing** is strict and validates scheme
2. **Path handling** uses standard Node.js resolution
3. **File validation** ensures SQLite format before operations
4. **Environment passing** maintains existing whitelist approach

**Recommendation**: Proceed with implementation. Security measures are appropriate for the threat model.

## Checklist

- [x] URL scheme validation implemented
- [x] Path traversal mitigations identified
- [x] Environment variable handling reviewed
- [ ] File extension validation (to implement)
- [ ] Security documentation (to write)
- [ ] Code review with security focus (at implementation)
