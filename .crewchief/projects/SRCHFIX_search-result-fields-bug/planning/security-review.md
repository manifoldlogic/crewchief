# Security Review: Search Result Fields Bug Fix

## Summary

This bug fix has **no meaningful security implications**. It exposes data that already exists in the database and is already being queried - we're just fixing the serialization layer to actually return it.

## Security Assessment

### Authentication & Authorization

**Change**: None

**Assessment**: No impact

**Rationale**: This fix doesn't change who can access search results or what repositories they can search. Access control remains at the daemon/MCP server level (filesystem permissions on database and configuration).

### Data Protection

**Change**: Exposing chunk_id, symbol_name, and kind in search results.

**Assessment**: No new security concerns

**Analysis**:
- These fields already exist in the database
- Search results already return file paths and line numbers
- chunk_id is just a database primary key (not sensitive)
- symbol_name is the function/class name visible in source code
- kind is just metadata about the symbol type

**Comparison**:
- **Already exposed**: File paths, line ranges, code content, scores
- **Now exposing**: chunk_id (integer), symbol_name (string), kind (string)
- **Not exposing**: Nothing new - all data already in database and accessible via other means

### Input Validation

**Change**: TypeScript interface now accepts chunk_id, symbol_name, kind from daemon.

**Assessment**: No new injection risks

**Validation**:
- chunk_id: Integer from database (already validated by SQLite)
- symbol_name: String from database (already escaped by tree-sitter parser)
- kind: Enum-like string from tree-sitter (limited set of values)

**Risk**: None - these values come from trusted source (database) and are already sanitized during indexing.

## Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| No rate limiting | Low | Out of scope - internal tool, trusted users | Accepted |
| No audit logging | Low | Out of scope - would require authentication layer | Accepted |
| Database filesystem permissions only | Low | Single-user assumption, documented | Accepted |
| Client must escape output | Medium | Document requirement for HTML escaping | Open |

## Data Integrity

**Change**: Removing hardcoded empty strings and 0 values, using actual database values.

**Assessment**: Improvement

**Before**: Data integrity violated - returning incorrect data (chunk_id=0, symbol_name='')

**After**: Data integrity restored - returning actual database values

**Benefit**: Clients can trust the data again (especially chunk_id for context retrieval).

## Threat Modeling

### Threat 1: Malicious chunk_id Injection

**Attack**: Client provides malicious chunk_id to context tool.

**Impact**: Context tool might return wrong code or error.

**Mitigation**:
- chunk_id validated as positive integer
- Database lookup returns empty result for invalid ID
- No SQL injection risk (parameterized queries)

**Likelihood**: Low (would only affect attacker's own session)

**Severity**: Low (worst case: wrong results, no data corruption)

**Status**: Acceptable

### Threat 2: Symbol Name XSS

**Attack**: Malicious code in repository has symbol name like `<script>alert('xss')</script>`.

**Impact**: If symbol_name displayed in UI without escaping, could execute JavaScript.

**Mitigation**:
- Responsibility of client (MCP consumer) to escape output
- Standard practice for all user-facing data
- Same risk already exists for file paths and content

**Likelihood**: Low (requires malicious code in indexed repository)

**Severity**: Medium (if client doesn't escape)

**Status**: Acceptable - document that clients must escape

**Action**: Add to documentation: "Clients displaying search results should HTML-escape all fields including symbol_name and kind."

### Threat 3: Information Disclosure via chunk_id

**Attack**: Enumerate chunk_ids to discover indexed code without searching.

**Impact**: Could bypass search filters to read all indexed code.

**Mitigation**:
- chunk_id is just a database primary key (sequential)
- Database already accessible to anyone who can run MCP server
- No additional information leaked (file paths already exposed in search)

**Likelihood**: Low (requires MCP server access)

**Severity**: Low (same data accessible via search or direct DB access)

**Status**: Acceptable

## Data Privacy

**Question**: Do chunk_id, symbol_name, or kind contain PII?

**Answer**: No

**Rationale**:
- chunk_id: Database primary key (integer)
- symbol_name: Function/class/method name from source code (public information)
- kind: Symbol type (enum-like string: "function", "class", etc.)

**Conclusion**: No PII implications

## MVP Security Scope

### In Scope

1. **Input validation**: TypeScript types enforce correct types
2. **SQL injection prevention**: Parameterized queries (already implemented, no change)
3. **Data integrity**: Return actual database values (not hardcoded defaults)

### Out of Scope

1. **Rate limiting**: Not part of this fix (daemon architecture concern)
2. **Audit logging**: Not part of this fix (would require auth system)
3. **Encryption**: Not part of this fix (database already unencrypted)
4. **Multi-user isolation**: Not part of this fix (single-user assumption)

## Security Checklist

- [x] No hardcoded secrets
- [x] Input validation on external inputs (TypeScript types)
- [x] Proper error handling (existing error handling unchanged)
- [x] Dependencies are up to date (no new dependencies)
- [x] No SQL injection vulnerabilities (parameterized queries)
- [ ] Client output escaping documented (needs documentation update)

## Deployment Security

### Rollout Plan

**No security-specific rollout steps needed**

**Standard deployment**:
1. Build Rust binary (cargo build --release)
2. Build TypeScript packages (pnpm build)
3. Deploy to packages (already in monorepo)

**Risk**: None - backward compatible change (additive fields)

### Rollback Plan

**If security issue found**: Revert commits (standard git revert)

**Time to rollback**: < 5 minutes (just a git revert + rebuild)

## Recommendations

### For This Fix

**Ship it** - No security blockers identified

### For Future Work

1. **Add output escaping documentation**: Document that clients must HTML-escape symbol_name and kind when displaying in UI (medium priority)

2. **Consider rate limiting**: Add to daemon architecture roadmap (low priority - internal tool)

3. **Consider audit logging**: Add to daemon architecture roadmap (low priority - internal tool)

4. **Document security model**: Create a security section in CLAUDE.md explaining single-user assumption and trust model (medium priority)

## Sign-Off

**Security Review Status**: Approved

**Reviewer Assessment**: This bug fix restores data integrity without introducing new security risks. All exposed data already exists in the database and is already accessible via search or direct database access. No authentication, authorization, or input validation changes required.

**Ship without security concerns**: Yes

**Caveats**:
- Clients displaying search results should HTML-escape all fields (standard practice, not specific to this fix)
- Known security gaps (rate limiting, audit logging) remain but are out of scope
- Security model assumes single-user, trusted environment (documented limitation)
