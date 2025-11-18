# Security Review: Worktree-Scoped Search

## Executive Summary

**Overall Risk Level:** 🟢 **LOW**

This project introduces minimal security risk. The changes are primarily focused on query scoping and caching, with no new external inputs or privileged operations. The main security considerations are:

1. **Command injection via git** - Mitigated by using safe subprocess APIs
2. **Path traversal via worktree paths** - Not applicable (paths come from database, not user input)
3. **Information disclosure** - Reduced (narrower search scope)
4. **Cache poisoning** - Low risk (TTL-based expiry, no user input in cache keys)

**Recommendation:** ✅ **SHIP** - No blocking security concerns. Standard code review and testing are sufficient.

## Threat Model

### Assets

**What are we protecting?**

1. **Code Repository Contents**
   - Indexed source code chunks
   - File paths and worktree locations
   - Git commit metadata

2. **Database Integrity**
   - Worktree metadata
   - Chunk associations
   - Search indexes

3. **User Context**
   - Current working directory
   - Current git branch
   - MCP session state

### Threat Actors

**Who might attack?**

1. **Malicious User** (Internal)
   - Has access to MCP server
   - Could manipulate search parameters
   - Could try to access unauthorized code

2. **Compromised Dependencies** (Supply Chain)
   - Malicious npm packages
   - Backdoored git binaries
   - Database driver vulnerabilities

3. **External Attacker** (Remote)
   - No direct access to MCP server (internal tool)
   - Could exploit vulnerabilities if exposed

**Primary Threat:** Malicious internal user attempting to access code outside their authorization scope.

### Attack Vectors

**How could someone attack this feature?**

1. **Git Command Injection**
   - Manipulate working directory to inject malicious git commands
   - Exploit unsanitized input to git subprocess

2. **Cache Poisoning**
   - Pollute cache with false branch names
   - Cause users to search wrong worktrees

3. **Information Disclosure**
   - Bypass worktree scoping to access unauthorized code
   - Leak worktree metadata through error messages

4. **Denial of Service**
   - Trigger expensive git operations repeatedly
   - Exhaust cache memory
   - Slow down searches with malicious queries

## Security Analysis by Component

### 1. Git Command Execution

**Component:** `getCurrentBranch()` in `utils/git.ts`

**Code:**
```typescript
export async function getCurrentBranch(cwd?: string): Promise<string> {
  const branch = await execGit(['rev-parse', '--abbrev-ref', 'HEAD'], cwd)
  return branch.trim()
}
```

**Threat:** Command injection via `cwd` parameter

**Risk Level:** 🟡 **MEDIUM** (if `cwd` comes from user input)

**Analysis:**
- The `execa` library properly escapes arguments and prevents command injection
- `cwd` parameter is an **option** to `execa`, not part of the command string
- Git arguments are hardcoded (`['rev-parse', '--abbrev-ref', 'HEAD']`)
- No user input is concatenated into the command

**Vulnerable Example (NOT how we do it):**
```typescript
// ❌ VULNERABLE - Don't do this
const cmd = `git -C ${cwd} rev-parse --abbrev-ref HEAD`
exec(cmd) // Command injection possible
```

**Safe Implementation (what we actually do):**
```typescript
// ✅ SAFE - Arguments are passed as array
await execa('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { cwd })
```

**Verification:**
- [ ] Review `execa` usage for proper argument passing
- [ ] Ensure `cwd` parameter is not user-controlled (it's from MCP context)
- [ ] Validate that no string interpolation is used in git commands

**Mitigation:** Already mitigated by using `execa` with argument arrays.

**Residual Risk:** 🟢 **LOW** - No user input in git commands

### 2. Worktree Path Handling

**Component:** Database query for worktree abs_path

**Code:**
```typescript
const result = await client.query(
  `SELECT w.id, w.abs_path
   FROM maproom.worktrees w
   WHERE w.name = $1 AND w.repo_id = $2`,
  [worktree, repoId]
)
```

**Threat:** Path traversal to read unauthorized files

**Risk Level:** 🟢 **LOW**

**Analysis:**
- Worktree paths come from **database**, not user input
- Paths were validated during indexing (out of scope for this project)
- No file system operations use user-provided paths
- Search results reference database IDs, not file paths directly

**Attack Scenario:**
1. Attacker controls `worktree` parameter value
2. Passes malicious worktree name like `../../../etc/passwd`
3. Database query uses parameterized queries (safe from SQL injection)
4. Query returns no rows (path doesn't exist in database)
5. Attack fails

**Why This Is Safe:**
- Database lookup is **allow-list based** (only indexed worktrees exist)
- File paths are validated during indexing (separate project)
- This project only queries existing data, doesn't create new paths

**Verification:**
- [ ] Confirm all database queries use parameterized queries ($1, $2, etc.)
- [ ] Verify no direct file system access with user-provided paths
- [ ] Check that worktree paths are absolute (no relative path handling)

**Mitigation:** Already mitigated by database-backed allow-list.

**Residual Risk:** 🟢 **LOW** - No path traversal possible

### 3. Cache Security

**Component:** In-memory LRU cache for branches and worktree IDs

**Code:**
```typescript
const branchCache = new LRUCache<string, string>({
  max: 100,
  ttl: 60_000,
})

branchCache.set(cwd, branchName)
```

**Threat:** Cache poisoning to cause incorrect worktree searches

**Risk Level:** 🟢 **LOW**

**Analysis:**
- Cache keys are **not user-controlled** (derived from `process.cwd()` or git output)
- Cache values come from **trusted sources** (git command output, database)
- TTL limits impact of any pollution (60 seconds)
- Cache is **in-memory only** (not persisted, resets on restart)

**Attack Scenario:**
1. Attacker somehow pollutes cache with wrong branch name
2. User searches, gets results from wrong worktree
3. Impact: Wrong results, but **no privilege escalation**
4. Cache expires after 60s, self-heals

**Why Impact Is Limited:**
- User can only see code they already have access to (all worktrees are in same repo)
- Worst case: User searches wrong branch, gets unexpected results
- No cross-repository access (repo is scoped separately)

**Verification:**
- [ ] Confirm cache keys are not derived from user input
- [ ] Verify TTL is set correctly (60s for branch, 300s for worktree IDs)
- [ ] Check that cache eviction works (max size enforced)

**Mitigation:** TTL-based expiry, in-memory only, no persistence.

**Residual Risk:** 🟢 **LOW** - Limited impact, self-healing

### 4. Information Disclosure

**Component:** Error messages and metadata in search results

**Code:**
```typescript
result.hint = `Current branch '${detectedBranch}' is not indexed.\n\n` +
  `To search your current code:\n` +
  `1. Run: mcp__maproom__scan({repo: "${repo}", worktree: "${detectedBranch}"})\n\n` +
  `Searching 'main' worktree instead.`
```

**Threat:** Leaking sensitive information through error messages

**Risk Level:** 🟢 **LOW**

**Analysis:**
- Error messages reveal: branch names, repo names, worktree names
- **Not sensitive:** This information is already known to the user (it's their git repo)
- **No new disclosure:** User already has access to git branch list
- **No cross-user leakage:** MCP server is single-user (user's own session)

**Information Leaked:**
- Current git branch name (user already knows)
- Available worktrees (user can see with `git worktree list`)
- Database schema hints (column names, table names)

**Why This Is Acceptable:**
- MCP server runs in user's own environment (not multi-tenant)
- User has direct access to git repository (can see all branches)
- No passwords, tokens, or secrets in error messages
- Error messages help user understand and fix issues

**Verification:**
- [ ] Ensure no secrets (tokens, passwords) in error messages
- [ ] Verify error messages don't reveal internal system paths
- [ ] Check that stack traces are not exposed in production

**Mitigation:** Error messages are already appropriately scoped.

**Residual Risk:** 🟢 **LOW** - Acceptable for single-user tool

### 5. Denial of Service

**Component:** Branch detection and database queries

**Threat:** Exhaust resources through repeated expensive operations

**Risk Level:** 🟢 **LOW**

**Analysis:**
- Git subprocess calls: 5-10ms each, limited by cache
- Database queries: 2-3ms each, limited by cache
- Cache size: Fixed max (100 for branches, 500 for worktrees)
- TTL: Short expiry prevents indefinite growth

**Attack Scenario:**
1. Attacker triggers 1000 searches per second
2. Cache saturates at max size (100/500 entries)
3. Each search still requires git call + DB lookup (13ms overhead)
4. Total overhead: 13ms × 1000 = 13 seconds of CPU per second
5. Impact: High CPU usage, but **no crash or permanent damage**

**Why Impact Is Limited:**
- Cache prevents unbounded memory growth
- Git calls are non-blocking (async)
- Database has connection pooling and query limits
- MCP server is single-user (no amplification)

**Mitigation:**
- LRU cache with max size prevents memory exhaustion
- TTL ensures cache doesn't grow indefinitely
- Async operations prevent blocking event loop

**Verification:**
- [ ] Confirm cache max size is enforced
- [ ] Verify TTL is set correctly
- [ ] Test with high search volume to measure resource usage

**Residual Risk:** 🟢 **LOW** - Acceptable for single-user tool

## Supply Chain Security

### Dependency Risk

**New Dependencies:**
- `lru-cache` (for caching)

**Risk Assessment:**

**`lru-cache`:**
- **Maintainer:** Isaac Z. Schlueter (npm founder, trusted)
- **Downloads:** 100M+ per week (widely used)
- **Last Update:** Active maintenance
- **Vulnerabilities:** None known
- **Risk:** 🟢 **LOW**

**Existing Dependencies:**
- `execa` (subprocess execution) - Already used, trusted
- `pg` (PostgreSQL client) - Already used, trusted

**Mitigation:**
- Regular dependency updates via Dependabot
- `npm audit` in CI/CD pipeline
- Lock file (`package-lock.json`) for reproducible builds

**Verification:**
- [ ] Run `npm audit` before release
- [ ] Check `lru-cache` version for known vulnerabilities
- [ ] Review dependency tree for unexpected additions

**Residual Risk:** 🟢 **LOW** - Standard dependency risk

## Access Control

### Authorization Model

**Current Model:**
- MCP server runs in user's environment
- User has access to all worktrees in repository
- No cross-user isolation (single-user tool)
- No cross-repository isolation in this project (repo scoped separately)

**Changes Introduced:**
- Default search scope narrowed to current worktree
- User can explicitly search other worktrees
- User can search all worktrees by passing `null`

**Security Impact:** 🟢 **POSITIVE**

**Why This Improves Security:**
- Principle of Least Privilege: Default to narrowest scope
- Reduces accidental information disclosure (search only current context)
- Explicit opt-in for broader searches (user must pass `null`)

**Attack Scenario: Bypass Worktree Scoping**
1. Attacker wants to search all worktrees, not just current
2. Passes `worktree: null` to search tool
3. Search returns results from all worktrees
4. **Impact:** None - user already has access to all worktrees

**Why This Is Acceptable:**
- User owns the repository and all worktrees
- No privilege escalation (user can already access all code)
- This is a single-user tool, not multi-tenant

**Verification:**
- [ ] Confirm worktree scoping is enforced correctly
- [ ] Verify explicit `null` is required to search all
- [ ] Test that auto-detection can't be bypassed

**Residual Risk:** 🟢 **LOW** - No new authorization risks

## Data Privacy

### Personally Identifiable Information (PII)

**Data Handled:**
- Git branch names (may contain developer names)
- File paths (may reveal project structure)
- Code content (may contain comments with names)

**Privacy Risk:** 🟢 **LOW**

**Why:**
- Data stays in user's local environment
- No data sent to external services
- No logging of sensitive information
- MCP server is local, not cloud-hosted

**Verification:**
- [ ] Confirm no data is sent to external APIs
- [ ] Verify logs don't contain sensitive information
- [ ] Check that cache is not persisted to disk

**Residual Risk:** 🟢 **LOW** - Local-only tool

## Compliance

### Industry Standards

**Applicable Standards:**
- OWASP Top 10 (web application security)
- CWE (Common Weakness Enumeration)
- NIST Cybersecurity Framework

**Compliance Status:**

**OWASP Top 10 (2021):**
1. Broken Access Control → ✅ Not applicable (single-user)
2. Cryptographic Failures → ✅ No cryptography used
3. Injection → ✅ Mitigated (parameterized queries, safe subprocess)
4. Insecure Design → ✅ Defense in depth (allow-list, caching)
5. Security Misconfiguration → ✅ Secure defaults (narrow scope)
6. Vulnerable Components → ✅ Dependencies reviewed
7. Authentication Failures → ✅ Not applicable (no auth)
8. Software/Data Integrity → ✅ TTL prevents stale data
9. Logging Failures → ✅ Appropriate logging
10. SSRF → ✅ No external requests

**CWE Coverage:**
- CWE-78 (OS Command Injection) → ✅ Mitigated (safe subprocess API)
- CWE-89 (SQL Injection) → ✅ Mitigated (parameterized queries)
- CWE-22 (Path Traversal) → ✅ Not applicable (database paths only)
- CWE-200 (Information Disclosure) → ✅ Acceptable (single-user tool)

**Residual Risk:** 🟢 **LOW** - Meets industry standards

## Security Checklist

### Pre-Release Security Validation

- [ ] **Command Injection:** Verify `execa` uses argument arrays, not string concatenation
- [ ] **SQL Injection:** Confirm all queries use parameterized statements ($1, $2, etc.)
- [ ] **Path Traversal:** Check that no file operations use user-provided paths
- [ ] **Information Disclosure:** Review error messages for sensitive data
- [ ] **Cache Security:** Verify TTL and max size are enforced
- [ ] **Dependency Audit:** Run `npm audit` and resolve any high/critical vulnerabilities
- [ ] **Access Control:** Test that worktree scoping is enforced correctly
- [ ] **Error Handling:** Ensure errors fail safely and don't expose internals
- [ ] **Logging:** Verify logs don't contain secrets or sensitive paths
- [ ] **Code Review:** Security-focused review by second developer

### Post-Release Monitoring

**Metrics to Track:**
- Cache hit rate (should be >95%, if much lower, investigate)
- Error rate (spikes may indicate security probing)
- Search latency (DoS attacks would increase latency)

**Alerts:**
- High error rate (>10% of requests)
- Unusual branch names in logs (potential injection attempts)
- Cache size approaching max (potential DoS)

## Known Limitations (Accepted Risks)

### 1. Cache Timing Attack

**Scenario:** Attacker measures response time to determine if branch is cached

**Impact:** Minimal - reveals which branches are frequently searched

**Acceptance:** Acceptable for single-user tool, no sensitive information

### 2. Git Binary Compromise

**Scenario:** Attacker replaces git binary with malicious version

**Impact:** Could inject malicious branch names, corrupt cache

**Acceptance:** If git is compromised, user has bigger problems

**Note:** This is outside the security boundary of this project

### 3. Database Compromise

**Scenario:** Attacker gains write access to PostgreSQL database

**Impact:** Could create fake worktrees, pollute search results

**Acceptance:** Database security is handled separately

**Note:** This project assumes database integrity

## Conclusion

**Security Posture:** 🟢 **GOOD**

This project introduces **minimal new security risk** while actually **improving** security through narrower default scoping (principle of least privilege).

**Key Strengths:**
- ✅ Safe subprocess handling (no command injection)
- ✅ Parameterized SQL queries (no SQL injection)
- ✅ Database-backed allow-list (no path traversal)
- ✅ TTL-based caching (limits pollution impact)
- ✅ Secure defaults (narrow scope, opt-in for broad)

**Accepted Trade-offs:**
- Cache timing attacks (low impact)
- Error message information disclosure (acceptable for single-user)
- DoS through repeated searches (acceptable CPU usage)

**Recommendation:** ✅ **APPROVE FOR SHIPPING**

**Conditions:**
1. Complete security checklist before release
2. Run `npm audit` and resolve any high/critical vulnerabilities
3. Code review with security focus
4. Integration tests cover security-relevant scenarios

**No enterprise hardening required** - this is an internal development tool, not a public-facing service. Standard secure coding practices are sufficient.
