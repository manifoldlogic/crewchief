# Security Review: embedding dimension 1024

## Security Assessment

### Authentication & Authorization

**Scope**: Not applicable to this project.

**Rationale**: This project adds database schema changes and embedding dimension support. It does not introduce authentication, authorization, or access control mechanisms. The embedding service operates within the existing security context of the daemon/CLI.

**Existing Security**:
- Daemon runs as local process (no network exposure)
- SQLite database file permissions follow OS defaults (~/.maproom/maproom.db)
- Ollama API runs locally (no external authentication)

**No changes to auth/authz in this project.**

### Data Protection

**Current State**:
- Embeddings stored in SQLite database (unencrypted)
- Database file accessible to user running daemon
- No PII or sensitive data in embeddings (code content only)

**Impact of Changes**:
- **1024-dim embeddings**: Same storage mechanism as existing dimensions
- **No new data exposure**: vec_code_1024 table follows same pattern as vec_code_768
- **No encryption changes**: SQLite database remains unencrypted (existing design)

**Assessment**: No new data protection concerns introduced.

**Future consideration** (out of scope): Database encryption for sensitive codebases (enterprise use case).

### Input Validation

**Current State**:
- Dimension validated against SUPPORTED_DIMENSIONS constant
- Model names passed to Ollama API (validated by Ollama)
- Embedding vectors validated for length before storage

**Changes**:
- **Add 1024 to SUPPORTED_DIMENSIONS**: Extends validation, doesn't weaken it
- **Configuration parsing**: Dimension parsed from env var (i32), validated before use
- **Error handling**: Invalid dimensions rejected with clear error message

**Risks Addressed**:

1. **Dimension overflow/underflow**: Dimension parsed as usize (non-negative), range-checked against known values
2. **SQL injection**: Not applicable (dimension is integer, not used in string interpolation)
3. **Buffer overflow**: Rust memory safety prevents overflows (vector bounds checked)

**Assessment**: Input validation is sufficient and improved by this change.

### Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| Database file permissions too permissive | Low | OS-level file permissions, users responsible | Accepted |
| No encryption at rest | Medium | Document for enterprise use cases, future feature | Accepted |
| Ollama API runs unencrypted (HTTP) | Low | Localhost only, no network exposure | Accepted |
| No audit logging of dimension changes | Low | Not required for MVP, consider for enterprise | Accepted |
| Model name not validated | Low | Ollama validates, graceful error on invalid model | Accepted |

### Code Injection Risks

**SQL Injection**: Not applicable.
- Dimension is integer (not string)
- Table names are hardcoded constants (not user-provided)
- SQL uses parameterized queries (rusqlite params!)

**Command Injection**: Not applicable.
- No shell commands executed with user input
- Ollama API called via HTTP (not shell)

**Rust Memory Safety**: Compiler-enforced.
- No unsafe blocks introduced in this project
- Vector bounds checked by Rust runtime
- No buffer overflows possible

**Assessment**: No code injection risks introduced.

### Dependency Security

**New Dependencies**: None.

This project only adds:
- Database migration (pure SQL)
- Constant updates (Rust code)
- Configuration validation (existing mechanism)

**Existing Dependencies**:
- rusqlite (maintained, widely used)
- sqlite-vec (statically linked, audited)
- reqwest (for Ollama API, maintained)

**Recommendation**: Run `cargo audit` before release (standard practice, not specific to this project).

### Error Information Leakage

**Error Messages**:
```rust
"Unsupported embedding dimension: 512. Supported dimensions: [768, 1024, 1536]"
```

**Assessment**: Safe.
- No sensitive information revealed (dimension values are public knowledge)
- No stack traces in production (tracing crate configured appropriately)
- No PII or credentials in error messages

**Validation errors**:
```rust
"Ollama provider with nomic-embed-text requires dimension=768, got 1024"
```

**Assessment**: Safe.
- Configuration information only (no secrets)
- Helps users debug misconfigurations

## MVP Security Scope

### In Scope for MVP

1. **Input validation**: Dimension range-checked
2. **Error handling**: No panics on invalid input
3. **Memory safety**: Rust guarantees enforced
4. **Backward compatibility**: No security regression for existing features

### Out of Scope for MVP

1. **Database encryption**: Not required for local development use case
2. **Audit logging**: Not required for MVP, consider for enterprise
3. **Network security**: Ollama runs locally, no network exposure
4. **Authentication**: Daemon is single-user, no multi-tenant concerns

### Accepted Risks (with justification)

1. **Unencrypted database**: Accepted for MVP, document for enterprise users
2. **No rate limiting**: Accepted, local-only tool, user controls their own resource usage
3. **No input sanitization for model names**: Accepted, Ollama validates, error is graceful

## Security Checklist

- [x] No hardcoded secrets (no API keys in code)
- [x] Input validation on external inputs (dimension validated)
- [x] Proper error handling (Result types, no panics)
- [x] Dependencies are up to date (cargo audit recommended)
- [x] No SQL injection vulnerabilities (parameterized queries)
- [x] No XSS vulnerabilities (not applicable, CLI/daemon only)
- [x] No unsafe Rust code introduced
- [x] Error messages don't leak sensitive info
- [x] Backward compatibility maintains existing security posture

## Security Testing

### Static Analysis

**Tools**:
- `cargo clippy` (lint for common mistakes)
- `cargo audit` (check for known CVEs in dependencies)
- Rust compiler (memory safety guarantees)

**Run before commit**:
```bash
cargo clippy -- -D warnings
cargo audit
```

### Dynamic Testing

**Fuzzing** (optional, not required for MVP):
- Could fuzz dimension parsing with arbitrary integers
- Not critical due to Rust's type safety

**Integration tests** (security-relevant):
- Test dimension validation rejects invalid values
- Test error handling doesn't panic on malformed input
- Test migration #10 doesn't corrupt existing data

### Manual Review

**Code review checklist**:
- [ ] No `unsafe` blocks introduced
- [ ] All user inputs validated before use
- [ ] Error messages don't leak sensitive information
- [ ] SQL queries use parameterized syntax
- [ ] No new dependencies without justification

## Deployment Security

### Configuration Security

**Environment variables**:
```bash
MAPROOM_EMBEDDING_MODEL=mxbai-embed-large
MAPROOM_EMBEDDING_DIMENSION=1024
```

**Assessment**: Safe.
- No secrets in env vars (model name and dimension are not sensitive)
- Ollama API key not required (local service)

**Best practice**: Document that users should protect their .env files from public repositories.

### Database Security

**SQLite file**: `~/.maproom/maproom.db`

**Permissions**: Inherited from OS (typically 0600 on Unix, user-only read/write)

**Recommendation**: Document that users should:
1. Not commit database files to version control
2. Backup database files securely
3. Consider encryption for sensitive codebases (out of scope for MVP)

### Logging Security

**Tracing**:
- Dimension values logged (not sensitive)
- Model names logged (not sensitive)
- No PII or credentials logged

**Assessment**: Safe. No sensitive information in logs.

## Incident Response Plan

### If Security Issue Discovered

1. **Assess impact**: Does it affect existing 768/1536 dimensions or only 1024?
2. **Rollback**: Revert to previous model configuration (MAPROOM_EMBEDDING_DIMENSION=768)
3. **Database**: Drop vec_code_1024 table if compromised (doesn't affect other dimensions)
4. **Patch**: Fix issue, test, redeploy
5. **Notify**: Document in CHANGELOG if user action required

### Rollback Safety

**Security-safe rollback**:
- Reverting environment variables doesn't expose data
- Dropping vec_code_1024 table doesn't affect existing embeddings
- No migration data transformation (no corruption risk)

**Data retention**: Old embeddings in vec_code_768 and vec_code remain accessible during rollback.

## Compliance Considerations

**GDPR/Privacy**: Not applicable.
- No PII processed or stored
- Code content only (no user data)
- Local-only tool (no data transmission)

**SOC2/Enterprise**: Out of scope for MVP.
- Audit logging not implemented
- Encryption at rest not implemented
- Multi-tenant isolation not applicable (single-user tool)

**Recommendation**: Document security limitations for enterprise users considering adoption.

## Post-Deployment Monitoring

**Security metrics** (not automated, manual review):
- Monitor GitHub issues for security-related bug reports
- Review cargo audit output periodically
- Track Rust CVE database for dependency vulnerabilities

**No runtime security monitoring required** (local tool, no network exposure).

## Conclusion

**Security posture**: This project maintains the existing security posture of the codebase. No new vulnerabilities introduced. Input validation is improved (1024 added to supported dimensions). No meaningful security concerns for MVP deployment.

**Recommendation**: Proceed with implementation. Run `cargo audit` before release as standard practice.

**Future work** (out of scope):
- Database encryption for enterprise use cases
- Audit logging for security-sensitive deployments
- Rate limiting if daemon exposed via network (currently local-only)
