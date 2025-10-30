# MCPSTART: MCP Provider Startup Fix - Security Review

## Security Context

This project involves:
- **Docker container orchestration** (potential for container escape, resource abuse)
- **Environment variable handling** (potential for credential exposure)
- **Command execution** (potential for injection attacks)
- **File system operations** (potential for unauthorized access)
- **Network services** (PostgreSQL, Ollama, MCP server)

However, this is a **developer tool** for local development, not a production service. Security should be reasonable and pragmatic, not paranoid.

## Threat Model

### Assets to Protect

1. **User Credentials**:
   - `GOOGLE_APPLICATION_CREDENTIALS` file path
   - `OPENAI_API_KEY` value
   - Database passwords (currently hardcoded as `maproom`)

2. **Code and Data**:
   - Indexed codebase (stored in PostgreSQL)
   - Docker volumes (embeddings, database data)

3. **Host System**:
   - Docker daemon access
   - File system access
   - Network ports

### Threat Actors

**In Scope**:
- Malicious npm package dependencies
- Compromised MCP client
- Accidental credential exposure

**Out of Scope** (developer responsibility):
- Malicious users with host access (they already have full control)
- Network attacks (local development environment)
- Physical attacks (not relevant)

## Security Analysis

### 1. Environment Variable Exposure

**Current State**:
- Environment variables are passed from MCP client → CLI → Docker Compose → containers
- Variables are logged in diagnostic mode
- Variables are visible in `docker inspect` output

**Risk**: Credentials visible in logs and Docker metadata

**Assessment**: **LOW RISK** for MVP
- This is a local development tool, not a shared service
- Docker metadata is already accessible to anyone with Docker access
- Diagnostic logs are opt-in via `MAPROOM_MCP_DEBUG=true`

**Mitigation**:
```javascript
// Redact sensitive values in logs
function diagnosticLog(message, data) {
  if (DIAGNOSTIC_MODE) {
    const redacted = redactSensitiveData(data);
    console.error('🔍 [DIAGNOSTIC]', message);
    console.error('   ', JSON.stringify(redacted, null, 2));
  }
}

function redactSensitiveData(data) {
  const sensitive = [
    'GOOGLE_APPLICATION_CREDENTIALS',
    'OPENAI_API_KEY',
    'DATABASE_URL'
  ];

  const redacted = { ...data };
  sensitive.forEach(key => {
    if (redacted[key]) {
      redacted[key] = '(redacted)';
    }
  });

  return redacted;
}
```

**Enterprise Consideration**: Production deployments should use secrets management (HashiCorp Vault, AWS Secrets Manager). For MVP, we document best practices:

```markdown
## Security Best Practices

- Use separate service accounts per environment
- Rotate credentials regularly
- Never commit credentials to version control
- Use `.env` files (gitignored) for local development
- Consider using credential management tools for teams
```

### 2. Command Injection

**Current State**:
- User input flows to Docker Compose commands
- Service names are constructed from predefined list
- No direct user input to shell commands

**Risk**: Command injection via environment variables

**Assessment**: **VERY LOW RISK**
- Service names are hardcoded: `['postgres', 'ollama', 'maproom-mcp']`
- Environment variable values are not interpolated into shell commands
- Using `spawn()` with argument arrays prevents shell injection

**Existing Safeguards**:
```javascript
// Service names are NOT user-controlled
const allServices = ['postgres', 'ollama', 'maproom-mcp'];
const requiredServices = allServices.filter(/* ... */);

// Arguments passed as array, not string (no shell interpretation)
spawn('docker', ['compose', 'up', '-d', ...requiredServices], {
  // ...
});
```

**No Additional Mitigation Needed**: Current approach is secure.

### 3. File System Access

**Current State**:
- CLI writes to `~/.maproom-mcp/`
- Copies files from package to user directory
- Auto-updates `docker-compose.yml`

**Risk**: Unauthorized file writes, path traversal

**Assessment**: **LOW RISK**
- File paths are constructed from `os.homedir()` (trusted)
- No user-controlled path components
- Auto-update only replaces specific known files

**Existing Safeguards**:
```javascript
const CONFIG_DIR = path.join(os.homedir(), '.maproom-mcp');
// ^ No user input, safe

const COMPOSE_FILE = path.join(CONFIG_DIR, 'docker-compose.yml');
// ^ Hardcoded filename, safe
```

**Additional Mitigation** (defense in depth):
```javascript
// Verify paths stay within CONFIG_DIR
function verifySafePath(filePath) {
  const resolved = path.resolve(filePath);
  const configDir = path.resolve(CONFIG_DIR);

  if (!resolved.startsWith(configDir)) {
    throw new Error(`Unsafe path detected: ${filePath}`);
  }

  return resolved;
}

// Use before file operations
const safeComposePath = verifySafePath(COMPOSE_FILE);
fs.copyFileSync(srcCompose, safeComposePath);
```

### 4. Docker Container Security

**Current State**:
- Containers run with default permissions
- No user namespace remapping
- PostgreSQL has hardcoded password
- Containers have network access

**Risk**: Container escape, privilege escalation

**Assessment**: **LOW RISK** for MVP
- Local development environment
- User already has Docker daemon access
- Containers don't run with `--privileged`

**Pragmatic Mitigations**:

**Option A: Document secure configuration** (MVP approach)
```markdown
## Security Hardening (Optional)

For shared development environments:

1. Change default database password in docker-compose.yml
2. Enable Docker user namespace remapping
3. Restrict container capabilities
4. Use read-only file systems where possible
```

**Option B: Secure defaults** (post-MVP)
```yaml
services:
  postgres:
    environment:
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-maproom}  # Allow override
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - DAC_OVERRIDE
      - SETGID
      - SETUID

  maproom-mcp:
    read_only: true
    security_opt:
      - no-new-privileges:true
```

**Recommendation**: **Document secure configuration** for MVP, implement hardening post-MVP if needed.

### 5. Network Exposure

**Current State**:
- PostgreSQL exposed on `0.0.0.0:5433`
- Ollama exposed on `0.0.0.0:11434`
- MCP server accessed via stdio (not network)

**Risk**: Services accessible from network

**Assessment**: **LOW RISK** for MVP
- Intended for local development
- Database requires authentication
- Ollama is public API (no auth)

**Mitigation**:
```yaml
services:
  postgres:
    ports:
      - "127.0.0.1:5433:5432"  # Bind to localhost only

  ollama:
    ports:
      - "127.0.0.1:11434:11434"  # Bind to localhost only
```

**Recommendation**: Bind to localhost only (simple change, high value).

### 6. Dependency Security

**Current State**:
- Package has production dependencies (pg, pino, zod, execa)
- Uses `npx -y` which auto-accepts installs
- No dependency scanning in CI

**Risk**: Supply chain attack via compromised dependency

**Assessment**: **MEDIUM RISK**
- npm ecosystem has history of malicious packages
- Using minimal dependencies reduces surface area
- `execa` is a well-maintained package

**Mitigations**:

**For MVP**:
```json
{
  "scripts": {
    "prepublishOnly": "npm audit --audit-level=high"
  }
}
```

**Post-MVP**:
- Enable Dependabot alerts
- Add `npm audit` to CI pipeline
- Consider using `npm ci` instead of `npm install`
- Document security scanning for users

### 7. Credential File Handling

**Current State**:
- Google credentials file path passed via env var
- File is mounted into container (currently not implemented)

**Risk**: Credential file exposed to container

**Assessment**: **LOW RISK**
- Container needs credentials to function
- File permissions preserved from host
- No credential file copying (just mounting)

**Current docker-compose.yml** doesn't mount credentials - should it?

**Recommendation**:
```yaml
maproom-mcp:
  volumes:
    - ${GOOGLE_APPLICATION_CREDENTIALS:-/dev/null}:${GOOGLE_APPLICATION_CREDENTIALS:-/dev/null}:ro
    # ^ Only mount if set, read-only
```

## Security Checklist

### Must Have (MVP)
- [x] No command injection (using spawn with args array)
- [x] No path traversal (using path.join from trusted base)
- [ ] Redact credentials in diagnostic logs
- [ ] Bind services to localhost only
- [ ] Run `npm audit` before publish

### Should Have (Post-MVP)
- [ ] Container security hardening (capabilities, read-only FS)
- [ ] User namespace remapping documentation
- [ ] Dependency scanning in CI
- [ ] Security advisory documentation

### Nice to Have (Future)
- [ ] Secret rotation tooling
- [ ] Audit logging for sensitive operations
- [ ] Rate limiting for API endpoints
- [ ] Encrypted credential storage

## Implementation Priority

For this MVP fix, implement these security improvements:

**Phase 1 (Critical - Include in Fix)**:
1. ✅ Bind PostgreSQL and Ollama to localhost only
2. ✅ Redact sensitive values in diagnostic logs
3. ✅ Run `npm audit` before publishing

**Phase 2 (Important - Next Release)**:
4. Add dependency scanning to CI
5. Document security best practices
6. Add credential file mounting (if needed)

**Phase 3 (Enhancement - Future)**:
7. Container security hardening
8. Secrets management integration

## Code Changes Required

### 1. Redact Sensitive Data in Logs

**File**: `bin/cli.cjs`

```javascript
const SENSITIVE_ENV_VARS = [
  'GOOGLE_APPLICATION_CREDENTIALS',
  'OPENAI_API_KEY',
  'DATABASE_URL',
  'POSTGRES_PASSWORD'
];

function redactSensitive(data) {
  if (!data || typeof data !== 'object') return data;

  const redacted = { ...data };

  Object.keys(redacted).forEach(key => {
    const upperKey = key.toUpperCase();
    if (SENSITIVE_ENV_VARS.some(sensitive => upperKey.includes(sensitive))) {
      redacted[key] = '(redacted)';
    }
  });

  return redacted;
}

function diagnosticLog(message, data) {
  if (DIAGNOSTIC_MODE) {
    console.error('🔍 [DIAGNOSTIC]', message);
    if (data) {
      const safe = redactSensitive(data);
      console.error('   ', JSON.stringify(safe, null, 2));
    }
  }
}
```

### 2. Bind Services to Localhost

**File**: `config/docker-compose.yml`

```yaml
services:
  postgres:
    ports:
      - "127.0.0.1:5433:5432"  # Changed from 0.0.0.0

  ollama:
    ports:
      - "127.0.0.1:11434:11434"  # Changed from 0.0.0.0
```

### 3. Add npm audit Check

**File**: `package.json`

```json
{
  "scripts": {
    "prepublishOnly": "tsc && npm audit --audit-level=high",
    "security-check": "npm audit --audit-level=moderate"
  }
}
```

## Security Documentation

Add to package README:

```markdown
## Security Considerations

### Credentials
- Never commit credentials to version control
- Use environment variables for sensitive configuration
- Consider using credential management tools for team environments

### Network Exposure
Services are bound to localhost (127.0.0.1) by default. To expose
to network (e.g., for Docker Desktop on macOS with containers in VM):

```yaml
# Edit ~/.maproom-mcp/docker-compose.yml
services:
  postgres:
    ports:
      - "0.0.0.0:5433:5432"  # WARNING: Exposes to network
```

### Diagnostic Logs
Diagnostic mode redacts sensitive values, but exercise caution
when sharing logs publicly.

### Reporting Security Issues
Email security@crewchief.dev (don't open public issues for vulnerabilities)
```

## Risk Summary

| Risk | Severity | Likelihood | Mitigation | Status |
|------|----------|------------|------------|--------|
| Credential exposure in logs | Medium | Low | Redact sensitive values | ✅ Planned |
| Command injection | High | Very Low | Using spawn with args | ✅ Existing |
| Network service exposure | Low | Medium | Bind to localhost | ✅ Planned |
| Container escape | High | Very Low | Default Docker security | ✅ Acceptable |
| Dependency compromise | Medium | Low | npm audit check | ✅ Planned |
| Path traversal | Medium | Very Low | Trusted path construction | ✅ Existing |

## Conclusion

This is a **developer tool for local development**, not a production service or shared infrastructure. Security should be **pragmatic**:

- ✅ **Prevent obvious vulnerabilities**: Command injection, path traversal, credential leakage
- ✅ **Document best practices**: Let developers make informed choices
- ✅ **Don't over-engineer**: No need for HSM, secrets management, audit logging for MVP
- ✅ **Defense in depth**: Multiple layers of simple protections

The three security improvements for this MVP are:
1. Redact credentials in diagnostic logs
2. Bind services to localhost
3. Run npm audit before publishing

These are **low-effort, high-value** changes that cover the most likely risks without adding complexity. Post-MVP, we can add container hardening and dependency scanning if there's demand.
