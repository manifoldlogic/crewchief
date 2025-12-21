# Security Review: Repository Public Readiness

## Security Assessment

### Overview

Before making a repository public, the primary security concern is **information disclosure** - ensuring no credentials, API keys, internal paths, or sensitive business information is exposed. This review covers the current state and remediation plan.

### Threat Model

| Threat | Likelihood | Impact | Notes |
|--------|------------|--------|-------|
| Exposed API keys/tokens | Low | Critical | Repository has good hygiene, but grep scan needed |
| Exposed credentials | Low | Critical | No .env with values found; examples are safe |
| Internal paths exposed | Medium | Low | MCP configs may have local paths |
| Sensitive business data | Low | Medium | Project archives may contain internal context |
| Security vulnerabilities in deps | Medium | Medium | Standard open source risk |

## Authentication & Authorization

**Not applicable** - This is a CLI tool, not a service with auth. However, the codebase does handle:

1. **Embedding provider API keys** - Users configure these; code doesn't store them
2. **Git credentials** - Relies on system git credential handling
3. **VSCode secrets** - Extension uses VSCode's secure storage

**Assessment:** No auth secrets should be in the repository. Verified:
- `.env` file exists but is empty
- `.env.example` files contain only placeholder values

## Data Protection

### Sensitive Data Categories

1. **API Keys/Tokens**
   - Found in: Test files (mock values), documentation (examples)
   - Action: Grep scan to verify no real keys

2. **Internal Paths**
   - Found in: `.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`
   - Risk: May expose local filesystem structure
   - Action: Add to `.gitignore`, remove from repo

3. **User-Generated Content**
   - Found in: `.crewchief/archive/projects/`
   - Risk: Internal project names, possibly sensitive context
   - Action: Remove archive from main branch

4. **Genetic Optimization Data**
   - Found in: `packages/cli/.crewchief/genetic-iterations/`
   - Risk: May contain internal data from optimization runs
   - Action: Remove entirely

### Grep Patterns for Secrets

The following patterns were searched:

```bash
# High-priority patterns
SECRET|PASSWORD|API_KEY|TOKEN|PRIVATE_KEY

# Found 1006 files with matches
# Most are legitimate code references (variable names, test mocks)
# Manual audit required for:
# - Non-test .json files
# - .env files
# - Configuration files
```

**Findings from initial scan:**
- `packages/vscode-maproom/src/config/secrets.ts` - Handles secrets securely (uses VSCode SecretStorage)
- Test files with `auth-service.ts`, `validate_token.py` - Mock implementations
- Configuration examples with placeholder values

## Input Validation

**Not directly applicable to cleanup**, but noted for completeness:
- Maproom CLI validates file paths
- MCP server validates tool inputs
- No user-provided data stored in repository

## Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| `.mcp.json` in repo | Low | Add to .gitignore, remove | Open |
| Empty `.env` file | Low | Remove from repo | Open |
| `.cursor/mcp.json` | Low | Already gitignored | OK |
| Project archive content | Low | Move to archive branch | Open |
| Genetic run data | Low | Delete from repo | Open |

## Initial Release Security Scope

### Must Complete Before Public Release

1. **Remove `.env` file** - Even empty, signals poor practice
2. **Remove `.mcp.json`** - May contain local paths
3. **Add sensitive paths to `.gitignore`**:
   ```gitignore
   .env
   !.env.example
   .mcp.json
   **/google-credentials.json
   **/maproom-sa-key.json
   ```
4. **Remove genetic iteration data** - May contain internal context
5. **Archive project history** - Move to separate branch

### Post-Release Security Maintenance

1. **Dependency updates** - Regular `pnpm audit` and `cargo audit`
2. **Secret scanning** - Enable GitHub secret scanning
3. **Contributor guidance** - Document in CONTRIBUTING.md

## Security Checklist

### Pre-Public-Release

- [ ] No hardcoded secrets (API keys, passwords, tokens)
- [ ] No `.env` files with values in repo
- [ ] No internal paths in committed configs
- [ ] No Google/cloud credentials files
- [ ] `.gitignore` updated with sensitive patterns
- [ ] Project archives moved to separate branch
- [ ] Genetic optimization data removed

### Verification Commands

```bash
# Search for potential secrets
grep -rn "sk-" --include="*.ts" --include="*.js" --include="*.json" .
grep -rn "AKIA" .  # AWS access key pattern
grep -rn "ghp_" .  # GitHub token pattern
grep -rn "AIza" .  # Google API key pattern

# Check for credential files
find . -name "*credentials*.json" -o -name "*secret*.json" -o -name "*key*.json"

# Verify .env files
find . -name ".env" -exec cat {} \; -print
```

### Security Scan Results

**Initial scan (to be updated after remediation):**

| Check | Status | Notes |
|-------|--------|-------|
| Hardcoded API keys | PASS | No real keys found |
| Credential files | PASS | Examples only |
| .env with values | PASS | Empty file (should remove) |
| Internal paths | WARN | MCP configs have local paths |
| Dependency vulnerabilities | TBD | Run audit post-cleanup |

## Recommendations

### Immediate Actions

1. **Add GitHub Secret Scanning** - Enable on repository settings
2. **Add Dependabot** - Automated dependency updates
3. **Pre-commit hooks** - Consider adding secret detection

### Long-term Improvements

1. **SECURITY.md** - Add security policy file for responsible disclosure
2. **CI secret scanning** - Add to GitHub Actions workflow
3. **Regular audits** - Quarterly security review

## Sign-off

Security review completed: [Date to be filled]
Reviewed by: [Agent/Human to be filled]
Findings addressed: [ ] Yes / [ ] Partial / [ ] No
