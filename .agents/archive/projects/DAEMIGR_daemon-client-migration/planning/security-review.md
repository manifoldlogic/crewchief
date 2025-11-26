# DAEMIGR Security Review

## Executive Summary

**Overall Risk Level:** LOW

The DAEMIGR architecture uses local process communication over stdin/stdout, avoiding network exposure and most common attack vectors. The primary security concerns are process isolation, credential management, and resource exhaustion. This review identifies gaps and provides pragmatic mitigations suitable for an MVP.

**Key Findings:**
- ✅ No network exposure (stdin/stdout IPC)
- ✅ Process isolation (per-client daemon)
- ⚠️ Environment variable credential exposure (`/proc/<pid>/environ`)
- ⚠️ No resource limits (memory, CPU unbounded)
- ⚠️ Potential command injection (if binary path user-controlled)

**Ship Decision:** ✅ SAFE TO SHIP with documented mitigations

## Threat Model

### Assets

**What are we protecting?**

1. **Database Credentials**
   - Asset: PostgreSQL connection string (`MAPROOM_DATABASE_URL`)
   - Value: Access to indexed code data
   - Exposure: Environment variables, process memory

2. **API Keys (Embedding Providers)**
   - Asset: OpenAI/Anthropic API keys
   - Value: Paid API access, rate limits
   - Exposure: Environment variables, network traffic

3. **Indexed Code Data**
   - Asset: Source code chunks, embeddings, metadata
   - Value: Proprietary code, intellectual property
   - Exposure: Database, search results

4. **Host System Resources**
   - Asset: CPU, memory, disk, file descriptors
   - Value: System stability, performance
   - Exposure: Daemon process, connection pool

### Adversaries

**Who might attack?**

1. **Malicious MCP Client** (Likelihood: LOW, Impact: MEDIUM)
   - Goal: Exfiltrate code data via search
   - Capability: Craft malicious search queries
   - Access: MCP protocol (authenticated or not)

2. **Local Attacker** (Likelihood: LOW, Impact: HIGH)
   - Goal: Steal credentials, access database
   - Capability: Read `/proc/<pid>/environ`, attach debugger
   - Access: Local machine access, same user

3. **Compromised Dependency** (Likelihood: LOW, Impact: HIGH)
   - Goal: Backdoor, credential theft
   - Capability: Arbitrary code execution in Node.js/Rust
   - Access: Supply chain attack

4. **Resource Exhaustion Attacker** (Likelihood: MEDIUM, Impact: LOW)
   - Goal: Crash daemon, degrade performance
   - Capability: Send many/large requests
   - Access: MCP protocol

### Attack Vectors

#### Vector 1: Environment Variable Exposure
**Scenario:** Local attacker reads daemon environment variables

**Attack Steps:**
1. Identify daemon process ID: `ps aux | grep crewchief-maproom`
2. Read environment: `cat /proc/<pid>/environ`
3. Extract credentials: `MAPROOM_DATABASE_URL`, `OPENAI_API_KEY`

**Impact:** HIGH (full credential disclosure)

**Likelihood:** LOW (requires local access, same user or root)

**Mitigation:**
- **Current:** Standard practice (environment variables)
- **MVP Mitigation:** Document risk, recommend secrets management for production
- **Future Enhancement:** Use platform-specific secrets (Keychain, Secret Service)

#### Vector 2: Command Injection via Binary Path
**Scenario:** Attacker controls binary path configuration

**Attack Steps:**
1. Modify config to point to malicious binary: `/tmp/evil-binary`
2. Trigger daemon start
3. Execute arbitrary code in place of daemon

**Impact:** CRITICAL (arbitrary code execution)

**Likelihood:** VERY LOW (requires config write access)

**Mitigation:**
- ✅ **Implemented:** Binary path from hardcoded candidates (not user input)
- ✅ **Implemented:** Binary discovery logic validates existence
- ⚠️ **MVP Gap:** No signature verification
- **Future Enhancement:** Sign binaries, verify signatures

#### Vector 3: Resource Exhaustion (Memory)
**Scenario:** Attacker sends many/large requests to exhaust memory

**Attack Steps:**
1. Send 1000 concurrent search requests
2. Daemon allocates memory for each request
3. System OOM kills daemon (or other processes)

**Impact:** MEDIUM (denial of service, data loss)

**Likelihood:** MEDIUM (easy to trigger if MCP exposed)

**Mitigation:**
- ⚠️ **MVP Gap:** No memory limits on daemon
- ⚠️ **MVP Gap:** No request queue backpressure
- **MVP Mitigation:** Document resource considerations
- **Future Enhancement:** Memory limits (cgroups, ulimit), request queue limits

#### Vector 4: Resource Exhaustion (Database Connections)
**Scenario:** Attacker exhausts database connection pool

**Attack Steps:**
1. Send many concurrent requests (> pool size)
2. Daemon blocks waiting for available connection
3. Other clients/services starved

**Impact:** MEDIUM (denial of service for search)

**Likelihood:** MEDIUM (easy to trigger)

**Mitigation:**
- ✅ **Implemented:** Connection pool with max size (default 5)
- ⚠️ **MVP Gap:** No timeout on pool acquisition
- ⚠️ **MVP Gap:** No connection pool monitoring
- **Future Enhancement:** Pool timeout, connection metrics

#### Vector 5: JSON-RPC Protocol Injection
**Scenario:** Attacker sends crafted JSON-RPC to exploit parsing

**Attack Steps:**
1. Send malformed JSON-RPC (injection payloads)
2. Exploit parser vulnerabilities (buffer overflow, etc.)
3. Achieve code execution or denial of service

**Impact:** HIGH (code execution) or MEDIUM (DoS)

**Likelihood:** VERY LOW (Rust parser robust, JSON spec strict)

**Mitigation:**
- ✅ **Implemented:** Strict JSON-RPC 2.0 validation
- ✅ **Implemented:** Rust memory safety (no buffer overflows)
- ✅ **Implemented:** Error handling (malformed input rejected)

#### Vector 6: SQL Injection via Search Query
**Scenario:** Attacker crafts search query to inject SQL

**Attack Steps:**
1. Send search query: `'; DROP TABLE chunks; --`
2. Daemon constructs SQL query with unsanitized input
3. Execute arbitrary SQL

**Impact:** CRITICAL (data loss, exfiltration)

**Likelihood:** VERY LOW (Rust uses parameterized queries)

**Mitigation:**
- ✅ **Implemented:** Parameterized queries in Rust (sqlx)
- ✅ **Implemented:** No string interpolation in SQL
- ✅ **Implemented:** Database user has limited permissions

#### Vector 7: Process Crash via Malformed Request
**Scenario:** Attacker sends malformed request to crash daemon

**Attack Steps:**
1. Send invalid JSON-RPC (missing fields, wrong types)
2. Daemon panics or crashes
3. Daemon restarts (DoS via restart loop)

**Impact:** LOW (auto-restart mitigates)

**Likelihood:** MEDIUM (easy to test)

**Mitigation:**
- ✅ **Implemented:** Strict input validation (Rust type system)
- ✅ **Implemented:** Auto-restart with exponential backoff
- ✅ **Implemented:** Circuit breaker (max 5 restarts)

## Architecture Security Analysis

### Component: DaemonClient (TypeScript)

**Security Characteristics:**

**Strengths:**
- ✅ No network exposure (local IPC only)
- ✅ Process isolation (daemon per client)
- ✅ Typed errors (no silent failures)

**Weaknesses:**
- ⚠️ Environment variables visible to process tree
- ⚠️ No authentication between client and daemon (not needed for local IPC)
- ⚠️ No encryption of IPC (not needed for stdin/stdout)

**Attack Surface:**
- Binary path configuration (low risk if hardcoded)
- Environment variables (credential exposure)
- Request timeout (potential DoS if too low/high)

**Mitigations:**
- Binary path from hardcoded candidates (not user input)
- Environment variables documented as sensitive
- Timeout configurable (default 30s reasonable)

### Component: Daemon (Rust - `crewchief-maproom serve`)

**Security Characteristics:**

**Strengths:**
- ✅ Memory safety (Rust prevents buffer overflows, use-after-free)
- ✅ Parameterized queries (no SQL injection)
- ✅ Strict input validation (type system enforces)
- ✅ Connection pooling (prevents connection exhaustion)

**Weaknesses:**
- ⚠️ No authentication (assumes trusted client)
- ⚠️ No authorization (client can search any repo/worktree)
- ⚠️ No rate limiting (client can spam requests)
- ⚠️ No resource limits (memory, CPU unbounded)

**Attack Surface:**
- JSON-RPC stdin (malformed input)
- Database connection (SQL injection, connection exhaustion)
- Embedding API (API key exposure, rate limits)

**Mitigations:**
- Strict JSON-RPC parsing (reject malformed)
- Parameterized queries (SQLx prevents injection)
- Connection pool (limits DB connections)
- API key from environment (standard practice)

### Component: PostgreSQL Database

**Security Characteristics:**

**Strengths:**
- ✅ Network isolation (localhost only)
- ✅ User authentication (password-protected)
- ✅ Limited permissions (daemon user can't drop schema)

**Weaknesses:**
- ⚠️ Credentials in environment variables
- ⚠️ No connection encryption (not needed for localhost)
- ⚠️ No audit logging (can't track who accessed what)

**Attack Surface:**
- Connection string (credential exposure)
- SQL queries (injection, though mitigated)
- Schema access (limited by user permissions)

**Mitigations:**
- Credentials from environment (standard practice)
- Localhost-only (no network exposure)
- Limited user permissions (can't drop schema)

## Known Gaps and Risks

### Gap 1: Credential Exposure via `/proc/<pid>/environ`

**Description:**
Environment variables (including credentials) are visible to any process with access to `/proc/<pid>/environ`.

**Risk Level:** MEDIUM
- **Impact:** HIGH (credential disclosure)
- **Likelihood:** LOW (requires local access, same user or root)

**MVP Mitigation:**
- Document risk in README
- Recommend secrets management for production (Vault, AWS Secrets Manager)
- Note: Standard practice for CLI tools (npm, git, etc.)

**Future Enhancement:**
- Use platform-specific secrets (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Daemon accepts credentials via stdin (not environment)

### Gap 2: No Resource Limits

**Description:**
Daemon has no memory or CPU limits. Malicious/buggy client could exhaust resources.

**Risk Level:** MEDIUM
- **Impact:** MEDIUM (DoS, system instability)
- **Likelihood:** MEDIUM (easy to trigger)

**MVP Mitigation:**
- Document resource considerations
- Recommend monitoring (memory usage, restart rate)
- Note: MCP server deployment typically isolated (Docker, systemd)

**Future Enhancement:**
- Memory limits via cgroups (Docker) or ulimit
- Request queue size limits (backpressure)
- Circuit breaker on resource exhaustion

### Gap 3: No Authentication Between Client and Daemon

**Description:**
Daemon trusts all input from stdin. If process isolation broken, attacker could send RPC.

**Risk Level:** LOW
- **Impact:** MEDIUM (unauthorized search)
- **Likelihood:** VERY LOW (requires process attachment)

**MVP Mitigation:**
- Process isolation (daemon owned by client)
- stdin/stdout not accessible outside process tree
- Note: Same trust model as LSP servers

**Future Enhancement:**
- Shared secret authentication (if shared daemon)
- TLS over stdio (complex, likely overkill)

### Gap 4: No Binary Signature Verification

**Description:**
DaemonClient doesn't verify binary signature. Attacker could replace binary.

**Risk Level:** MEDIUM
- **Impact:** CRITICAL (arbitrary code execution)
- **Likelihood:** LOW (requires write access to binary path)

**MVP Mitigation:**
- Binary path from hardcoded candidates (not user writable)
- File permissions (755, owned by root or installer)
- Note: Standard practice for CLI tools (not unique to DAEMIGR)

**Future Enhancement:**
- Sign binaries (code signing certificate)
- Verify signature before spawn
- Checksum validation (hash against known-good)

### Gap 5: No Audit Logging

**Description:**
No record of who searched what, when. Forensics impossible.

**Risk Level:** LOW
- **Impact:** LOW (no compliance/forensics)
- **Likelihood:** N/A (not an attack, but observability gap)

**MVP Mitigation:**
- Structured logging (JSON logs with repo, query, timestamp)
- Optional log aggregation (send to SIEM)
- Note: MVP focused on functionality, not compliance

**Future Enhancement:**
- Audit log mode (detailed query logs)
- Integration with security tools (Splunk, ELK)
- Compliance support (GDPR, HIPAA if needed)

## Mitigation Strategy

### Ship Blockers (MUST FIX)

**None** - No security issues block MVP ship.

### MVP Mitigations (DOCUMENT)

1. **Credential Management**
   - Document environment variable exposure risk
   - Recommend secrets management for production
   - Example: AWS Secrets Manager, HashiCorp Vault

2. **Resource Limits**
   - Document memory/CPU considerations
   - Recommend deployment in isolated environment (Docker, systemd)
   - Recommend monitoring (memory usage, restart rate)

3. **Binary Integrity**
   - Document binary discovery logic
   - Recommend file permissions (755, root-owned)
   - Note: No signature verification in MVP

4. **Access Control**
   - Document trust model (daemon trusts client)
   - Note: No authentication between client and daemon
   - Recommend network isolation (MCP server firewalled)

### Post-MVP Enhancements

**Priority 1: High Impact, Medium Effort**
1. **Memory Limits** (cgroups, ulimit)
2. **Request Queue Limits** (backpressure)
3. **Connection Pool Timeout** (prevent indefinite blocking)

**Priority 2: Medium Impact, High Effort**
1. **Binary Signature Verification**
2. **Platform-Specific Secrets** (Keychain, Credential Manager)
3. **Audit Logging** (structured logs, SIEM integration)

**Priority 3: Low Impact, High Effort**
1. **Authentication** (shared secret, if shared daemon)
2. **Encryption** (TLS over stdio, likely overkill)
3. **Compliance Support** (GDPR, HIPAA)

## Security Best Practices

### For Developers

**When Contributing:**
1. ✅ Never log credentials (DATABASE_URL, API keys)
2. ✅ Use parameterized queries (never string interpolation)
3. ✅ Validate all input (Zod schemas, Rust types)
4. ✅ Handle errors gracefully (no panic in production code)
5. ✅ Review dependencies (npm audit, cargo audit)

### For Deployers

**When Deploying MCP Server:**
1. ✅ Use environment variables for credentials (not hardcoded)
2. ✅ Restrict file permissions on binaries (755, root-owned)
3. ✅ Deploy in isolated environment (Docker, systemd)
4. ✅ Monitor resource usage (memory, CPU, restart rate)
5. ✅ Use secrets management (Vault, AWS Secrets Manager)

**When Deploying VSCode Extension:**
1. ✅ Credentials from VSCode secrets API (not user settings)
2. ✅ Restrict daemon process permissions (same user)
3. ✅ Monitor daemon restarts (warn user if frequent)

## Compliance Considerations

### Data Privacy (GDPR, CCPA)

**Data Collected:**
- Source code chunks (indexed by user)
- Search queries (logged if enabled)
- Embeddings (derived from code)

**Data Retention:**
- Persistent (until user deletes repository)
- Logs: 30 days (configurable)

**User Rights:**
- Right to delete (drop repository from database)
- Right to access (query search history if logged)

**MVP Stance:**
- No PII collected (source code is user-owned)
- No cross-user data sharing
- User controls data (can delete anytime)

### Secrets Management (PCI-DSS, SOC 2)

**Secrets Stored:**
- Database credentials
- API keys (OpenAI, Anthropic)

**Storage Method:**
- Environment variables (standard practice)
- Not encrypted at rest (MVP limitation)

**MVP Stance:**
- Document environment variable risk
- Recommend secrets management for production
- No PCI-DSS compliance (not handling payment data)

## Incident Response

### Credential Leak

**Scenario:** Database credentials exposed

**Response:**
1. Rotate credentials immediately
2. Update environment variables
3. Restart daemon
4. Audit database access logs (if enabled)
5. Revoke old credentials

**Prevention:**
- Use secrets management (Vault, AWS Secrets Manager)
- Rotate credentials regularly (30 days)
- Monitor for unusual access patterns

### Daemon Compromise

**Scenario:** Daemon binary replaced by attacker

**Response:**
1. Stop daemon immediately
2. Verify binary integrity (checksum)
3. Restore from known-good source
4. Restart daemon
5. Investigate how attacker gained write access

**Prevention:**
- File permissions (755, root-owned)
- Signature verification (future enhancement)
- Immutable infrastructure (Docker, read-only filesystem)

### Denial of Service

**Scenario:** Attacker exhausts resources (memory, connections)

**Response:**
1. Identify attack source (MCP client, IP)
2. Kill daemon (free resources)
3. Implement rate limiting (future enhancement)
4. Restart daemon
5. Monitor resource usage

**Prevention:**
- Resource limits (cgroups, ulimit)
- Request queue limits (backpressure)
- Circuit breaker (max restarts)

## Conclusion

**Security Posture:** ACCEPTABLE FOR MVP

**Key Strengths:**
- ✅ No network exposure (stdin/stdout IPC)
- ✅ Process isolation (per-client daemon)
- ✅ Memory safety (Rust prevents common vulnerabilities)
- ✅ Parameterized queries (no SQL injection)

**Accepted Risks:**
- ⚠️ Environment variable credential exposure (document, recommend secrets management)
- ⚠️ No resource limits (document, recommend monitoring)
- ⚠️ No binary signature verification (future enhancement)

**Ship Decision:** ✅ **SAFE TO SHIP**

**Conditions:**
1. Document security considerations in README
2. Recommend secrets management for production
3. Recommend deployment in isolated environment
4. Provide incident response guide

**Post-MVP Roadmap:**
1. Memory limits and request queue backpressure
2. Binary signature verification
3. Platform-specific secrets (Keychain, Credential Manager)
4. Audit logging (if compliance needed)

---

**Security Review Complete:** 2025-11-22
**Reviewer:** Claude (AI Assistant)
**Status:** APPROVED FOR MVP SHIP
