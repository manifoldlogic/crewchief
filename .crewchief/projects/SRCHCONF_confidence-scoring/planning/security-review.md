# Security Review: Confidence Scoring

## Security Assessment

### Overall Risk Level: **LOW**

Confidence scoring is a **transparency feature** that exposes mathematical properties of search results. It processes no user secrets, makes no authorization decisions, and introduces no new attack surface.

**Key Security Properties**:
- No authentication/authorization logic
- No sensitive data processing
- No user-controlled computation
- No database writes
- No external network calls
- No new input vectors

## Authentication & Authorization

### Current State: None Required

**Rationale**: Confidence signals are properties of search results, not sensitive data. Anyone who can execute a search can see confidence scores for their results.

**Access Control**: Follows same model as search itself:
- If user can call MCP search tool → user can see confidence
- If user can call search with `debug: true` → user can see confidence
- No separate permission checks needed for MVP

### Future Considerations (Post-MVP)

**If Authorization Layer Added**:
- Could gate `include_confidence` behind permission (e.g., `user.hasPermission('debug_mode')`)
- Would mirror existing debug mode permission model
- Not needed for MVP—confidence is not sensitive

**Example Future Implementation**:
```typescript
if (params.include_confidence && !user.hasPermission('debug_mode')) {
  throw new PermissionError('Confidence scoring requires debug permission');
}
```

**Decision**: Skip for MVP. Confidence is informational, not sensitive.

## Data Protection

### Sensitive Data Analysis

**What Confidence Contains**:
1. `source_count` - Number of search sources (1-4)
2. `score_gap` - Numeric difference between scores
3. `is_exact_match` - Boolean indicating exact match
4. `relative_score` - Ratio of scores (0.0-1.0)
5. `rank` - Position in result list (1-N)

**Data Classification**: **Public/Non-Sensitive**
- Mathematical properties of search results
- No PII (Personally Identifiable Information)
- No credentials, tokens, or secrets
- No business-sensitive data
- No query history or profiling

**Comparison to Existing Debug Mode**:
- Debug mode exposes `base_fts`, `kind_multiplier`, `exact_match_multiplier`
- Confidence signals are **less detailed** than debug mode
- If debug mode is acceptable, confidence is acceptable

### Data Flow Security

**Data Path**: In-memory only
1. Search pipeline computes scores (already happens)
2. Confidence module reads scores from memory
3. Confidence signals computed (stack-allocated primitives)
4. Serialized to JSON (no sensitive data)
5. Returned to caller (same as search results)

**No Persistence**:
- Confidence not stored in database
- Not logged (except at debug level, same as scores)
- Not cached separately from search results
- Ephemeral data only

**Encryption**: Not needed (no sensitive data, in-memory only)

## Input Validation

### User-Controlled Inputs

**Single Parameter**: `include_confidence: boolean`

**Validation**:
```typescript
// TypeScript (Zod schema)
const SearchParamsSchema = z.object({
  query: z.string().min(1),
  repo: z.string().min(1),
  include_confidence: z.boolean().optional(), // NEW
  // ... other params
});
```

**Rust Validation**:
```rust
pub struct SearchOptions {
    pub include_confidence: bool, // Type system enforces boolean
}
```

**Attack Vectors**: None
- Boolean type prevents injection
- No string parsing, no SQL, no shell commands
- No user-controlled math (all computation on internal data structures)

### Internal Data Validation

**Score Gap Calculation**:
```rust
let score_gap = if index < all_results.len() - 1 {
    result.score - all_results[index + 1].score
} else {
    0.0 // Last result, no gap
};
```
**Safety**: Array bounds checked before access

**Relative Score Calculation**:
```rust
let relative_score = if top_score > 0.0 {
    result.score / top_score
} else {
    0.0 // Avoid division by zero
};
```
**Safety**: Division by zero guarded

**Source Count**:
```rust
let source_count = result.source_scores.len();
```
**Safety**: HashMap.len() cannot fail

### Injection Risks

**SQL Injection**: ❌ Not Applicable
- Confidence computed from in-memory data
- No SQL queries executed
- No database writes

**Command Injection**: ❌ Not Applicable
- No shell commands
- No process spawning
- No file system operations

**Code Injection**: ❌ Not Applicable
- No eval(), no dynamic code execution
- Type-safe Rust computation only
- Serde serialization (safe by design)

**JSON Injection**: ❌ Not Applicable
- Serde handles escaping automatically
- No manual JSON string concatenation
- All values are primitives (numbers, booleans)

## Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| No permission check for confidence | Low | Confidence is non-sensitive, follows search permissions | Accepted for MVP |
| Exact match detection depends on debug mode | Low | Graceful degradation: defaults to false if unavailable | Acceptable |
| No rate limiting on confidence computation | Low | Bounded by search rate limiting (already exists) | Accepted |
| Timing side-channel (score values) | Very Low | Confidence computation is O(1), reveals no secret data | Accepted |

### Gap Analysis

**No Permission Check**:
- **Risk**: Users can enable confidence without special permission
- **Impact**: Low—confidence is informational, not sensitive
- **Mitigation**: Confidence follows search permissions (if you can search, you can see confidence)
- **Status**: Accepted for MVP, can add permission layer in future if needed

**Exact Match Detection Dependency**:
- **Risk**: `is_exact_match` may be inaccurate if debug data unavailable
- **Impact**: Low—defaults to false (conservative), other signals still valid
- **Mitigation**: Document limitation, consider always computing exact_match_multiplier
- **Status**: Acceptable, documented in code comments

**No Rate Limiting**:
- **Risk**: Users could spam confidence requests to consume CPU
- **Impact**: Low—confidence computation is O(m) where m ≤ 20, <5ms overhead
- **Mitigation**: Existing search rate limiting applies (confidence doesn't bypass it)
- **Status**: Accepted, same risk profile as search itself

**Timing Side-Channel**:
- **Risk**: Confidence computation time might reveal score values
- **Impact**: Very Low—scores are non-secret, already returned in response
- **Mitigation**: None needed, timing reveals no secret information
- **Status**: Not a concern

## MVP Security Scope

### In Scope for MVP

✅ **Input Validation**:
- Boolean parameter validation (TypeScript + Rust)
- Type safety via Rust type system
- Array bounds checking in computation logic

✅ **Error Handling**:
- Graceful degradation for missing data
- No panics on invalid inputs
- No sensitive data in error messages

✅ **Dependency Safety**:
- Zero new dependencies (stdlib only)
- Existing dependencies already vetted

### Out of Scope for MVP (Future Work)

🔄 **Permission Layer**:
- Could add `hasPermission('debug_mode')` check
- Not needed for MVP (confidence is non-sensitive)
- Can add in Phase 2 if required

🔄 **Audit Logging**:
- Could log when confidence is requested
- Not needed for MVP (search already logged)
- Can add for analytics if desired

🔄 **Rate Limiting (Dedicated)**:
- Could add separate limit for confidence requests
- Not needed for MVP (search rate limiting sufficient)
- Can add if abuse detected

## Security Checklist

### Code Security

- [x] No hardcoded secrets
- [x] No credentials in code or configuration
- [x] No API keys or tokens
- [x] No database passwords

### Input Validation

- [x] Boolean parameter validated by type system
- [x] Array bounds checked before access
- [x] Division by zero guarded
- [x] No user-controlled arithmetic
- [x] No string parsing vulnerabilities

### Data Protection

- [x] No PII processed
- [x] No sensitive data in confidence signals
- [x] No data persistence
- [x] No logging of sensitive data

### Injection Prevention

- [x] No SQL injection risk (no SQL queries)
- [x] No command injection risk (no shell commands)
- [x] No code injection risk (no eval/dynamic code)
- [x] No JSON injection risk (Serde handles escaping)

### Error Handling

- [x] Graceful degradation for missing data
- [x] No panics on invalid inputs
- [x] No sensitive data in error messages
- [x] No stack traces exposed to users

### Dependencies

- [x] Zero new dependencies (stdlib only)
- [x] Existing dependencies vetted
- [x] No transitive dependency additions
- [x] Cargo.lock committed (pinned versions)

### Supply Chain

- [x] No new crates added
- [x] No npm packages added
- [x] No external API calls
- [x] No third-party services

## Deployment Security

### Rollout Plan

**Phase 1**: Opt-in beta (`include_confidence=false` default)
- Low risk: Feature disabled by default
- Users must explicitly enable
- Can monitor for abuse

**Phase 2**: General availability (still opt-in)
- Monitor adoption rate
- Check for unexpected usage patterns
- Gather feedback on any concerns

**(Future) Phase 3**: Default enabled
- Only after validation period
- Can feature-flag disable if issues arise

### Rollback Strategy

**If Security Issue Detected**:
1. Set `include_confidence=false` as hard default (revert parameter)
2. Disable feature flag (if implemented)
3. Investigate issue
4. Fix and re-deploy
5. Re-enable after validation

**Rollback Complexity**: Very Low
- No database schema changes
- No data migrations
- Simple code revert
- No user data to clean up

## Threat Model

### Threats Considered

**Threat 1: Information Disclosure**
- **Attack**: User attempts to infer sensitive data from confidence signals
- **Impact**: Low—confidence contains only public mathematical properties
- **Likelihood**: Low—no sensitive data present
- **Mitigation**: Confidence is derived from non-sensitive search results
- **Status**: Not a concern

**Threat 2: Denial of Service**
- **Attack**: User spams confidence requests to consume CPU
- **Impact**: Low—<5ms overhead per request, bounded by search rate limiting
- **Likelihood**: Low—easier to DoS via search itself
- **Mitigation**: Existing search rate limiting applies
- **Status**: Acceptable risk

**Threat 3: Privilege Escalation**
- **Attack**: User attempts to access confidence without permission
- **Impact**: None—confidence is non-sensitive
- **Likelihood**: N/A—no permissions required
- **Mitigation**: None needed
- **Status**: Not applicable

**Threat 4: Data Tampering**
- **Attack**: User attempts to manipulate confidence values
- **Impact**: None—confidence computed server-side, not user-provided
- **Likelihood**: None—no user input in computation
- **Mitigation**: All computation on trusted server data
- **Status**: Not a concern

### Threats NOT Considered (Out of Scope)

- **Physical Security**: Server infrastructure (out of scope for feature)
- **Network Security**: TLS/HTTPS (handled at infrastructure layer)
- **Account Security**: User authentication (handled by MCP layer)
- **Compliance**: GDPR/CCPA (no PII collected or processed)

## Security Review Conclusion

**Verdict**: ✅ **Ship without meaningful security concerns**

**Rationale**:
1. No sensitive data processed or exposed
2. No new attack surface introduced
3. Zero new dependencies or external calls
4. Follows existing security model (same as search)
5. Graceful error handling prevents information leakage
6. Input validation prevents injection attacks
7. Backward compatible (opt-in feature)
8. Simple rollback if issues detected

**Recommendation**: Proceed with implementation. No security blockers for MVP.

**Future Security Enhancements** (optional, post-MVP):
- Add permission check if auth system implemented
- Add audit logging for analytics (not security)
- Monitor usage patterns for abuse detection

**Sign-Off**: Security review complete. No security concerns for MVP launch.
