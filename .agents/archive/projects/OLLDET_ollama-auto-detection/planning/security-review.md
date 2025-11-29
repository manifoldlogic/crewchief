# Security Review: Ollama Auto-Detection

## Summary

This change introduces network probing of multiple endpoints for Ollama detection. The security impact is minimal as it only affects local development tooling and uses read-only health check endpoints.

## Security Analysis

### Network Probing

**What we're doing:**
- HTTP GET requests to `/api/tags` on up to 3 endpoints
- 2-second timeout per request
- Sequential, not parallel

**Endpoints probed:**
1. User-configured endpoint (from env var)
2. `localhost:11434`
3. `host.docker.internal:11434`

**Risk assessment: LOW**

These are all local or explicitly configured endpoints. No external network calls. No data exfiltration risk.

### Information Disclosure

**What could be exposed:**
- Ollama's `/api/tags` returns list of installed models

**Risk assessment: NEGLIGIBLE**

This information stays local. Not logged beyond debug level. Not transmitted anywhere.

### Denial of Service

**Potential issue:**
- 6-second max startup delay if all endpoints timeout

**Risk assessment: LOW**

Only affects cold start. Doesn't retry. Doesn't loop. Controlled timeout.

### SSRF (Server-Side Request Forgery)

**Potential issue:**
- `MAPROOM_EMBEDDING_API_ENDPOINT` could point to arbitrary URLs

**Risk assessment: LOW**

- Only checks `/api/tags` endpoint (read-only)
- User must explicitly set env var
- Only used for embedding generation (local tool)
- No credentials sent in health check

**Mitigation consideration:**
- Could validate URL scheme (http/https only)
- Could restrict to localhost/private IPs
- NOT implementing for MVP (user-controlled tool)

## Environment Variables

### `MAPROOM_EMBEDDING_API_ENDPOINT`

- **Set by:** User
- **Contains:** URL to Ollama embed endpoint
- **Sensitivity:** Low (local service URL)
- **Storage:** Environment variable (standard practice)

No secrets or credentials in this env var.

## Network Boundaries

| Endpoint | Network | Trust Level |
|----------|---------|-------------|
| `localhost:11434` | Loopback | High |
| `host.docker.internal:11434` | Host bridge | High |
| Custom endpoint | User-defined | User responsibility |

All endpoints are either local or explicitly user-configured.

## Authentication

Ollama doesn't require authentication by default. Our health check:
- Sends no credentials
- Expects no auth headers
- Uses standard HTTP GET

If user runs authenticated Ollama proxy, they should use explicit config.

## Logging

**Current logging:**
- Debug level: endpoint checks, failures
- Info level: successful detection

**Security consideration:**
- Don't log full response bodies
- Don't log credentials (none used)
- Current logging is appropriate

## Recommendations

### MVP (Implement Now)

1. **Validate URL scheme** - Only allow `http://` and `https://` schemes
2. **Document behavior** - User should know which endpoints are tried

### Future (Out of Scope)

1. **Private IP restriction** - Could restrict to RFC 1918 addresses
2. **Configurable fallbacks** - Let users disable host.docker.internal check
3. **Auth support** - Handle authenticated Ollama proxies

## Compliance

No compliance concerns for local development tooling:
- No PII handling
- No external data transmission
- No credential storage
- No audit logging requirements

## Conclusion

**Security rating: LOW RISK**

This change:
- Only probes local/user-configured endpoints
- Uses read-only health checks
- Has controlled timeouts
- Logs at appropriate levels
- Doesn't handle sensitive data

Approved for implementation without additional security controls.
