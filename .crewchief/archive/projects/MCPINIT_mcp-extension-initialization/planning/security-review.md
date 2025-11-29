# Security Review: MCP Extension Initialization

## Threat Model

### Assets to Protect

1. **User Credentials**
   - OpenAI API keys (starts with `sk-`)
   - Google Cloud service account keys (JSON files)
   - Database passwords

2. **User Data**
   - Source code being indexed
   - MCP configuration containing server definitions
   - VS Code workspace settings

3. **System Resources**
   - Docker daemon access
   - File system write permissions
   - Network ports (5432 for PostgreSQL)

### Threat Actors

1. **Malicious Extensions**: Other VS Code extensions with access to workspace
2. **Compromised Dependencies**: Supply chain attacks via npm packages
3. **Local Attackers**: Other users on shared machines
4. **Network Attackers**: Man-in-the-middle on API calls

### Attack Vectors

1. **Credential Theft**: Logging, file system, environment variables
2. **Configuration Tampering**: Malicious MCP server injection
3. **Command Injection**: Unsafe process spawning
4. **Path Traversal**: Writing outside workspace directory
5. **Docker Escape**: Container breakout or privilege escalation

## Security Requirements

### 1. Credential Management

**Requirement**: API keys and passwords must never be stored in plaintext

**Implementation**:

```typescript
// ✅ CORRECT: Use VS Code SecretStorage
async function storeCredentials(context: vscode.ExtensionContext, provider: string, apiKey: string) {
  await context.secrets.store(`maproom.${provider}.apiKey`, apiKey)
}

// ✅ CORRECT: Use environment variable references in config files
const mcpConfig = {
  mcpServers: {
    maproom: {
      env: {
        OPENAI_API_KEY: '${env:OPENAI_API_KEY}' // VS Code resolves at runtime
      }
    }
  }
}

// ❌ WRONG: Plaintext in config files
const mcpConfig = {
  mcpServers: {
    maproom: {
      env: {
        OPENAI_API_KEY: 'sk-actual-key-here' // Never do this!
      }
    }
  }
}
```

**Validation**:
- [ ] No credentials in `.vscode/mcp.json`
- [ ] No credentials in VS Code settings
- [ ] No credentials in extension logs
- [ ] Credentials stored in `context.secrets`
- [ ] Environment variable syntax used for MCP config

### 2. File System Operations

**Requirement**: Only write to workspace `.vscode/` directory

**Threat**: Path traversal could overwrite system files or other projects

```typescript
// ✅ SAFE: Validate workspace root exists
async function writeConfig(workspaceRoot: string | undefined, config: MCPConfig) {
  if (!workspaceRoot) {
    throw new Error('No workspace folder open')
  }

  const vscodePath = path.join(workspaceRoot, '.vscode')
  const configPath = path.join(vscodePath, 'mcp.json')

  // Verify path is within workspace
  const resolvedConfig = path.resolve(configPath)
  const resolvedWorkspace = path.resolve(workspaceRoot)

  if (!resolvedConfig.startsWith(resolvedWorkspace)) {
    throw new Error('Invalid path: outside workspace')
  }

  // Safe to write
  await fs.promises.mkdir(vscodePath, { recursive: true })
  await fs.promises.writeFile(configPath, JSON.stringify(config, null, 2))
}

// ❌ DANGEROUS: No validation
async function writeConfig(filename: string, config: MCPConfig) {
  await fs.promises.writeFile(filename, JSON.stringify(config)) // Could write anywhere!
}
```

**Validation**:
- [ ] All file writes validate workspace root exists
- [ ] Paths resolved and checked within workspace
- [ ] No symlink following outside workspace
- [ ] Directory creation with `recursive: true` (safe)
- [ ] No hardcoded absolute paths

### 3. Logging and Telemetry

**Requirement**: Never log credentials or sensitive data

```typescript
// ✅ SAFE: Mask credentials
function logSetupStart(provider: string, apiKey: string) {
  outputChannel.appendLine(`[Setup] Provider: ${provider}`)
  outputChannel.appendLine(`[Setup] API Key: ${maskCredential(apiKey)}`)
}

function maskCredential(value: string): string {
  if (value.length < 8) return '***'
  return `${value.substring(0, 4)}...${value.substring(value.length - 4)}`
}

// ❌ DANGEROUS: Full credential in logs
function logSetupStart(provider: string, apiKey: string) {
  outputChannel.appendLine(`[Setup] Starting with ${provider} and key ${apiKey}`)
}
```

**Validation**:
- [ ] Output channel reviewed for credential leaks
- [ ] Error messages don't contain secrets
- [ ] Stack traces sanitized before display

### 4. Network Communication

**Requirement**: Use secure connections for API calls

**Current State**: This extension doesn't make API calls directly - the MCP server does

**Responsibility**: Ensure MCP server configuration uses HTTPS

```typescript
// If we ever fetch data in extension:
const response = await fetch(url, {
  // ✅ Node.js validates certificates by default
  // No need to set rejectUnauthorized: false
})
```

**Validation**:
- [ ] No `rejectUnauthorized: false` in any HTTP clients
- [ ] No hardcoded HTTP URLs (use HTTPS)
- [ ] SSL certificate validation enabled

### 5. Configuration Merging

**Requirement**: Preserve existing MCP servers, don't overwrite

**Threat**: Extension could accidentally delete user's other MCP configurations

```typescript
// ✅ SAFE: Merge with existing config
async function registerMCPServer(workspaceRoot: string, config: ProviderConfig) {
  const configPath = path.join(workspaceRoot, '.vscode', 'mcp.json')

  // Read existing config
  let existingConfig: MCPConfig = { mcpServers: {} }
  if (fs.existsSync(configPath)) {
    const content = await fs.promises.readFile(configPath, 'utf-8')
    existingConfig = JSON.parse(content)
  }

  // Merge, not replace
  existingConfig.mcpServers = existingConfig.mcpServers || {}
  existingConfig.mcpServers.maproom = buildMaproomConfig(config)

  // Write back
  await fs.promises.writeFile(configPath, JSON.stringify(existingConfig, null, 2))
}

// ❌ DANGEROUS: Overwrites entire file
async function registerMCPServer(workspaceRoot: string, config: ProviderConfig) {
  const configPath = path.join(workspaceRoot, '.vscode', 'mcp.json')
  const mcpConfig = {
    mcpServers: {
      maproom: buildMaproomConfig(config) // Lost other servers!
    }
  }
  await fs.promises.writeFile(configPath, JSON.stringify(mcpConfig, null, 2))
}
```

**Validation**:
- [ ] Read existing config before writing
- [ ] Merge maproom server with existing servers
- [ ] Unit test: preserves other MCP servers
- [ ] Unit test: overwrites existing maproom config (update scenario)

## Dependency Security

### NPM Package Vetting

**Current Dependencies** (from extension package.json):
- `@types/node` - Type definitions (dev only)
- `@types/vscode` - Type definitions (dev only)
- `typescript` - Build tool (dev only)
- `vitest` - Test framework (dev only)

**No runtime dependencies!** This is excellent for security.

**For Future Dependencies**:
- [ ] Check npm audit before adding
- [ ] Review package download count (>100k/week preferred)
- [ ] Check last publish date (<6 months old)
- [ ] Review GitHub issues for security concerns
- [ ] Prefer packages with security policy (SECURITY.md)

### Supply Chain Risk

**Mitigation**:
- Use `pnpm` lockfile to pin versions
- Enable Dependabot security updates
- Run `pnpm audit` in CI pipeline

**GitHub Actions**:
```yaml
- name: Security Audit
  run: pnpm audit --audit-level moderate
```

## Secrets Management Strategy

### Storage Options

| Location | Security | Use Case |
|----------|----------|----------|
| VS Code SecretStorage | ✅ Encrypted | API keys, passwords |
| Environment Variables | ⚠️ Process visible | CI/CD, team shared |
| `.vscode/mcp.json` | ❌ Plaintext | References only: `${env:KEY}` |
| Workspace Settings | ❌ Plaintext | Non-sensitive config only |

### Recommended Flow

1. **User Enters Credentials** → Setup wizard collects via `vscode.window.showInputBox({ password: true })`
2. **Extension Stores** → `context.secrets.store('maproom.openai.apiKey', value)`
3. **Pass to CLI** → Set environment variable when spawning process
4. **MCP Config References** → Write `${env:OPENAI_API_KEY}` to mcp.json
5. **User Sets Environment** → User adds to shell profile or .env file

### VS Code Secrets API

```typescript
// Store securely
await context.secrets.store('maproom.provider.apiKey', apiKey)

// Retrieve securely
const apiKey = await context.secrets.get('maproom.provider.apiKey')

// Delete when no longer needed
await context.secrets.delete('maproom.provider.apiKey')

// Listen for changes (e.g., user changed via command palette)
context.secrets.onDidChange((e) => {
  if (e.key.startsWith('maproom.')) {
    // Refresh configuration
  }
})
```

**Benefits**:
- Encrypted storage
- OS keychain integration (macOS Keychain, Windows Credential Manager)
- Synced across machines (if settings sync enabled)
- Not stored in workspace (safe to commit workspace config)

## VS Code Extension Security Best Practices

### Capabilities Declaration

**Current `package.json` capabilities**:
- Workspace file access (for `.vscode/mcp.json`)
- Command execution (for setup wizard)
- Status bar display

**NOT using**:
- Unrestricted file system access
- Network requests (except Docker via CLI)
- Clipboard access
- Terminal access
- Webview (no custom HTML rendering)

### Extension Permissions

```json
// package.json
{
  "capabilities": {
    "untrustedWorkspaces": {
      "supported": false,
      "description": "This extension requires trusted workspace to manage Docker containers"
    }
  }
}
```

**Why refuse untrusted workspaces**: Setup triggers Docker operations and file writes. Only run in workspaces user trusts.

### Code Signing

**For Distribution**:
- [ ] Use `vsce` with publisher token
- [ ] Enable GitHub verified badge
- [ ] Document checksum for VSIX file

```bash
# Generate checksum
sha256sum vscode-maproom-*.vsix > SHA256SUMS

# Verify
sha256sum -c SHA256SUMS
```

## Security Testing

### Static Analysis

**Tools to Run**:
1. **ESLint Security Plugin**
```bash
pnpm add -D eslint-plugin-security
```

```js
// .eslintrc.js
{
  plugins: ['security'],
  extends: ['plugin:security/recommended']
}
```

2. **npm audit**
```bash
pnpm audit --audit-level moderate
```

3. **Semgrep** (optional, for deeper analysis)
```bash
semgrep --config=auto src/
```

### Dynamic Testing

**Security Test Cases**:

```typescript
// test/security/credential-leak.test.ts
describe('Credential Leak Prevention', () => {
  it('should not log API keys to output channel', () => {
    const logger = createTestLogger()
    const manager = new SetupManager(logger)

    manager.runSetup({
      provider: 'openai',
      apiKey: 'sk-test123456'
    })

    const logs = logger.getAllLogs()
    expect(logs.join('\n')).not.toContain('sk-test123456')
  })

  it('should not include API keys in error messages', async () => {
    const manager = new SetupManager(mockOutputChannel)

    try {
      await manager.runSetup({
        provider: 'openai',
        apiKey: 'sk-secret'
      })
    } catch (error) {
      expect(error.message).not.toContain('sk-secret')
    }
  })
})

// test/security/path-traversal.test.ts
describe('Path Traversal Prevention', () => {
  it('should reject paths outside workspace', async () => {
    const writer = new MCPConfigWriter()

    await expect(
      writer.registerMCPServer({
        workspaceRoot: '/workspace',
        outputPath: '../../../etc/passwd' // Attempt traversal
      })
    ).rejects.toThrow('Invalid path')
  })
})

// test/security/command-injection.test.ts
describe('Command Injection Prevention', () => {
  it('should not execute injected commands in provider name', async () => {
    const manager = new SetupManager(mockOutputChannel)

    const maliciousProvider = 'openai; rm -rf /'

    await expect(
      manager.runSetup({ provider: maliciousProvider })
    ).rejects.toThrow('Invalid provider')
  })
})
```

## Incident Response

### If Credentials Leaked

1. **Immediate Actions**:
   - Revoke exposed API keys
   - Notify users via extension update message
   - Publish hotfix release
   - Update documentation

2. **Investigation**:
   - Review logs to determine scope
   - Check GitHub commit history
   - Scan issue tracker for reports

3. **Prevention**:
   - Add test case reproducing the leak
   - Update security review with lesson learned
   - Consider pre-commit hooks for secret scanning

### If Vulnerability Reported

1. **Acknowledge** within 24 hours
2. **Assess** severity using CVSS
3. **Fix** in private branch
4. **Coordinate** disclosure with reporter
5. **Release** patch version
6. **Disclose** via GitHub Security Advisory

### Security Contact

**Future**: Add `SECURITY.md` to repository root:

```markdown
# Security Policy

## Reporting a Vulnerability

Email: security@manifoldlogic.com (if creating)
Or: GitHub Security Advisory (private)

Please include:
- Description of vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

We will respond within 24 hours.
```

## Compliance Considerations

### GDPR (if applicable)

- [ ] Document what data is collected (API keys, workspace paths)
- [ ] Provide deletion mechanism (uninstall removes secrets)
- [ ] No telemetry without consent
- [ ] Privacy policy in marketplace listing

### Open Source Licenses

- [ ] Verify all dependencies are compatible with MIT license
- [ ] No GPL dependencies (copyleft risk)
- [ ] Include LICENSE file in extension
- [ ] Credit third-party code in README

## Security Checklist for Code Review

Before merging any PR:

- [ ] No hardcoded credentials
- [ ] No shell injection vulnerabilities
- [ ] File operations stay within workspace
- [ ] Credentials stored in SecretStorage
- [ ] Error messages sanitized
- [ ] Dependencies audited
- [ ] Tests cover security scenarios
- [ ] No new lint/security warnings

## Security Posture Summary

### Strengths

✅ **Zero runtime dependencies** - Minimal attack surface
✅ **Uses VS Code SecretStorage** - Industry standard credential management
✅ **No subprocess management** - Extension doesn't spawn processes
✅ **No network requests** - No data exfiltration risk
✅ **Workspace-scoped** - No global system changes
✅ **Minimal code** - Only ~150 lines of new code to audit

### Risks

⚠️ **MCP CLI Trust** - Extension trusts VS Code will invoke `@crewchief/maproom-mcp` correctly
⚠️ **Environment Variables** - If user sets globally, visible to all processes
⚠️ **Configuration File Access** - Extension writes to `.vscode/mcp.json`

### Mitigations

- Pin exact MCP version for compatibility
- Recommend workspace-specific environment variables
- Validate all file paths before writing
- Refuse untrusted workspaces

## Conclusion

This extension follows security best practices by:

1. **Storing credentials securely** via VS Code SecretStorage
2. **No subprocess management** - Delegates everything to VS Code MCP client
3. **Limiting file system access** to workspace `.vscode/` directory
4. **Avoiding credential leaks** through environment variable references
5. **Minimizing code** to reduce attack surface (~150 lines)

The extension's security posture is dramatically simpler than originally planned because it doesn't manage Docker or spawn processes. It only:
- Writes a configuration file (`.vscode/mcp.json`)
- Stores credentials in VS Code's secure storage
- Lets VS Code handle MCP server invocation

The primary trust boundary is between VS Code and the MCP CLI. Since both are components we control or trust (Anthropic for VS Code, us for the CLI), this is an acceptable security model.
