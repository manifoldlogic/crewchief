# Security Review: MCP Server Simplification

## Architecture Security Analysis

### Attack Surface Reduction

This project **improves** security posture by removing code:

| Removed Component | Security Benefit |
|-------------------|------------------|
| Docker orchestration | No container privilege escalation paths |
| Ollama container mgmt | No model download attack vector |
| Config file management | No local file permission issues |
| Multi-container networking | Simpler network topology |

**Net effect**: ~1,920 lines of code removed = ~1,920 fewer lines that could have vulnerabilities.

### Remaining Attack Surface

1. **Environment Variables**
   - `MAPROOM_DATABASE_URL` - Contains credentials
   - `OPENAI_API_KEY` - API key
   - `GOOGLE_APPLICATION_CREDENTIALS` - Service account

2. **Database Connection**
   - PostgreSQL with credentials
   - Connection over network (localhost or container network)

3. **MCP Protocol**
   - JSON-RPC over stdio
   - Tool handlers process untrusted input (search queries)

4. **Rust Daemon**
   - Subprocess spawned by MCP server
   - Inherits environment variables

## Risk Evaluation

### Low Risk: Credential Exposure
**Current state**: Credentials in environment variables
**Analysis**: This is standard practice for MCP servers. Environment variables are:
- Not logged (existing redaction code preserved)
- Not written to disk
- Process-scoped

**Mitigation**: Keep existing credential redaction in error messages.

### Low Risk: SQL Injection
**Current state**: Rust daemon uses parameterized queries
**Analysis**: No changes to query handling. Existing protections remain.

### Negligible Risk: MCP Protocol
**Current state**: JSON-RPC parsing via standard library
**Analysis**: MCP protocol is sandboxed by the client. The server can only:
- Read files (via `open` tool)
- Execute database queries
- Return search results

No filesystem writes, no command execution.

### Not Applicable: Container Escape
**Before**: Multiple containers with shared volumes
**After**: Only PostgreSQL container, managed externally

MCP server runs on host, not in container. No container escape vector.

## Known Gaps

### Gap 1: Database Credentials in URL
**Issue**: `MAPROOM_DATABASE_URL` contains password in plaintext
**Risk Level**: Low - standard practice, environment-scoped
**Enterprise Fix**: Use connection strings from secret manager
**MVP Approach**: Accept this standard pattern

### Gap 2: No TLS for Local Database
**Issue**: Default connection to `localhost:5433` is unencrypted
**Risk Level**: Negligible - localhost traffic only
**Enterprise Fix**: Configure SSL certificates
**MVP Approach**: Accept for local development

### Gap 3: API Keys in Environment
**Issue**: `OPENAI_API_KEY` passed via environment
**Risk Level**: Low - standard practice for API integrations
**Enterprise Fix**: Secret injection at runtime
**MVP Approach**: Accept this standard pattern

## MVP-Appropriate Mitigations

### Implemented (Keep)
1. **Credential redaction in logs** - Existing code masks sensitive values
2. **Parameterized queries** - Rust daemon prevents SQL injection
3. **Process isolation** - MCP server is sandboxed by client

### Not Implemented (Accept for MVP)
1. **TLS for database** - Overkill for localhost
2. **Secret manager integration** - Enterprise feature
3. **Audit logging** - Beyond MVP scope

## Security Checklist

- [x] No hardcoded credentials in code
- [x] Credentials not logged or exposed
- [x] SQL queries use parameterization
- [x] No arbitrary command execution
- [x] No arbitrary file writes
- [x] Attack surface reduced (code deletion)

## Ship Decision

**Recommendation: Ship without security concerns**

This simplification improves security by:
1. Removing ~2,000 lines of code (fewer bugs)
2. Eliminating container orchestration (simpler architecture)
3. Preserving existing security measures

No new security risks introduced. Existing protections maintained.
