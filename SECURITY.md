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

**npm Dependencies:**
- Tool: `pnpm audit`
- Known vulnerabilities resolved via pnpm overrides

**Rust Dependencies:**
- Tool: `cargo-audit`
- Regular audits performed before releases

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
