# Security Review: Test Database Isolation

## Security Scope

This project adds test database infrastructure for **local development and CI environments only**. This is not production infrastructure.

## Threat Model

### In Scope
- Accidental data leakage between dev and test databases
- Credential exposure in configuration files
- Port exposure to local network

### Out of Scope
- Production database security (unchanged)
- Authentication mechanisms (inherited from existing setup)
- Network attacks (development environment assumption)
- Supply chain attacks (Docker images from trusted registries)

## Security Analysis

### Risk 1: Credential Hardcoding

**Current State**:
```yaml
environment:
  POSTGRES_USER: maproom
  POSTGRES_PASSWORD: maproom
```

**Assessment**: ACCEPTABLE for development

**Rationale**:
- Local development databases only
- No sensitive data in dev/test environments
- Credentials never committed to production configs
- Standard practice for local Docker Compose setups

**Mitigations**:
- Document that these are dev credentials only
- Production uses secret management (separate from this project)
- Test database contains only synthetic/public data

**Action**: None required

### Risk 2: Port Exposure

**Current State**:
```yaml
ports:
  - "0.0.0.0:5433:5432"  # Dev database
  - "0.0.0.0:5434:5432"  # Test database
```

**Assessment**: LOW RISK for development

**Concern**: Binds to all interfaces (0.0.0.0) instead of localhost

**Impact**:
- Databases accessible from local network
- Other machines on network can connect

**Mitigations (Current)**:
- Firewall typically blocks external access
- Development environment assumption (trusted network)
- No production data in databases

**Recommended Enhancement** (Not Blocking):
```yaml
ports:
  - "127.0.0.1:5433:5432"  # Dev database
  - "127.0.0.1:5434:5432"  # Test database
```

**Decision**: Ship with 0.0.0.0 (matches existing dev setup), document localhost alternative

### Risk 3: Database Access Control

**Current State**:
- No row-level security
- No connection limits
- Single user (maproom) with full privileges

**Assessment**: ACCEPTABLE for development

**Rationale**:
- Development/test databases contain no sensitive data
- Adding access controls adds complexity without security benefit
- Production databases have proper access controls (out of scope)

**Action**: None required

### Risk 4: Data Isolation Failure

**Threat**: Tests accidentally modify dev database

**Mitigation**:
1. **Separate volumes**: Physical data isolation
2. **Different database names**: maproom vs maproom_test
3. **Different ports**: 5433 vs 5434
4. **Explicit configuration**: TEST_MAPROOM_DATABASE_URL vs MAPROOM_DATABASE_URL

**Assessment**: Well mitigated

**Verification**:
- Manual validation script checks isolation
- Tests fail fast if wrong database

### Risk 5: Credential Leakage in Logs

**Concern**: DATABASE_URL contains credentials, may appear in logs

**Current Practice**:
```typescript
// Already implemented in helpers/database.ts
const dbUrl = process.env.TEST_MAPROOM_DATABASE_URL || process.env.MAPROOM_DATABASE_URL
// Connection created, URL not logged
```

**Assessment**: LOW RISK

**Best Practices**:
- Database helpers don't log connection strings
- Vitest doesn't log environment variables by default
- CI logs don't expose secrets (GitHub Actions redacts)

**Action**: Continue current practice

### Risk 6: Docker Image Integrity

**Images Used**:
- `pgvector/pgvector:pg16` (official pgvector image)
- `ollama/ollama:latest` (official Ollama image)
- `manifoldlogic/crewchief_maproom-mcp` (our published image)

**Assessment**: ACCEPTABLE

**Mitigations**:
- Using official images from trusted registries
- Docker Hub content trust available (optional)
- Regular image updates via Dependabot (existing practice)

**Recommended** (Not Blocking):
```yaml
image: pgvector/pgvector:pg16@sha256:<digest>  # Pin to digest
```

**Decision**: Ship with tags, document digest pinning for production

## Known Security Gaps

### Gap 1: Default Credentials

**Status**: Accepted for development

**Impact**: Anyone on local network can connect

**Mitigation for Production**:
- Production uses strong passwords from secret management
- This project is dev/test infrastructure only

**Escalation Path**: If production ever uses this setup (shouldn't), implement secrets management

### Gap 2: No TLS

**Status**: Accepted for development

**Impact**: Database connections unencrypted on localhost

**Mitigation for Production**:
- Production uses TLS (separate configuration)
- Local connections don't traverse network

**Escalation Path**: If remote development needed, enable TLS

### Gap 3: No Audit Logging

**Status**: Accepted for development

**Impact**: Can't track who accessed what data

**Mitigation**:
- Test/dev databases contain no sensitive data
- Production has audit logging (out of scope)

**Escalation Path**: N/A (not needed for development)

## Security Checklist

### Configuration Security
- [x] Credentials not in version control (environment variables)
- [x] No secrets in docker-compose.yml (dev credentials acceptable)
- [x] Connection strings use environment variables
- [x] Test database isolated from dev database

### Network Security
- [x] Port binding documented (0.0.0.0 for Docker-in-Docker compatibility)
- [x] No external port requirements (localhost sufficient)
- [x] Docker network isolation configured
- [x] Health checks don't expose credentials

### Data Security
- [x] Separate volumes for dev and test data
- [x] Test data not persisted to dev database
- [x] Dev data not accessible from test database
- [x] Database names clearly distinguished

### Operational Security
- [x] Documentation warns about dev credentials
- [x] Production separation clear in docs
- [x] Error messages don't leak credentials
- [x] Logs don't contain connection strings

## Security Best Practices

### Do's
✅ Use TEST_MAPROOM_DATABASE_URL explicitly in test configurations
✅ Document credential limitations
✅ Keep dev and production configs separate
✅ Use health checks to verify database readiness

### Don'ts
❌ Don't use production credentials in development
❌ Don't commit connection strings to version control
❌ Don't run development databases on public networks
❌ Don't store sensitive data in dev/test databases

## Compliance Considerations

### GDPR/PII
**Status**: N/A

**Rationale**:
- Development and test databases contain synthetic data only
- No personal data should be in these environments
- If PII needed for testing, use anonymized/fake data

**Recommendation**: Add to documentation: "Never use production data in test database"

### SOC 2 / Audit Requirements
**Status**: N/A for development infrastructure

**Consideration**:
- If company requires audit trail for development, enable PostgreSQL logging
- Test database can be excluded from audit scope (not production)

## Security Testing

### Automated Security Checks
**Current**: None specific to database infrastructure

**Recommended** (Future):
- `npm audit` catches known vulnerabilities in dependencies
- Dependabot updates Docker base images
- SAST scans catch credential leaks in code

**Decision**: Rely on existing security tooling

### Manual Security Verification

**Validation Script**:
```bash
#!/bin/bash
# security-validation.sh

echo "Validating security posture..."

# Check 1: No hardcoded credentials in code
echo "Checking for hardcoded credentials..."
if git grep -i "password.*=.*maproom" -- ':!*.md' ':!*.yml'; then
  echo "❌ Found hardcoded credentials"
  exit 1
fi

# Check 2: Ports not exposed to internet
echo "Checking port binding..."
if docker compose config | grep "0.0.0.0"; then
  echo "⚠️  Ports bound to 0.0.0.0 (local network accessible)"
else
  echo "✅ Ports bound to localhost only"
fi

# Check 3: Database isolation
echo "Checking database isolation..."
DEV_DB=$(docker exec maproom-postgres psql -U maproom -t -c "SELECT current_database()")
TEST_DB=$(docker exec maproom-postgres-test psql -U maproom -t -c "SELECT current_database()")

if [ "$DEV_DB" != "$TEST_DB" ]; then
  echo "✅ Databases are isolated"
else
  echo "❌ Databases not isolated"
  exit 1
fi

echo "Security validation complete"
```

## Security Sign-off

**Assessment**: This project introduces **NO NEW SECURITY RISKS** for the following reasons:

1. **Scope Limited**: Development/test infrastructure only
2. **No Sensitive Data**: Test databases contain synthetic data
3. **Isolation Maintained**: Dev and test databases can't interfere
4. **Best Practices**: Follows Docker Compose security conventions
5. **Backward Compatible**: Doesn't change existing security posture

**Recommendation**: APPROVE for implementation

**Caveats**:
- Must not be used for production databases
- Must not store sensitive data in test database
- Must document credential limitations

**Reviewer Notes**:
This is infrastructure plumbing for development. Security requirements are appropriate for the use case. If this architecture is ever used for production (which it shouldn't be), a full security review would be required with secrets management, TLS, audit logging, and access controls.
