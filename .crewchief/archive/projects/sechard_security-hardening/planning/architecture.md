# Architecture: Security Hardening

## Architecture Decisions
- **Dependency Updates**: Primary mechanism is updating versions in `Cargo.toml` and `package.json`.
- **Replacements**: If a dependency is abandoned and vulnerable, replace it with a maintained alternative.

## Technology Choices
- **Cargo Audit**: For Rust vulnerability scanning.
- **NPM Audit**: For Node.js vulnerability scanning.
- **Dependabot/Renovate**: Consider configuring for future automation (out of scope for this specific project, but good for roadmap).

## Constraints
- Updates must not break the build or existing functionality.
