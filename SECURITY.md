# Security Policy

## Supported Versions

Only the latest version receives security updates:

| Package | Supported |
|---------|-----------|
| @crewchief/cli | Latest only |
| @crewchief/maproom-mcp | Latest only |

## Reporting a Vulnerability

**Please do NOT open public issues for security vulnerabilities.**

Instead, report vulnerabilities by emailing: security@danielbushman.com

We will respond within 48 hours and provide a fix timeline.

### What to include in your report:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Our commitment:
- We will acknowledge receipt within 48 hours
- We will provide a fix timeline within 1 week
- We will credit you in the security advisory (unless you prefer anonymity)

## Security Measures

### Build and Release Security
- npm packages published from GitHub Actions only
- Binaries built on GitHub-hosted runners
- NPM_TOKEN stored as encrypted GitHub secret
- Tag protection prevents unauthorized releases
- Binary validation before publish

### Dependency Security

**Latest Security Audit:** 2025-11-21 (SECHARD Project)

**npm Dependencies:**
- Tool: `pnpm audit`
- Result: ✅ **0 vulnerabilities**
- Last Run: 2025-11-21
- Fixes Applied: 15 vulnerabilities resolved via pnpm overrides
  - Critical: 3 (glob command injection, happy-dom RCE)
  - High: 2 (vite middleware issues)
  - Moderate: 4 (js-yaml prototype pollution, esbuild SSRF)
  - Low: 3 (tmp symlink)

**Rust Dependencies:**
- Tool: `cargo-audit v0.22.0`
- Result: ⚠️ **1 accepted risk**, 3 warnings (unmaintained)
- Last Run: 2025-11-21
- Fixed: 1 (protobuf via prometheus 0.13→0.14)
- Accepted Risk: ring v0.17.9 AES issue (transitive, update blocked)

**Audit Documentation:**
- Full details: [SECURITY-AUDIT.md](./SECURITY-AUDIT.md)
- Accepted risks documented with justification
- Next audit: 2026-02-21 (quarterly)

**Security Practices:**
- pnpm lock file ensures reproducible builds
- Regular dependency updates (monthly)
- Automated audits before releases
- Security overrides for critical fixes:
  ```json
  {
    "pnpm": {
      "overrides": {
        "glob": "^11.1.0",
        "vite": "^5.4.20",
        "js-yaml": "^4.1.1",
        "tmp": "^0.2.4",
        "happy-dom": "^20.0.2",
        "esbuild": "^0.25.0"
      }
    }
  }
  ```

## Scope

In scope:
- crewchief CLI tool
- maproom-mcp MCP server
- GitHub Actions workflows
- Build and release automation

Out of scope:
- Third-party dependencies (report to upstream)
- Social engineering attacks
- Physical security
