# VSCDAEMN Security Review

## Executive Summary

**Overall Risk Level**: LOW - Safe to ship with documented mitigations

The VSCode extension daemon migration inherits the security profile from DAEMIGR (daemon-client package), which was already reviewed and deemed LOW risk. No new attack vectors introduced.

## Architecture Security Analysis

### Threat Model

**Attacker Capabilities**:
- Local process access (can read `/proc/<pid>/environ`)
- File system access (can execute binaries in workspace)
- Network access (can connect to localhost PostgreSQL)

**Assets to Protect**:
- Database URL with credentials (MAPROOM_DATABASE_URL)
- API keys for embedding providers (OPENAI_API_KEY, etc.)
- Indexed code data (chunks, embeddings)

### Attack Vectors

#### 1. Environment Variable Exposure (MEDIUM)
**Risk**: Credentials visible in `/proc/<pid>/environ`

**Mitigation**:
- Use VSCode SecretStorage (encrypted at rest)
- Environment variables only passed to daemon process
- Daemon runs with user privileges (no privilege escalation)
- **Action**: Document risk in extension README

#### 2. Binary Integrity (LOW)
**Risk**: Malicious binary executed by extension

**Mitigation**:
- Binary shipped with extension VSIX
- Binary path from hardcoded candidates (not user-configurable)
- File permissions checked (executable, owned by user)
- **Action**: None (current implementation sufficient for MVP)

#### 3. Daemon Crash Denial of Service (LOW)
**Risk**: Attacker crashes daemon repeatedly

**Mitigation**:
- Circuit breaker prevents restart loops (max 5 attempts)
- User can restart manually via command
- Extension continues to function (watch processes independent)
- **Action**: None (auto-restart handles this)

#### 4. PostgreSQL Injection (VERY LOW)
**Risk**: SQL injection via malicious file paths

**Mitigation**:
- Rust binary uses parameterized queries (sqlx)
- File paths validated and sanitized
- No user input directly interpolated into SQL
- **Action**: None (handled by Rust binary)

### Known Gaps and Risk Evaluation

| Gap | Risk Level | MVP Mitigation | Enterprise Solution |
|-----|-----------|----------------|---------------------|
| Credentials in env vars | MEDIUM | VSCode SecretStorage | Platform secrets (Keychain, Credential Manager) |
| Binary unsigned | LOW | VSIX packaging | Code signing with certificate |
| No audit logging | LOW | Output channel logging | Structured audit log with rotation |
| Local-only PostgreSQL | NONE | Docker container | No change needed |

## MVP-Appropriate Mitigations

### Ship Without Addressing
- ❌ Binary signature verification - Not critical for internal extension
- ❌ Platform-specific secrets - SecretStorage sufficient for MVP
- ❌ Audit logging - Output channel provides debugging capability
- ❌ Memory limits - VSCode extension host manages memory

### Document for Users
- ✅ Environment variable credential exposure risk
- ✅ Recommended: Use VSCode SecretStorage (not .env files)
- ✅ PostgreSQL connection should be localhost-only
- ✅ Disable extension if untrusted workspace

### Implement Now
- ✅ VSCode SecretStorage for credentials (already implemented)
- ✅ Binary path from hardcoded candidates (already implemented)
- ✅ Circuit breaker for daemon restart (daemon-client handles)
- ✅ Graceful degradation on daemon failure (fallback to error message)

## Compliance Considerations

**Data Residency**: Code data stored in PostgreSQL (user-controlled location)
- MVP: Local Docker container (no data leaves machine)
- Enterprise: User can configure remote PostgreSQL (compliance responsibility shifts to user)

**Credential Storage**: VSCode SecretStorage (encrypted at rest)
- MVP: Platform-specific encryption (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux)
- Enterprise: Same as MVP (VSCode handles encryption)

**Access Control**: File system permissions
- MVP: Daemon runs with user privileges (same as extension)
- Enterprise: No additional access control needed (user owns workspace)

## Incident Response

### Daemon Crash
**Detection**: Auto-restart fails after 5 attempts
**Response**: 
1. Show error notification to user
2. Log error to output channel
3. Provide command to restart manually
4. Recommend checking PostgreSQL availability

### Credential Leak
**Detection**: User reports credentials compromised
**Response**:
1. Rotate credentials immediately
2. Update DATABASE_URL in SecretStorage
3. Restart daemon with new credentials
4. No code changes needed

### Malicious Binary
**Detection**: Antivirus flags binary
**Response**:
1. Verify binary checksum (future enhancement)
2. Re-download extension from marketplace
3. Report issue to extension developer

## Security Testing

**Unit Tests**:
- Daemon starts with env vars correctly
- Credentials not logged to output channel
- Binary path validated before execution

**Integration Tests**:
- PostgreSQL credentials work end-to-end
- Daemon crashes don't expose credentials
- Circuit breaker prevents restart loops

**Manual Tests**:
- Verify credentials in SecretStorage (not plaintext)
- Check `/proc/<pid>/environ` (credentials visible but expected)
- Test with untrusted workspace (extension can be disabled)

## Conclusion

The VSCode extension daemon migration **introduces no new security risks** and inherits the LOW risk profile from DAEMIGR. MVP mitigations (SecretStorage, circuit breaker, binary validation) are sufficient for safe deployment.

**Recommendation**: Ship with current security mitigations, document credential exposure risk in README.
