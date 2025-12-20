# Security Review: Maproom Skill Progressive Disclosure (MPRSKL)

## Security Assessment

This ticket has **low security impact** as it primarily involves:
1. Configuration coordination (internal code change)
2. Documentation restructuring (static files)
3. Error message enhancement (user-facing text)

No new attack surfaces are introduced. No authentication, network, or data handling changes.

### Authentication & Authorization

**Not applicable to this ticket.**

Maproom authentication is handled at the embedding provider level:
- OpenAI: `OPENAI_API_KEY` env var (existing)
- Google: Service account credentials (existing)
- Ollama: No auth (local service)

This ticket does not change authentication flows.

### Data Protection

**No changes to data handling.**

The bug fix modifies config loading flow, not data processing:
- No new data paths
- No changes to embedding storage
- No changes to database access
- No PII handling affected

### Input Validation

**Existing validation unchanged.**

Configuration input validation remains:
- Provider names validated against enum
- Dimension validated as positive integer
- API endpoints validated for format
- File paths validated for existence (Google credentials)

New `from_env_with_provider()` method accepts a `Provider` enum, not raw user input. The enum is type-safe and cannot receive arbitrary values.

### Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| Error messages may leak config info | Low | Error messages show provider and dimension (non-sensitive) | Accepted |
| Documentation may become stale | Low | CLI --help is authoritative, docs are supplementary | Accepted |

## Initial Release Security Scope

### In Scope

1. **Type-safe provider propagation** - `Provider` enum, not string
2. **No new env vars** - Uses existing `MAPROOM_EMBEDDING_*` namespace
3. **No credential changes** - API keys handled by existing code
4. **Static documentation** - Markdown files with no executable content

### Out of Scope (Future)

- API key rotation mechanisms
- Credential encryption at rest
- Audit logging for config changes
- Rate limiting for embedding API calls

## Security Checklist

- [x] No hardcoded secrets
  - No credentials in code
  - Error messages don't include API keys

- [x] Input validation on external inputs
  - Provider names go through enum parsing
  - Dimension values validated as integers
  - No new external inputs added

- [x] Proper error handling (no info leakage)
  - Error messages include provider and dimension (non-sensitive)
  - No API keys, tokens, or credentials in errors
  - Stack traces only in debug mode

- [x] Dependencies are up to date
  - No new dependencies added
  - Existing deps unchanged

- [x] No SQL injection vulnerabilities
  - No SQL changes in this ticket
  - Config is in-memory, not database

- [x] No XSS vulnerabilities
  - Documentation is static markdown
  - CLI output is text-only

## Threat Analysis

### Potential Concerns (Low Risk)

1. **Environment Variable Enumeration**
   - Risk: Error messages mention env var names
   - Impact: Attacker learns which vars exist
   - Mitigation: Env var names are documented publicly anyway
   - Status: Accepted (non-sensitive information)

2. **Configuration Disclosure in Errors**
   - Risk: Error messages show current config values
   - Impact: Attacker learns dimension setting
   - Mitigation: Dimension is not sensitive (1024, 1536, 768)
   - Status: Accepted

3. **Documentation Manipulation**
   - Risk: Malicious skill documentation could mislead agents
   - Impact: Agent could run wrong commands
   - Mitigation: Skills are in git, code review required
   - Status: Accepted (standard git workflow)

### Non-Concerns

- **API Key Exposure**: Keys never logged or shown in errors
- **Network Attack Surface**: No new network endpoints
- **Privilege Escalation**: No privilege changes
- **Data Exfiltration**: No new data access paths

## Recommendations

1. **Keep error messages helpful but minimal**
   - Show: provider name, expected/actual dimensions
   - Don't show: API keys, full config dumps, internal paths

2. **Skill documentation review**
   - Verify commands shown are safe to run
   - No dangerous flags documented without warnings
   - Clear separation of read-only vs write operations

3. **Test error paths**
   - Ensure all error messages are appropriate
   - No sensitive data in any error path

## Conclusion

This ticket has minimal security implications. The changes are:
- Internal configuration coordination (type-safe)
- Static documentation (reviewed via git)
- User-facing error messages (non-sensitive)

No additional security measures required beyond standard code review.
