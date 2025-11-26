# VSCODEDB - Security Review

## Scope

This security review covers the architecture changes for adding SQLite support to the VSCode extension. The review focuses on practical security considerations for an MVP release.

## Architecture Security Analysis

### 1. File Path Handling

**Change**: New `maproom.database.sqlitePath` setting allows users to specify custom SQLite database paths.

**Risk Assessment**: LOW-MEDIUM

**Concerns**:
- Path traversal attacks (e.g., `../../etc/passwd`)
- Symlink following to sensitive locations
- Tilde expansion edge cases

**Mitigations Applied**:
```typescript
// Path expansion is controlled
function expandPath(p: string): string {
  return p.startsWith('~') ? p.replace('~', homedir()) : p
}

// Paths resolved to absolute
const resolved = isAbsolute(expanded) ? expanded : resolve(process.cwd(), expanded)
```

**Additional Safeguards** (Already in place):
- SQLite opens files read-only for queries
- File must exist before use (no arbitrary file creation)
- Path comes from user settings, not external input

**Recommendation**: ACCEPTABLE for MVP. No additional mitigations needed.

### 2. Settings Storage

**Change**: New settings stored in VSCode's configuration system.

**Risk Assessment**: LOW

**Analysis**:
- VSCode settings stored in JSON files within workspace
- No sensitive data in new settings (path is not secret)
- PostgreSQL credentials already stored in settings (existing risk)

**Recommendation**: ACCEPTABLE. No change to existing security posture.

### 3. Database Access

**Change**: Extension now accesses SQLite files directly.

**Risk Assessment**: LOW

**Analysis**:
- SQLite files are local to user's machine
- No network access required for SQLite mode
- Same permission model as any other local file
- User explicitly chooses database path

**Recommendation**: ACCEPTABLE. Local file access is standard for VSCode extensions.

### 4. Docker Bypass

**Change**: Docker containers no longer required for SQLite mode.

**Risk Assessment**: LOW (Security Improvement)

**Analysis**:
- Fewer running services = smaller attack surface
- No container networking required
- No Docker socket exposure
- Eliminates container escape concerns

**Recommendation**: SECURITY IMPROVEMENT. SQLite mode has better security profile.

## Known Gaps

### G-1: PostgreSQL Password in Settings

**Status**: Pre-existing, out of scope

**Description**: PostgreSQL credentials stored in VSCode settings are visible in JSON files.

**Current State**: Password stored in plaintext in `.vscode/settings.json`

**Future Consideration**: Could use VSCode SecretStorage API, but this is out of scope for this project.

### G-2: SQLite File Permissions

**Status**: Acceptable risk

**Description**: SQLite database files inherit filesystem permissions. If user creates database in shared location, it may be readable by others.

**Mitigation**: Default path `~/.maproom/maproom.db` is in user's home directory with standard permissions.

**User Guidance**: Document that custom paths should have appropriate permissions.

### G-3: Binary Execution

**Status**: Pre-existing, out of scope

**Description**: Extension spawns `crewchief-maproom` binary. If binary is replaced with malicious version, arbitrary code execution is possible.

**Current Mitigation**: Binary bundled with extension, signed by VSCode marketplace.

## MVP-Appropriate Mitigations

### What We're Implementing

| Risk | Mitigation | Rationale |
|------|------------|-----------|
| Path traversal | Resolve to absolute paths | Standard practice |
| Missing file | Existence check before use | Clear error messages |
| Invalid settings | Type validation in schema | VSCode handles |

### What We're NOT Implementing (Enterprise Considerations)

| Risk | Why Deferred |
|------|-------------|
| Encrypted database | SQLite doesn't support natively; would require SEE |
| Access logging | No audit requirements for personal tool |
| Path allowlist | Would complicate UX for legitimate use |
| Sandboxed file access | VSCode extension API doesn't support |

## Threat Model

### Assets

| Asset | Value | Location |
|-------|-------|----------|
| Code index | Medium | SQLite file |
| PostgreSQL credentials | Medium | VSCode settings |
| Source code (indexed) | High | User's filesystem |

### Threat Actors

| Actor | Motivation | Capability |
|-------|------------|------------|
| Local user | Curiosity | Full filesystem access |
| Malicious extension | Data theft | VSCode API access |
| Network attacker | Data theft | No access (SQLite is local) |

### Attack Vectors

| Vector | Risk | Mitigation |
|--------|------|------------|
| Settings manipulation | Low | User controls settings |
| SQLite file theft | Low | Standard file permissions |
| Path injection | Low | Settings API, not user input |

## Security Testing

### Manual Security Checks

**Before Release**:
1. Verify path expansion doesn't escape home directory
2. Verify symlink following behavior
3. Verify settings schema rejects invalid types
4. Verify error messages don't expose sensitive paths

### Automated Security Checks

**In CI**:
- TypeScript type checking (prevents type confusion)
- No new dependencies (no supply chain additions)
- Existing test suite covers path handling

## Compliance Considerations

### OWASP Relevance

| OWASP Top 10 | Applicable? | Notes |
|--------------|-------------|-------|
| A01: Broken Access Control | No | Local file access |
| A02: Cryptographic Failures | No | No encryption required |
| A03: Injection | Low | No SQL queries from user input |
| A04: Insecure Design | No | Standard VSCode patterns |
| A05: Security Misconfiguration | Low | Sensible defaults |
| A06: Vulnerable Components | N/A | No new dependencies |
| A07: Auth Failures | No | No authentication |
| A08: Data Integrity | Low | SQLite checksums |
| A09: Logging Failures | N/A | Personal tool |
| A10: SSRF | No | No network requests |

### Data Privacy

- No telemetry added
- No data sent to external services (in SQLite mode)
- PostgreSQL mode sends data to user's configured server only

## Recommendations Summary

### Must Do (This Project)

1. ✅ Use absolute path resolution
2. ✅ Verify file existence before access
3. ✅ Provide clear error messages
4. ✅ Use sensible defaults (home directory)

### Should Do (Future Work)

1. Consider SecretStorage for PostgreSQL credentials
2. Add settings validation for path format
3. Document security best practices in README

### Won't Do (Out of Scope)

1. Database encryption
2. Path allowlisting
3. Access audit logging
4. Sandboxed file access

## Sign-Off

**Security Review Status**: APPROVED for MVP

**Conditions**:
- All "Must Do" items implemented
- No new dependencies added
- Path handling follows documented patterns

**Reviewed By**: Architecture design phase
**Date**: 2025-11-26
