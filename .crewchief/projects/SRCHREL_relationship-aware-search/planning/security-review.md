# Security Review: Relationship-Aware Search Ranking

## Security Assessment

**Risk Level:** LOW

This feature enhances internal search ranking logic. No external API changes, no new authentication requirements, no sensitive data exposure. All changes are server-side Rust code.

## Security Analysis

### Authentication & Authorization

**Current State:** Search is performed within user's repository access context (existing auth).

**This Feature:** NO CHANGES to authentication or authorization.

**Risk:** NONE
- No new endpoints
- No permission model changes
- Same access control as baseline search

### Data Protection

**Current State:** Search accesses code chunks and edges in database (user's own repositories).

**This Feature:** NO CHANGES to data access patterns.

**Sensitive Data Handling:**
- Edge quality weights: NOT sensitive (configuration values)
- Graph scores: NOT sensitive (derived from public code structure)
- Search queries: Already logged and secured

**Risk:** NONE
- No new PII or sensitive data processed
- No data exfiltration vectors introduced

### Input Validation

**Current State:** Search queries validated by existing pipeline.

**This Feature:** Configuration file inputs (YAML weights).

**Validation Strategy:**
```rust
impl EdgeQualityWeights {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate weight ranges (prevent negative/extreme values)
        if self.production_code <= 0.0 || self.production_code > 10.0 {
            return Err(ConfigError::InvalidWeight("production_code must be 0.0-10.0"));
        }
        // ... validate all weights
        Ok(())
    }
}
```

**Risk:** LOW
- Configuration loaded at startup (not runtime user input)
- Validation prevents negative/extreme weights
- Invalid config fails fast (service won't start)

**Mitigation:** Configuration validation with clear error messages.

### SQL Injection

**Current State:** Parameterized queries prevent SQL injection.

**This Feature:** Enhanced SQL query with additional parameters.

**Query Pattern:**
```rust
let query = r#"
    SELECT ... WHERE ce.type = ?1 AND src_chunk.kind LIKE '%test%'
"#;

let params = params![
    weights.extends,       // f32 parameter (safe)
    weights.implements,    // f32 parameter (safe)
    // ... all params are typed
];
```

**Risk:** NONE
- All parameters are typed (f32, i64, not strings)
- No string concatenation in SQL
- Uses rusqlite parameterized queries (safe)

**Mitigation:** Parameterized queries (already in place).

### Denial of Service (DoS)

**Current State:** Search has timeouts and rate limits.

**This Feature:** Slightly more complex SQL query.

**Attack Vectors:**
1. **Malicious weights:** User sets weights to extreme values
   - **Mitigation:** Validation rejects values outside 0.0-10.0 range
2. **Performance attack:** Trigger slow queries
   - **Mitigation:** Query timeout (existing), feature flag rollback
3. **Configuration parsing attack:** Malformed YAML
   - **Mitigation:** YAML parser errors fail service startup (not runtime)

**Risk:** LOW
- Query complexity increase is minimal (+8ms)
- Timeout mechanisms already in place
- Feature flag allows instant rollback

**Mitigation:** Query timeouts, configuration validation, feature flag.

### Error Handling & Information Leakage

**Current State:** Errors logged server-side, generic messages to client.

**This Feature:** Configuration errors, query errors.

**Error Handling:**
```rust
// Configuration error (startup, not runtime)
Err(ConfigError::InvalidWeight(field)) => {
    error!("Invalid weight for {}: {}", field, value);
    // Service fails to start (safe failure mode)
}

// Query error (runtime)
Err(GraphError::Database(e)) => {
    warn!("Graph query failed: {}", e);
    // Return empty results, continue search with other executors
}
```

**Risk:** NONE
- Configuration errors: Logged server-side only, fail startup
- Query errors: Logged with context, generic error to client
- No database schema or internal details leaked

**Mitigation:** Generic error messages to client, detailed logs server-side only.

## Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| Configuration file readable by server process | Low | Standard file permissions (only service user) | Accepted |
| No audit log for configuration changes | Low | Configuration in version control (git audit trail) | Accepted |
| Feature flag changes not logged | Low | Could add logging in Phase 2 if needed | Accepted |
| No rate limiting on graph executor specifically | Low | Overall search rate limit applies | Accepted |
| Test detection heuristic could be gamed | Very Low | Heuristic is optimization, not security boundary | Accepted |

## MVP Security Scope

### In Scope
✅ Configuration validation (prevent invalid weights)
✅ SQL injection prevention (parameterized queries)
✅ Error handling (no information leakage)
✅ DoS prevention (timeouts, feature flag rollback)

### Out of Scope (Not Security Issues)
- Audit logging for configuration changes (use git)
- Hot config reload authentication (Phase 2 feature)
- Cross-repository graph traversal (not in MVP)

### Deferred to Future
- Configuration encryption at rest (not needed, weights not sensitive)
- Advanced rate limiting per executor (overall limit sufficient)

## Security Checklist

Before deployment:

### Code Security
- [x] No hardcoded secrets (weights are configuration, not secrets)
- [x] Input validation on configuration (ranges, types)
- [x] Proper error handling (generic messages to client)
- [x] Dependencies up to date (`cargo audit` clean)
- [x] No SQL injection vulnerabilities (parameterized queries)
- [x] N/A: No XSS vulnerabilities (server-side only, no HTML)

### Deployment Security
- [ ] Configuration file permissions set (service user read-only)
- [ ] Feature flag default documented (rollback plan)
- [ ] Monitoring alerts configured (performance, errors)
- [ ] Rollback procedure documented and tested (see plan.md)
- [ ] Rollback drill executed in staging (verify <5 minute rollback time)

### Operational Security
- [ ] Configuration changes reviewed (pull request process)
- [ ] Production config access restricted (ops team only)
- [ ] Logs configured (structured logging with context)
- [ ] Incident response plan (rollback via feature flag)

## Threat Model

### Threat: Malicious Configuration

**Scenario:** Attacker modifies config file to DoS search.

**Attack Vector:**
1. Gain write access to `maproom-search.yml`
2. Set extreme weights (e.g., `production_code: 1000000`)
3. Service restarts, loads malicious config

**Impact:**
- Graph scores extremely skewed
- Potential performance degradation

**Likelihood:** Very Low (requires server file system access)

**Mitigation:**
- **Prevention:** File permissions (service user read-only)
- **Detection:** Configuration validation rejects extreme values
- **Response:** Service fails to start with validation error

**Residual Risk:** ACCEPTED (requires server compromise, detected immediately)

### Threat: SQL Performance Attack

**Scenario:** Trigger slow graph queries to DoS search.

**Attack Vector:**
1. Craft query that maximizes graph executor work
2. Send repeatedly to overload database

**Impact:**
- Slow search responses
- Potential timeout errors

**Likelihood:** Low (existing rate limits apply)

**Mitigation:**
- **Prevention:** Query timeout (existing), rate limiting
- **Detection:** Latency monitoring alerts
- **Response:** Feature flag rollback (set enable_quality_scoring=false)

**Residual Risk:** ACCEPTED (mitigated by existing defenses)

### Threat: Configuration Secret Exposure

**Scenario:** Edge quality weights leaked.

**Attack Vector:**
1. Read configuration file from server
2. Extract weights

**Impact:**
- NONE (weights are not secrets, publicly documented defaults)

**Likelihood:** N/A

**Mitigation:** Not needed (weights are intentionally tunable, not sensitive)

**Residual Risk:** NONE (not a security issue)

## Compliance

**GDPR/Privacy:** N/A (no PII processed)
**SOC 2:** No impact (internal optimization)
**ISO 27001:** Configuration change management via git (existing process)

## Security Sign-Off

This feature introduces **no meaningful security risk**. All changes are server-side optimizations to existing search ranking logic. No new attack surfaces, no sensitive data exposure, no authentication changes.

**Risk Assessment:** LOW
**Security Review:** APPROVED for MVP
**Additional Requirements:** Standard configuration file permissions, feature flag rollback plan
