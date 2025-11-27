# Security Review: Multi-Agent Concurrency

## Scope

This review covers the security implications of:
1. Unix socket server for daemon communication
2. Multi-client access to shared daemon
3. Process lifecycle management (PID files, signals)

## Architecture Security Analysis

### Unix Socket Security

**Current approach**: Unix domain socket at `/tmp/maproom-{uid}.sock`

**Security properties**:
- **File permissions**: Socket created with user-only access (mode 0600)
- **UID isolation**: Each user gets their own socket
- **No network exposure**: Unix sockets are local-only

**Assessment**: ✅ Appropriate for single-user workstation use

```rust
// Recommended implementation
let listener = UnixListener::bind(&socket_path)?;
std::fs::set_permissions(&socket_path, Permissions::from_mode(0o600))?;
```

### PID File Security

**Current approach**: `/tmp/maproom-{uid}.pid` with flock

**Risks**:
- **Symlink attack**: Attacker could symlink PID file to sensitive location
- **Race condition**: Between check and creation

**Mitigations**:
```rust
// Use O_EXCL to prevent symlink attacks
let file = OpenOptions::new()
    .write(true)
    .create_new(true)  // Fails if file exists (including symlinks)
    .open(&pid_path)?;

// Exclusive lock
flock(file.as_raw_fd(), LOCK_EX | LOCK_NB)?;
```

**Assessment**: ✅ With O_EXCL, acceptable security posture

### Request Validation

**Risk**: Malicious client sends crafted JSON-RPC requests

**Analysis**:
- All requests are JSON-RPC 2.0 with defined schema
- Methods limited to: `ping`, `search`, `upsert`, `status`
- No arbitrary code execution paths
- SQLite queries use parameterized statements (no SQL injection)

**Assessment**: ✅ Existing input validation sufficient

### Multi-Client Isolation

**Risk**: One client affects another's requests/responses

**Analysis**:
- Each client has unique session ID (UUID v4)
- Request IDs are per-client namespaced
- Response routing based on session + request ID
- No shared mutable state between client handlers (except database)

**Assessment**: ✅ Session isolation adequate

## Known Gaps

### 1. No Authentication

**Gap**: Any process running as the user can connect to the socket

**Risk level**: Low - this matches current stdio model where any process can spawn daemon

**Acceptable for MVP**: Yes - maproom is a developer tool on personal workstations

**Future consideration**: For shared server deployments, consider:
- Token-based auth via environment variable
- mTLS for network deployments

### 2. No Rate Limiting

**Gap**: A client could flood the daemon with requests

**Risk level**: Low - self-DoS on personal workstation

**Acceptable for MVP**: Yes

**Future consideration**: Semaphore-based per-client limits

### 3. No Request Size Limits

**Gap**: Large JSON payloads could exhaust memory

**Risk level**: Low - length-prefixed protocol allows size checking

**Mitigation** (easy to add):
```rust
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB

fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
    let length = u32::from_be_bytes(src[..4].try_into()?) as usize;
    if length > MAX_MESSAGE_SIZE {
        return Err(Error::MessageTooLarge);
    }
    // ...
}
```

**Assessment**: Add this check - minimal effort, good defense

### 4. Socket Path in /tmp

**Gap**: `/tmp` is world-writable, potential for confusion attacks

**Risk level**: Low - socket name includes UID, socket permissions are 0600

**Alternative considered**: `~/.maproom/maproom.sock`
- Pro: More isolated directory
- Con: Requires HOME access, may not exist in all environments

**Decision**: Keep `/tmp` for MVP, document alternative in config

## MVP-Appropriate Mitigations

### Implemented in This Project

| Mitigation | Effort | Risk Addressed |
|------------|--------|----------------|
| Socket mode 0600 | Low | Unauthorized access |
| PID file O_EXCL | Low | Symlink attacks |
| Message size limit | Low | Memory exhaustion |
| Session UUID isolation | Medium | Cross-client interference |

### Deferred to Future

| Mitigation | Reason for Deferral |
|------------|---------------------|
| Authentication | Not needed for single-user workstation |
| Rate limiting | Self-DoS only, low priority |
| Audit logging | Operational, not security-critical |
| Encrypted socket | Overkill for localhost |

## Enterprise Considerations (Not Implemented)

For future multi-user or server deployments:

1. **Authentication**: JWT or API key per client
2. **Authorization**: Per-repo access control
3. **Audit trail**: Request logging with client identity
4. **Network isolation**: TLS for non-localhost
5. **Resource quotas**: Per-user memory/CPU limits

These are mentioned for completeness but explicitly out of scope for this MVP.

## Security Checklist

### Pre-Implementation

- [x] Socket permissions defined (0600)
- [x] PID file creation pattern (O_EXCL + flock)
- [x] Message size limit specified (10MB)
- [x] Session isolation design reviewed

### Implementation Verification

- [ ] Socket created with correct permissions
- [ ] PID file uses O_EXCL
- [ ] Message size enforced in codec
- [ ] No SQL injection in new code paths
- [ ] Signals handled safely (no SIGKILL data loss)

### Post-Implementation

- [ ] Security-focused code review
- [ ] Test with malformed JSON-RPC messages
- [ ] Verify socket permissions on macOS and Linux

## Conclusion

The shared daemon architecture introduces minimal new attack surface:

- Unix socket with 0600 permissions provides adequate isolation
- Existing JSON-RPC validation continues to apply
- PID file handling with O_EXCL prevents symlink attacks
- Message size limits prevent memory exhaustion

**Security posture**: Acceptable for MVP release on single-user workstations. Enterprise hardening would be a separate future project if multi-user deployments become a requirement.
