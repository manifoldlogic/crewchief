# Security Review: CLI-Maproom Alignment

**Project:** CLIMAP - CLI-Maproom Alignment
**Date:** 2025-01-10

## Security Scope

**Context:** CLI refactoring project focused on:
- Command registration changes
- Environment validation
- Documentation updates
- Argument forwarding (no changes to forwarding logic)

**Security Posture:** This is a **low-risk refactoring** with minimal security surface

## Architecture Security Analysis

### Current Architecture (Unchanged)

```
User Input → CLI Parser → Argument Array → spawnSync → Rust Binary
```

**Security Properties:**
1. **No shell injection:** Uses `spawnSync` with array args, not shell strings
2. **No eval:** No dynamic code execution
3. **No network:** CLI doesn't make HTTP requests
4. **No database:** CLI doesn't connect to PostgreSQL

**Analysis:** ✅ Architecture remains secure

### Changes Being Made

1. **Command structure refactor**
   - Risk: None (same argument handling)
   - Change: Commander.js routing only

2. **Environment validation**
   - Risk: Information disclosure (error messages)
   - Change: New validation checks

3. **Deprecated aliases**
   - Risk: None (same forwarding)
   - Change: Additional command registrations

## Threat Model

### Assets

1. **User credentials** (OPENAI_API_KEY, GOOGLE_APPLICATION_CREDENTIALS)
2. **Database connection string** (MAPROOM_DATABASE_URL)
3. **Local file system** (via Rust binary operations)

### Threats (STRIDE Analysis)

#### Spoofing
**Threat:** Attacker impersonates maproom binary

**Current Mitigation:**
```typescript
const bin = resolvePackagedMaproomBin()
// Checks in order:
// 1. CREWCHIEF_MAPROOM_BIN env var (explicit path)
// 2. Packaged binary in CLI (verified during build)
// 3. Symlink in bin/ (developer setup)
// 4. PATH lookup (user-controlled)
```

**Risk:** MEDIUM (PATH injection possible)

**Additional Mitigation (New):**
```typescript
// Add binary verification
function verifyBinary(binPath: string): boolean {
  // Check file exists and is executable
  if (!fs.existsSync(binPath)) return false

  try {
    // Verify it's actually crewchief-maproom by checking --version
    const result = spawnSync(binPath, ['--version'], {
      encoding: 'utf8',
      timeout: 1000
    })
    return result.stdout?.includes('crewchief-maproom')
  } catch {
    return false
  }
}
```

**Status:** ⚠️ Consider adding binary verification

#### Tampering
**Threat:** Attacker modifies arguments before forwarding

**Current Mitigation:**
- Arguments passed as array (no string manipulation)
- Direct spawnSync call (no intermediate steps)

**Risk:** LOW (no attack vector in CLI)

**Status:** ✅ No changes needed

#### Repudiation
**Threat:** User denies running destructive command

**Current Mitigation:** None (CLI doesn't log)

**Risk:** LOW (not a security concern for CLI tool)

**Status:** ✅ No changes needed

#### Information Disclosure
**Threat:** Error messages leak sensitive information

**Current Behavior:**
```typescript
// Rust binary might output connection strings in errors
// Example: "Failed to connect to postgresql://user:pass@host/db"
```

**New Validation (Potential Issue):**
```typescript
// Don't log full connection strings
if (!dbUrl) {
  errors.push('No database connection configured.')
  // ❌ DON'T: errors.push(`Missing: ${dbUrl}`)
  // ✅ DO: errors.push('Set MAPROOM_DATABASE_URL environment variable.')
}
```

**Mitigation:**
- Validation messages reference env var names, not values
- Never log credentials or connection strings
- Keep error messages generic

**Status:** ⚠️ Ensure validation doesn't leak secrets

#### Denial of Service
**Threat:** Malicious input causes CLI to hang/crash

**Attack Vectors:**
1. Extremely long arguments
2. Special characters in arguments
3. Infinite loop in validation

**Current Mitigation:**
- Commander.js handles parsing safely
- spawnSync has no shell expansion
- Rust binary validates inputs

**New Validation (Potential Issue):**
```typescript
// Validation should be fast and bounded
function validateMaproomEnvironment(): ValidationResult {
  // ✅ Just env var checks (fast)
  // ❌ Don't: Try connecting to database (slow, can hang)
  // ❌ Don't: Validate all files in repo (unbounded)
}
```

**Status:** ✅ Validation is lightweight (env checks only)

#### Elevation of Privilege
**Threat:** CLI gains unintended permissions

**Current Behavior:**
- CLI runs as user (no privilege escalation)
- Rust binary runs as user
- No sudo, no setuid

**Risk:** NONE (not applicable)

**Status:** ✅ No changes needed

## Known Security Gaps

### 1. Environment Variable Exposure

**Gap:** API keys in environment variables visible to all processes

**Context:**
```bash
export OPENAI_API_KEY=sk-...
# Now visible to all child processes, ps aux, etc.
```

**Recommendation:** Document secure alternatives
- Use secret management tools (Vault, AWS Secrets Manager)
- Use `.env` files with restricted permissions (chmod 600)
- Never commit `.env` files

**Status:** ⚠️ Document best practices, not enforced

### 2. Binary Verification

**Gap:** No cryptographic verification of Rust binary

**Attack Scenario:**
1. Attacker replaces `crewchief-maproom` binary in PATH
2. CLI executes malicious binary
3. Malicious binary reads credentials from environment

**Current Mitigation:** Packaged binary checked first (before PATH)

**Potential Mitigation:**
- Checksum verification of packaged binary
- Code signing (macOS/Windows)

**Risk:** MEDIUM for PATH-based lookup, LOW for packaged binary

**MVP Decision:** ⚠️ Document risk, implement verification in Phase 2

### 3. Credential Logging

**Gap:** Rust binary might log credentials in debug mode

**Current State:** Unknown (Rust team responsibility)

**Recommendation:** Coordinate with Rust team
- Ensure `MAPROOM_DATABASE_URL` redacted in logs
- Ensure API keys never logged
- Review Rust logging configuration

**Status:** ⚠️ Out of scope for CLI, document concern

## Input Validation

### Command-Line Arguments

**Current:**
```typescript
.argument('[args...]')  // Commander.js parses safely
.action((args: string[]) => runMaproomForward(args || []))
```

**Security:**
- Commander.js prevents injection (uses array args)
- spawnSync prevents shell expansion
- Rust binary validates actual values

**Additional Validation:** None needed (Rust handles it)

**Status:** ✅ Secure by design

### Environment Variables

**New Validation:**
```typescript
const provider = process.env.MAPROOM_EMBEDDING_PROVIDER

// Validate against known providers
const knownProviders = ['ollama', 'openai', 'google']
if (provider && !knownProviders.includes(provider)) {
  warnings.push(`Unknown provider: ${provider}`)
  warnings.push(`Known providers: ${knownProviders.join(', ')}`)
}
```

**Security:**
- No injection risk (env vars are strings)
- No execution of env var values
- Just reading and checking

**Status:** ✅ Safe validation

## Output Handling

### Error Messages

**Secure Pattern:**
```typescript
// ✅ Good: Reference names, not values
logger.error('MAPROOM_DATABASE_URL not set')

// ❌ Bad: Expose values
logger.error(`Invalid URL: ${dbUrl}`)
```

**New Validation Code:**
```typescript
// Audit all error messages
export function validateMaproomEnvironment(): ValidationResult {
  // ✅ Never include credential values
  // ✅ Never include connection strings
  // ✅ Never include API keys
  // ✅ Only reference environment variable names
}
```

**Status:** ✅ Will audit during implementation

### Help Text

**Current:**
```typescript
.addHelpText('after', '\nExample: crewchief maproom:scan')
```

**Security:** No sensitive data in help (examples use placeholders)

**Status:** ✅ Safe

## Dependencies

### New Dependencies

**None.** This project doesn't add dependencies.

**Existing Dependencies:**
- `commander` - Well-maintained, no known vulnerabilities
- `chalk` - Simple library, low risk
- `inquirer` - Interactive prompts, no security concerns

**Status:** ✅ No new attack surface

## Secrets Management

### Current Approach

Users set environment variables directly:
```bash
export OPENAI_API_KEY=sk-...
export MAPROOM_DATABASE_URL=postgresql://user:pass@host/db
```

**Risks:**
- Visible in process list
- Stored in shell history
- Accessible to all child processes

### Recommended Approach (Documentation)

**Add to README:**
```markdown
## Secure Credential Management

### Option 1: .env File (Development)
```bash
# Create .env file (never commit!)
cat > .env <<EOF
MAPROOM_DATABASE_URL=postgresql://...
OPENAI_API_KEY=sk-...
EOF

# Restrict permissions
chmod 600 .env

# Load with direnv or dotenv
direnv allow
```

### Option 2: Secret Manager (Production)
```bash
# AWS Secrets Manager
export OPENAI_API_KEY=$(aws secretsmanager get-secret-value --secret-id openai-key --query SecretString --output text)

# HashiCorp Vault
export MAPROOM_DATABASE_URL=$(vault kv get -field=url maproom/db)
```

### Security Best Practices
- Never commit credentials to git
- Use `.gitignore` for `.env` files
- Rotate API keys regularly
- Use read-only database credentials when possible
```

**Status:** ✅ Document, don't enforce (MVP)

## MVP-Appropriate Security

### What We're Implementing

✅ **Safe argument forwarding** (already exists)
✅ **No credential logging** (audit validation code)
✅ **Generic error messages** (don't leak secrets)
✅ **Security documentation** (best practices guide)

### What We're Deferring

⚠️ **Binary verification** - Nice to have, not critical for MVP
⚠️ **Credential encryption** - User responsibility
⚠️ **Audit logging** - Not needed for CLI tool
⚠️ **Rate limiting** - Rust binary concern

### What We're NOT Doing

❌ **Secret storage** - Use external tools (Vault, etc.)
❌ **Access control** - OS-level file permissions
❌ **Network security** - Rust binary handles HTTPS
❌ **Cryptographic operations** - Rust binary concern

## Security Checklist

**Before Merge:**

- [ ] **Code Review**
  - [ ] No credentials in error messages
  - [ ] No credentials in logs
  - [ ] No credentials in help text
  - [ ] No shell command construction (use array args)

- [ ] **Validation Audit**
  - [ ] validateMaproomEnvironment() doesn't leak values
  - [ ] Error messages reference env var names only
  - [ ] Warnings are informative but not revealing

- [ ] **Documentation**
  - [ ] Security best practices section added
  - [ ] Credential management documented
  - [ ] `.env` file usage explained
  - [ ] Secret manager integration suggested

- [ ] **Dependency Check**
  - [ ] Run `pnpm audit`
  - [ ] Check for high/critical vulnerabilities
  - [ ] Update dependencies if needed

## Risk Assessment

### Overall Risk: LOW

**Justification:**
1. No new network operations
2. No new database operations
3. No new credential storage
4. Pure refactoring with validation layer
5. Forwarding model unchanged

### Residual Risks

**MEDIUM: Binary Spoofing via PATH**
- Mitigation: Document risk, use packaged binary
- Future: Add verification in Phase 2

**LOW: Credential Exposure in Environment**
- Mitigation: Document best practices
- Future: Support credential file input

**LOW: Error Message Information Disclosure**
- Mitigation: Audit all validation messages
- Future: Structured logging with redaction

## Conclusion

**Security Posture:** This refactoring maintains existing security properties and adds minimal new surface area (environment validation). The validation layer is designed with security in mind (no credential logging, generic error messages).

**MVP Readiness:** ✅ Safe to ship with:
1. Validation message audit
2. Security best practices documentation
3. Dependency check

**No Security Blockers Identified**
