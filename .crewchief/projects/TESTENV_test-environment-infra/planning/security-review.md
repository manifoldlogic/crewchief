# Security Review: Test Environment Infrastructure

## Scope

This security review covers:
1. SQL test fixtures and their loading mechanism
2. Dockerized Rust daemon service
3. Test database configuration

## Risk Assessment

### Overall Risk Level: **LOW**

This is test infrastructure that:
- Runs only in development/CI environments
- Contains no production data
- Has no external network exposure
- Uses well-established patterns (Docker, PostgreSQL)

## Security Analysis by Component

### 1. SQL Test Fixtures

**Assets**: Pre-indexed test data, SQL scripts

**Threats**:
| Threat | Likelihood | Impact | Risk |
|--------|------------|--------|------|
| SQL injection in fixture loading | Very Low | Medium | Low |
| Sensitive data in fixtures | Very Low | Low | Negligible |
| Fixture tampering | Very Low | Low | Negligible |

**Mitigations**:
- Fixtures loaded via `psql` directly, not dynamic SQL
- Fixtures contain synthetic test data only
- Fixtures are version-controlled and reviewed

**Recommendation**: No additional security measures needed.

### 2. Dockerized Daemon

**Assets**: Rust binary, database credentials, container

**Threats**:
| Threat | Likelihood | Impact | Risk |
|--------|------------|--------|------|
| Container escape | Very Low | High | Low |
| Credential exposure | Low | Medium | Low |
| Denial of service | Low | Low | Negligible |
| Supply chain attack | Very Low | High | Low |

**Mitigations**:
- Container runs with default (non-root) user
- Database credentials are test-only (no production access)
- Container is profile-gated (not started by default)
- Multi-stage build reduces attack surface
- No external network exposure (Docker network only)

**Recommendations**:
1. Use non-root user in Dockerfile
2. Don't hardcode credentials (use environment variables)
3. Pin base image versions

### 3. Test Database

**Assets**: PostgreSQL test instance, test data

**Threats**:
| Threat | Likelihood | Impact | Risk |
|--------|------------|--------|------|
| Data exfiltration | Very Low | Low | Negligible |
| Unauthorized access | Low | Low | Negligible |
| Resource exhaustion | Low | Low | Negligible |

**Mitigations**:
- Test database is isolated from production
- Credentials are well-known (test-only: `maproom:maproom`)
- No sensitive data stored
- Container networking isolates from external access

**Recommendation**: Current setup is appropriate for test environment.

## Credential Management

### Current Credentials (Test-Only)

```
Database: maproom_test
User: maproom
Password: maproom
Host: postgres-test:5432 (internal) or host.docker.internal:5434 (external)
```

**Assessment**: These credentials are intentionally simple for test environments. They provide no access to production systems.

**NOT Required for MVP**:
- Secrets management (test credentials are public)
- Credential rotation
- Encrypted connections (internal Docker network)

## Docker Security Checklist

```dockerfile
# crates/maproom/Dockerfile - Security considerations

# ✓ Use specific base image version (not :latest)
FROM rust:1.75-slim as builder

# ✓ Multi-stage build (smaller attack surface)
FROM debian:bookworm-slim

# ✓ Install only required packages
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# ✓ Run as non-root user
RUN useradd -r -s /bin/false maproom
USER maproom

# ✓ Copy only necessary files
COPY --from=builder /app/target/release/crewchief-maproom /usr/local/bin/
```

## Network Security

### Internal Network Topology

```
┌─────────────────────────────────────────┐
│        Docker Network (isolated)         │
│                                          │
│   postgres-test ◄──── maproom-daemon    │
│       :5432              :8080           │
│                                          │
│   No external exposure                   │
└─────────────────────────────────────────┘
          ▲
          │ Port mapping (dev only)
          │ host.docker.internal:5434
          ▼
┌─────────────────────────────────────────┐
│        Host (DevContainer)               │
│                                          │
│   Vitest tests connect via mapped port  │
└─────────────────────────────────────────┘
```

**Assessment**: Network isolation is appropriate. Test services are not exposed externally.

## CI/CD Security

### GitHub Actions Considerations

- No production credentials in CI
- Test database is ephemeral (destroyed after job)
- Docker images built fresh (not pulled from registry for MVP)

**Future consideration**: If daemon image is published to registry, implement:
- Image signing
- Vulnerability scanning
- SBOM generation

## Security Gaps (Accepted for MVP)

| Gap | Risk | Reason for Acceptance |
|-----|------|----------------------|
| No TLS for test DB | Low | Internal network only |
| Simple credentials | Negligible | Test environment only |
| No audit logging | Negligible | Test environment only |
| No rate limiting | Negligible | Test environment only |

## Recommendations Summary

### Must Have (Blocking)
- None for MVP (test infrastructure only)

### Should Have (Non-blocking)
1. Non-root user in daemon Dockerfile
2. Pin base image versions
3. Document that test credentials must never be used in production

### Nice to Have (Future)
1. Image vulnerability scanning in CI
2. Signed container images if published
3. Network policies if running in Kubernetes

## Sign-off

**Risk Level**: Low
**Recommendation**: Proceed with implementation
**Conditions**: Follow Dockerfile security checklist
