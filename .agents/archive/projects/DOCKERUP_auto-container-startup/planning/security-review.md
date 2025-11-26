# Security Review: Auto Container Startup Integration

## Threat Model

### Trust Boundaries

```
┌────────────────────────────────────────┐
│         User's Machine (Trusted)       │
│                                        │
│  ┌──────────────────────────────────┐ │
│  │  VSCode Extension Process        │ │
│  │  - Runs with user privileges     │ │
│  │  - Access to workspace files     │ │
│  │  - Can spawn processes           │ │
│  └──────────────────────────────────┘ │
│             │                          │
│             │ spawns                   │
│             ↓                          │
│  ┌──────────────────────────────────┐ │
│  │  Docker Desktop                  │ │
│  │  - Runs with elevated privileges │ │
│  │  - Controls containers           │ │
│  │  - Manages network/volumes       │ │
│  └──────────────────────────────────┘ │
│             │                          │
│             │ creates                  │
│             ↓                          │
│  ┌──────────────────────────────────┐ │
│  │  PostgreSQL Container            │ │
│  │  - Isolated namespace            │ │
│  │  - Bound to localhost only       │ │
│  │  - Default credentials           │ │
│  └──────────────────────────────────┘ │
│                                        │
└────────────────────────────────────────┘
          ↑
          │ no external access
          └─ Network boundary
```

**Key Points**:
- Extension runs with user privileges (not elevated)
- Docker socket accessed via Docker Desktop (pre-authorized)
- Containers isolated via Docker namespaces
- No external network access required

### Attack Surface

**1. Command Injection** (Shell commands)
**2. Path Traversal** (File operations)
**3. Credential Exposure** (Environment variables)
**4. Container Escape** (Docker vulnerabilities)
**5. Port Conflicts** (Denial of service)

## Security Analysis

### 1. Command Injection

**Risk**: Shell command injection via unsanitized input

**Vulnerable Code Patterns**:
```typescript
// BAD: User input in command
exec(`docker compose -f ${userInput}`)

// BAD: Template literals with external data
spawn('sh', ['-c', `docker exec ${containerName} pg_isready`])
```

**Our Implementation** (DockerManager, line 45-60):
```typescript
// GOOD: No user input in commands
spawn('docker', ['compose', 'up', '-d'])

// GOOD: Fixed arguments
spawn('docker', ['exec', 'maproom-postgres', 'pg_isready'])
```

**Verdict**: ✅ Safe
- No user input in Docker commands
- All arguments are hardcoded strings
- No shell interpretation (`spawn` not `exec`)

**Mitigations**:
- Use `spawn()` not `exec()` (no shell parsing)
- Fixed argument arrays (no string interpolation)
- No runtime command construction

### 2. Path Traversal

**Risk**: File operations outside workspace boundary

**Vulnerable Code Patterns**:
```typescript
// BAD: User-controlled path
const configPath = path.join(workspace, userInput, 'mcp.json')

// BAD: No validation
fs.writeFile(`${workspace}/../../../etc/passwd`, data)
```

**Our Implementation** (extension.ts, new code):
```typescript
// GOOD: Fixed path components
const dockerManager = new DockerManager(outputChannel)
// DockerManager uses: path.join(extensionRoot, 'config', 'docker-compose.yml')

// GOOD: No user-controlled paths in Docker startup
```

**Existing Protection** (MCPConfigWriter, already reviewed):
```typescript
// From MCPINIT-1001 security review:
validatePath(configPath, resolvedWorkspace)
// Ensures path is within workspace root
```

**Verdict**: ✅ Safe
- No user-controlled path components in new code
- Docker Compose file path is hardcoded (bundled with extension)
- Existing path validation in MCP config writer (unchanged)

**Mitigations**:
- Fixed paths for Docker Compose file
- No runtime path construction from user input
- Reuse existing `validatePath()` for any future file operations

### 3. Credential Exposure

**Risk**: Sensitive data exposed in logs, environment, or config files

**Attack Vectors**:
- API keys logged to Output panel
- Database password in plaintext config
- Environment variables leaked to child processes

**Our Implementation**:

**Database Credentials** (docker-compose.yml):
```yaml
environment:
  POSTGRES_USER: maproom
  POSTGRES_PASSWORD: maproom  # ⚠️ Hardcoded
```

**Risk Assessment**: Low
- Local development only (bound to localhost)
- Default credentials documented
- No external network access

**Embedding Provider Credentials** (ProcessOrchestrator, fixed in commit 58ed3ba6):
```typescript
// GOOD: Uses SecretStorage, not plaintext
const credentialEnv = await secretsManager.getEnvironmentVars(provider)

// GOOD: Logs keys but not values
this.log(`Embedding credentials configured: ${credentialKeys.join(', ')}`)
// Output: "Embedding credentials configured: MAPROOM_OPENAI_API_KEY"
// NOT: "Embedding credentials configured: MAPROOM_OPENAI_API_KEY=sk-proj-..."
```

**Verdict**: ✅ Acceptable for MVP
- Database: Default password acceptable (localhost only, documented risk)
- API keys: Properly secured in SecretStorage
- Logging: Sanitized (keys logged, values masked)

**Future Improvements** (Post-MVP):
- Allow custom PostgreSQL password via settings
- Rotate default credentials on each extension install
- Add warning if using default password in production

### 4. Container Escape

**Risk**: Attacker escapes container to access host system

**Attack Prerequisites**:
1. Attacker must compromise PostgreSQL container
2. Container must have privileged access or kernel vulnerabilities

**Our Configuration** (docker-compose.yml):
```yaml
# GOOD: No privileged mode
# GOOD: No host network mode
# GOOD: No volume mounts to sensitive host paths

services:
  maproom-postgres:
    image: pgvector/pgvector:pg16  # Official image, regularly updated
    # No: privileged: true
    # No: network_mode: "host"
    volumes:
      - maproom-postgres-data:/var/lib/postgresql/data  # Named volume, not host path
```

**Risk Assessment**: Low
- Standard Docker isolation (namespaces, cgroups)
- No privileged escalation paths
- Official base image (pgvector maintained by pgvector team)
- Named volumes (not bind mounts to sensitive paths)

**Verdict**: ✅ Safe
- Containers run with minimal privileges
- No intentional escape routes
- Relies on Docker Desktop security model

**Mitigations**:
- Use official, maintained images
- No custom privileged configurations
- Document requirement: Docker Desktop must be from trusted source

### 5. Port Conflicts (Denial of Service)

**Risk**: Malicious process occupies port 5432/3000 before extension

**Attack Scenario**:
1. Attacker starts rogue PostgreSQL on port 5432
2. Extension connects to attacker's database
3. Attacker intercepts/modifies semantic search data

**Our Implementation** (DockerManager, error handling):
```typescript
try {
  await spawn('docker', ['compose', 'up', '-d'])
} catch (error) {
  // Error propagated to user
  throw new Error(`Failed to start Docker services: ${error.message}`)
}
```

**Risk Assessment**: Low
- Port conflict causes Docker Compose to fail (not connect to rogue service)
- Error shown to user (not silent)
- User must manually resolve conflict

**Verdict**: ✅ Safe
- Fail-secure: Extension errors if port unavailable
- No automatic fallback to unknown port
- User alerted to conflict

**Mitigations**:
- Clear error message mentions port conflict
- Documentation: How to check/resolve port conflicts
- Future: Health check validates connecting to correct service (schema version check)

## Architecture Security

### Defense in Depth

**Layer 1: Process Isolation**
- Extension runs in sandboxed VSCode extension host
- Limited filesystem access (workspace only)
- No elevated privileges

**Layer 2: Docker Isolation**
- Containers isolated via namespaces (network, PID, mount)
- Resource limits enforced by cgroups
- Official images with regular security updates

**Layer 3: Network Isolation**
- PostgreSQL bound to localhost only (not 0.0.0.0)
- MCP server bound to localhost only
- No external network access required

**Layer 4: Credential Management**
- API keys in VSCode SecretStorage (encrypted)
- Database password: Default (documented limitation)
- No plaintext credentials in config files

### Known Security Gaps (Accepted for MVP)

#### 1. Default PostgreSQL Password

**Issue**: `POSTGRES_PASSWORD=maproom` is hardcoded and public

**Risk**: Low (localhost only, local attacker already has full access)

**Acceptance**: ✅ Acceptable for MVP
- PostgreSQL bound to 127.0.0.1 (not externally accessible)
- Local attacker already has access to workspace files (more sensitive)
- Documented in README security section

**Future Mitigation**:
- Generate random password on first run
- Store in VSCode settings (encrypted)
- Migrate existing users with schema version check

#### 2. No TLS for PostgreSQL Connection

**Issue**: Extension connects to PostgreSQL over unencrypted localhost connection

**Risk**: Minimal (localhost traffic)

**Acceptance**: ✅ Acceptable for MVP
- Localhost traffic not typically sniffed
- Would require root/admin privileges (game over anyway)
- Performance overhead not justified for local development

**Future Mitigation**:
- Add `sslmode=require` for remote PostgreSQL (future feature)
- Document requirement for production deployments

#### 3. Docker Socket Access

**Issue**: Extension spawns Docker commands, which access Docker socket

**Risk**: Low (user already authorized Docker Desktop)

**Acceptance**: ✅ Acceptable (standard pattern)
- Same security model as official Docker extension
- User explicitly installed Docker Desktop
- No more privileged than `docker compose` CLI

**Mitigations**:
- Document Docker Desktop requirement in README
- No additional privileges requested beyond standard Docker access

## Compliance & Privacy

### Data Collection

**What We Store**:
- Workspace file paths (in PostgreSQL)
- Embedding vectors (derived from code)
- Provider selection (OpenAI, Google, Ollama)
- API keys (in VSCode SecretStorage)

**What We DON'T Store**:
- User identity (no auth)
- Telemetry (no analytics)
- Network requests (no phone-home)

**Data Residency**:
- PostgreSQL: Local Docker container, volume on user's machine
- SecretStorage: VSCode encrypted storage, local filesystem
- No cloud storage, no external transmission (except embedding provider APIs)

### Third-Party Dependencies

**Embedding Providers** (User Choice):
1. **OpenAI API** (if user selects OpenAI)
   - Transmits: Code snippets for embedding generation
   - User responsibility: API key, terms of service
   - Extension role: Passes data, doesn't store/log

2. **Google Vertex AI** (if user selects Google)
   - Transmits: Code snippets for embedding generation
   - User responsibility: Google Cloud project, credentials
   - Extension role: Passes data, doesn't store/log

3. **Ollama** (if user selects Ollama)
   - Local inference (no external transmission)
   - No third-party dependency
   - Extension role: Sends code to local Ollama API

**Verdict**: ✅ Transparent
- User explicitly selects provider
- Extension doesn't hide data transmission
- Clear in setup wizard which provider sends data externally

### GDPR Considerations (If Applicable)

**Data Subject Rights**:
- **Right to erasure**: User deletes Docker volume, workspace data gone
- **Right to data portability**: PostgreSQL dump provides export
- **Right to be forgotten**: No cloud storage, deletion is local

**Legal Basis**: Not applicable (local software, no data processing service)

## Incident Response

### If Security Issue Discovered

**1. Severity Assessment**:
- **Critical**: Remote code execution, credential theft
- **High**: Local privilege escalation, data exfiltration
- **Medium**: Denial of service, information disclosure
- **Low**: Cosmetic issues, non-exploitable bugs

**2. Response Timeline**:
- **Critical/High**: Patch within 24-48 hours, publish security advisory
- **Medium**: Patch within 1 week, mention in release notes
- **Low**: Fix in next regular release

**3. Disclosure Policy**:
- Private disclosure: Email maintainer directly
- Coordinated disclosure: 90 days before public announcement
- Public disclosure: GitHub Security Advisory

**4. User Notification**:
- Extension update with clear changelog
- GitHub release notes with severity/impact
- README update with migration instructions

## Security Checklist (Pre-Release)

**Code Review**:
- [ ] No user input in shell commands
- [ ] All file paths validated against workspace root
- [ ] No plaintext credentials in config files
- [ ] Environment variables sanitized in logs
- [ ] No `eval()` or `exec()` with external data

**Configuration Review**:
- [ ] Docker Compose: No privileged containers
- [ ] Docker Compose: No host network mode
- [ ] Docker Compose: Ports bound to localhost only
- [ ] Docker Compose: Official base images used

**Dependency Review**:
- [ ] All npm packages from trusted sources
- [ ] Docker images from official repositories
- [ ] No deprecated/unmaintained dependencies

**Documentation Review**:
- [ ] Security considerations in README
- [ ] Docker Desktop requirement clearly stated
- [ ] Credential storage model explained
- [ ] Known limitations documented

## Summary

### Security Posture

**Strengths**:
- ✅ No command injection vectors
- ✅ Path traversal protections (existing)
- ✅ Credential storage via SecretStorage
- ✅ Container isolation via Docker
- ✅ Network isolation (localhost only)

**Weaknesses** (Accepted for MVP):
- ⚠️ Default PostgreSQL password
- ⚠️ No TLS for local connections
- ⚠️ Docker socket access required

**Overall Risk**: **Low**
- Local development tool
- No external network exposure
- Standard security model for Docker-based VSCode extensions

### Recommendation

**Verdict**: ✅ **Safe to ship**

This integration introduces **no new security risks** beyond what already exists in:
- DockerManager (VSMAP-1001, already reviewed)
- MCPConfigWriter (MCPINIT-1001, already reviewed)
- ProcessOrchestrator (VSMAP-1003, already reviewed)

The ~50 lines of new integration code:
- Calls existing, reviewed methods
- Introduces no user input
- Introduces no file operations
- Introduces no credential handling

**Security impact**: Neutral (reuses secure components)

### Post-MVP Improvements

**Priority 1** (Next Release):
- Random PostgreSQL password generation
- Encrypted password storage in settings

**Priority 2** (6 months):
- Schema version check in health check (prevents wrong DB connection)
- Configurable ports via settings

**Priority 3** (1 year):
- TLS support for remote PostgreSQL
- Audit logging for sensitive operations
