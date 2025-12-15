# Security Review: Maproom Binary Configuration

## Security Assessment

### Overview

This project completes an existing configuration feature for specifying custom binary paths. Security considerations are minimal since:
- No new attack surface (config already loads arbitrary JavaScript)
- No network communication
- No sensitive data handling
- No authentication/authorization changes

**Risk Level: LOW**

The primary security concern is **arbitrary code execution via config files**, which is an existing, accepted risk of the JavaScript config file pattern.

### Authentication & Authorization

**Not applicable.** This project does not involve:
- User authentication
- Access control
- API endpoints
- Multi-user systems

The config file is local to the developer's machine and requires filesystem access to modify.

### Data Protection

**No sensitive data involved.**

**Data handled:**
1. Config file path (filesystem path)
2. Binary path (filesystem path)
3. Environment variable value (CREWCHIEF_MAPROOM_BIN)

**None of these are sensitive.** All are local filesystem paths on the developer's machine.

**No encryption needed:** Paths are not secrets and are stored in plaintext config files by design.

### Input Validation

**Config validation:** Already implemented via Zod schema

```typescript
export const RepositorySchema = z.object({
  maproomBinaryPath: z.string().optional(),
})
```

**Validation coverage:**
- ✅ Type validation (must be string or undefined)
- ✅ Optional field (won't crash if missing)
- ❌ Path validation (no check if path is safe)

**Path injection risks:**

| Attack Vector | Risk | Mitigation |
|---------------|------|------------|
| Path traversal (../../etc/passwd) | Low | Binary execution requires file to exist; OS prevents executing non-binaries |
| Absolute path to system binary (/bin/rm) | Medium | User could point to destructive system binary |
| Symlink to system binary | Medium | Same as above |
| Binary with malicious code | High | User explicitly configures their own binary - assumed trusted |

**Mitigation strategy:**
- **Accepted risk**: Config files execute arbitrary JavaScript already (more dangerous than binary paths)
- **Trust model**: Developer controls their own config file
- **No validation added**: Would be security theater (config can execute arbitrary code anyway)
- **Documentation**: Clearly state config file should be gitignored (already true for .local.js)

**Decision:** No additional path validation. The trust model assumes developers control their own environment.

### Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| No validation of binary path safety | Low | Developer controls config, assumed trusted | Accepted |
| Config executes arbitrary JavaScript | High | Inherent to JS config pattern, matches industry standards | Accepted |
| Relative paths could escape project | Low | Resolved relative to config file location | Accepted |
| No binary signature verification | Medium | Out of scope for local development tool | Accepted |

### Command Injection

**Scenario:** Could a malicious binary path cause command injection?

**Analysis:**
```typescript
const result = findMaproomBinary({
  configPath: config.repository.maproomBinaryPath
})

spawnSync(result.path, args, { stdio: 'inherit' })
```

**Risk:** Low
- Binary path is used directly in `spawnSync`, not shell-interpreted
- No string concatenation or interpolation
- Args are controlled by CLI, not config

**Mitigation:** None needed. `spawnSync` doesn't invoke a shell by default.

**Edge case:** What if path contains shell metacharacters like `;` or `|`?
- **Not a risk**: spawnSync treats path as literal string, doesn't interpret shell syntax

### Arbitrary Code Execution

**Primary risk:** Config file can execute arbitrary JavaScript during import.

**Example attack:**
```javascript
// crewchief.config.local.js
import { execSync } from 'child_process'
execSync('rm -rf /')  // Malicious code

export default {
  repository: {
    maproomBinaryPath: './safe/path'
  }
}
```

**Risk level:** HIGH (but accepted)

**Mitigations:**
1. **Config file is local** - attacker needs filesystem access
2. **Gitignored** - .local.js files not committed, can't inject via git
3. **Standard pattern** - Same risk as tsconfig.json, .eslintrc.js, etc.
4. **Documentation** - Clearly state .local.js should be gitignored

**Decision:** Accepted risk. This is standard for JavaScript tooling.

### Privilege Escalation

**Could this feature enable privilege escalation?**

**Scenarios:**
1. Point to setuid binary → Execute with elevated privileges
2. Point to system binary → Execute system commands
3. Point to malicious binary → Run attacker code

**Analysis:**
- Binary runs with user's privileges (no elevation)
- User already controls environment (can run any binary directly)
- Config merely provides convenience (vs. env var or direct execution)

**Risk:** None. No privilege escalation possible beyond what user already has.

### Dependencies

**No new dependencies introduced.**

**Existing dependencies:**
- Zod (schema validation) - well-maintained, widely used
- Node.js built-ins (fs, path, child_process) - no CVE concerns

**Supply chain risk:** Not applicable to this change.

## MVP Security Scope

### In Scope

1. **Config validation** - Zod ensures type safety
2. **Error handling** - Graceful fallback if config invalid
3. **Documentation** - Warn about gitignoring .local.js files

### Out of Scope

1. **Binary signature verification** - Not needed for local development
2. **Path allowlisting** - Would be ineffective (arbitrary JS execution already possible)
3. **Sandboxing** - Out of scope for development tool
4. **Audit logging** - Not needed for single-user local tool
5. **Binary version constraints** - Feature work, not security

### Future Security Considerations

**If this feature expands in the future:**

1. **Multi-user environments** - Would need access controls
2. **Remote config loading** - Would need HTTPS, signature verification
3. **Binary download** - Would need checksum verification
4. **CI/CD usage** - Would need allowlisting, audit logs

**None apply to current scope** (local development only).

## Security Checklist

- [x] No hardcoded secrets (no secrets involved)
- [x] Input validation on external inputs (Zod validates config schema)
- [x] Proper error handling (try/catch prevents crashes)
- [x] Dependencies are up to date (no new dependencies)
- [x] No SQL injection vulnerabilities (no SQL involved)
- [x] No XSS vulnerabilities (no web interface)
- [x] No command injection (spawnSync doesn't use shell)
- [x] No path traversal beyond accepted risk (relative paths resolved safely)
- [x] No privilege escalation (runs with user privileges)
- [x] Documentation includes security guidance (gitignore .local.js)

## Threat Model

### Threat Actors

**Primary threat:** Malicious contributor to codebase
- Could modify crewchief.config.js (committed file)
- Could not modify crewchief.config.local.js (gitignored)

**Mitigation:** Code review catches malicious config changes.

**Secondary threat:** Compromised developer machine
- Attacker has filesystem access
- Can modify any file, including config
- Config binary path is least of concerns (attacker can do anything)

**Mitigation:** None needed. If machine is compromised, game over anyway.

**Not a threat:** External attacker without filesystem access
- Cannot modify config files
- Cannot exploit this feature remotely

### Attack Vectors

**Ranked by likelihood:**

1. **Malicious JS in config file** (High likelihood, High impact)
   - Existing risk, not introduced by this project
   - Mitigated by code review, gitignore

2. **Pointing to destructive binary** (Low likelihood, Medium impact)
   - Requires developer error or malicious intent
   - Mitigated by trust model (developer controls environment)

3. **Path injection** (Very low likelihood, Low impact)
   - Prevented by spawnSync not using shell
   - Mitigated by existing safeguards

4. **Binary tampering** (Very low likelihood, High impact)
   - Requires filesystem access (game over anyway)
   - Out of scope for local development tool

### Trust Boundaries

**Trusted:**
- Developer's local machine
- Config files on local filesystem
- Binaries compiled from source (local Rust builds)

**Untrusted:**
- (None in this project's scope)

**Trust assumption:** Developer controls their own environment and config files.

## Recommendations

### For MVP

1. **Documentation update** - Add to local-development.md:
   ```markdown
   **Security Note:** Never commit crewchief.config.local.js to version control.
   Use .local.js for machine-specific settings like custom binary paths.
   ```

2. **No code changes needed** - Existing validation sufficient

3. **No additional validation** - Would be security theater given arbitrary JS execution

### For Future

**If expanding beyond local development:**

1. Implement binary allowlist for CI/CD environments
2. Add checksum verification for downloaded binaries
3. Consider read-only config format (TOML/YAML) instead of executable JavaScript
4. Audit logging for binary path changes in multi-user setups

**None applicable to current scope.**

## Compliance

**Not applicable.** This is a developer tool, not subject to:
- GDPR (no personal data)
- PCI-DSS (no payment data)
- HIPAA (no health data)
- SOC 2 (not a service)

## Conclusion

**Security posture: ACCEPTABLE**

This project introduces **no new security risks** beyond the existing accepted risk of JavaScript config files. The threat model is appropriate for a local development tool where developers control their own environment.

**No security blockers for MVP.**

**Recommendation: Proceed with implementation.**
