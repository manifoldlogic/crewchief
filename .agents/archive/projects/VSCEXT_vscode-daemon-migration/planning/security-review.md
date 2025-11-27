# Security Review: VSCode Extension Daemon Migration

## Executive Summary

**Overall Risk Level: LOW**

This project simplifies the extension architecture by removing Docker dependency and switching to local-only components (SQLite, host Ollama). The security posture improves because:

1. No containerized services to secure
2. No network-exposed database
3. Local-only communication (localhost Ollama)
4. Existing credential management retained

## Security Assessment

### 1. Ollama Integration

**Component**: OllamaClient communicating with localhost:11434

**Risk Level**: LOW

**Analysis**:
- Communication is localhost-only (127.0.0.1)
- No authentication required (Ollama's default)
- No sensitive data sent (only model names, text for embedding)
- Model pull downloads from Ollama's official registry

**Mitigations**:
- ✅ Hardcode `localhost` to prevent SSRF
- ✅ Validate model names (alphanumeric + colon + dash only)
- ✅ Timeout all HTTP requests (prevent hanging)

**Code Pattern**:
```typescript
class OllamaClient {
  // SECURITY: Hardcoded to localhost, not configurable
  private readonly baseUrl = 'http://127.0.0.1:11434'

  async pullModel(name: string): Promise<void> {
    // SECURITY: Validate model name format
    if (!/^[a-z0-9][a-z0-9._-]*(?::[a-z0-9._-]+)?$/i.test(name)) {
      throw new Error('Invalid model name format')
    }
    // ...
  }
}
```

### 2. Watch Process Spawning

**Component**: WatchProcessManager spawning `crewchief-maproom watch`

**Risk Level**: LOW

**Analysis**:
- Binary path comes from extension root (not user input)
- Arguments are constructed from validated workspace path
- Environment variables contain API credentials (existing pattern)

**Mitigations**:
- ✅ Binary path resolved from `context.extensionPath` (VSCode-controlled)
- ✅ Workspace path validated via VSCode's `workspaceFolders` API
- ✅ No shell execution (direct spawn)
- ✅ Environment variable passthrough is controlled whitelist

**Code Pattern**:
```typescript
class WatchProcessManager {
  async start(): Promise<void> {
    // SECURITY: Binary path from extension, not user input
    const binaryPath = path.join(this.config.extensionRoot, 'bin', platform, 'crewchief-maproom')

    // SECURITY: No shell=true, direct process spawn
    this.process = spawn(binaryPath, [
      'watch',
      '--path', this.config.workspaceRoot, // From VSCode API
    ], {
      shell: false, // Explicit: no shell injection risk
      env: this.buildEnvironment(), // Controlled whitelist
    })
  }

  private buildEnvironment(): NodeJS.ProcessEnv {
    // SECURITY: Whitelist of allowed environment variables
    return {
      MAPROOM_DATABASE_URL: this.config.databaseUrl,
      MAPROOM_EMBEDDING_PROVIDER: this.config.provider,
      // Only pass through known API key env vars
      ...(process.env.OPENAI_API_KEY && { OPENAI_API_KEY: process.env.OPENAI_API_KEY }),
    }
  }
}
```

### 3. SQLite Database Access

**Component**: Direct SQLite file access via watch process

**Risk Level**: LOW

**Analysis**:
- Database file stored in user's home directory (`~/.maproom/maproom.db`)
- No network exposure (local file only)
- Standard SQLite permissions (user-readable)
- No PII stored (only code chunks and embeddings)

**Mitigations**:
- ✅ Default path in user's home directory
- ✅ File permissions controlled by OS
- ✅ No SQL injection risk (Rust binary uses parameterized queries)

### 4. Credential Storage (Unchanged)

**Component**: SecretsManager using VSCode SecretStorage

**Risk Level**: LOW (No change from current)

**Analysis**:
- API keys stored in VSCode's encrypted SecretStorage
- Keys passed to watch process via environment variables
- Existing, reviewed pattern from previous implementation

**Mitigations**:
- ✅ Uses VSCode's built-in SecretStorage API
- ✅ Keys never logged or exposed in UI
- ✅ Environment variable passthrough is standard practice

### 5. Removed Attack Surface (Docker)

**Security Improvement**: Removing Docker dependency eliminates:

| Removed Risk | Description |
|--------------|-------------|
| Container escape | No containers to escape from |
| Exposed PostgreSQL | No network-listening database |
| Docker daemon access | No Docker socket interaction |
| Image supply chain | No container images to verify |
| Volume mount risks | No host path mounts |

## Threat Model

### Assets

1. **User's source code** - Indexed and stored locally
2. **API credentials** - OpenAI/Google keys if configured
3. **Embeddings** - Vector representations of code

### Threat Actors

1. **Malicious extension** - Other VSCode extensions
2. **Local attacker** - Access to user's machine
3. **Network attacker** - (Minimal surface after this change)

### Attack Vectors

| Vector | Pre-Migration | Post-Migration | Change |
|--------|--------------|----------------|--------|
| Network-exposed DB | PostgreSQL on :5433 | None | ✅ Removed |
| Docker daemon | Extension needs daemon | None | ✅ Removed |
| Ollama network | N/A | localhost:11434 | ⚠️ New (localhost only) |
| File system | SQLite file | SQLite file | No change |
| Credential storage | SecretStorage | SecretStorage | No change |

## Recommendations

### Required (Must Fix Before Ship)

None - no critical security issues identified.

### Recommended (Should Address)

1. **Validate Ollama host setting** - If making host configurable in future, validate it's localhost
2. **Log security events** - Log failed Ollama connections, invalid model names

### Future Considerations (Not for This Project)

1. **SQLite encryption** - For high-security environments (not typical use case)
2. **Ollama authentication** - When Ollama adds auth support
3. **Binary signature verification** - Verify watch binary signature

## Compliance Notes

### Data Residency

- All data stored locally (user's machine)
- Embeddings may be generated via cloud APIs (OpenAI/Google) if user chooses
- Ollama option keeps all processing local

### GDPR/Privacy

- No PII collected by extension
- Code is user's own code
- No telemetry changes in this project

## Security Checklist

Before shipping, verify:

- [ ] Ollama client hardcodes localhost
- [ ] Model names are validated
- [ ] No shell=true in spawn calls
- [ ] Environment variable whitelist is explicit
- [ ] No Docker code remains
- [ ] No PostgreSQL connection code remains
- [ ] Secrets still use VSCode SecretStorage

## Approval

**Security Review Status**: ✅ Approved for Implementation

**Rationale**: This project reduces attack surface by removing Docker/PostgreSQL and adds only localhost-based Ollama communication. No new security concerns introduced.

**Reviewer Notes**:
- The move to SQLite + host Ollama is a security improvement
- Existing credential handling is appropriate
- No external network connections except optional cloud embedding APIs (unchanged)
