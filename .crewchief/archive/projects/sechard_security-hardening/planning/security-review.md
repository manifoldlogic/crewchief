# Security Review: Security Hardening

## Security Assessment
- This project is explicitly about improving security.
- **Goal**: Eliminate High/Critical vulnerabilities.

## Gaps & Risks
- **Transitive Dependencies**: Sometimes a vulnerability is deep in the tree and requires `cargo update -p` or `npm dedupe` tricks, or waiting for upstream fixes.

## Mitigations
- **Overrides/Patching**: Use `[patch.crates-io]` in Rust or `overrides` in `package.json` if necessary to force a fix.
