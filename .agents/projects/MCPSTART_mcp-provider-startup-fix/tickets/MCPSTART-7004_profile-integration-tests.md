# Ticket: MCPSTART-7004: Profile Integration Tests

## Status
- [ ] **Task completed** - DEFERRED (Phase 3 solution implemented instead)
- [ ] **Tests pass** - N/A (ticket deferred)
- [ ] **Verified** - N/A (ticket deferred)

**Note**: Phase 7 (Docker Compose Profiles) was deferred in favor of the Phase 3 solution (explicit stop/remove of unnecessary services via CLI logic). The Phase 3 implementation successfully solves the problem and has been validated in production through v1.3.1. Docker profiles remain a viable alternative for future enhancement.

## Agents
- integration-tester (primary)
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests validating Docker Compose profile behavior, including profile activation based on `EMBEDDING_PROVIDER`, correct service startup patterns, and proper error handling for profile-related issues.

## Background
This ticket validates Phase 4 (Service Profiles) implementation from MCPSTART_ARCHITECTURE.md. After implementing profile configuration (MCPSTART-7001) and startup logic (MCPSTART-7002), we need automated tests ensuring profiles work correctly across all provider configurations.

Reference: MCPSTART_ARCHITECTURE.md Phase 4 (lines 243-305)

## Acceptance Criteria
- [ ] Integration tests cover profile-based startup with Ollama provider
- [ ] Integration tests cover profile-based startup with external providers (OpenAI, Anthropic, VoyageAI)
- [ ] Tests verify correct service list when using `--profile ollama`
- [ ] Tests verify correct service list when no profiles specified
- [ ] Tests validate docker compose command construction with profile flags
- [ ] Tests cover edge cases (missing provider, invalid provider, empty string)
- [ ] All existing integration tests still pass with profile changes
- [ ] Test output clearly indicates which profile scenarios are being validated

## Technical Requirements
- Add test cases to existing integration test suite (from MCPSTART-4002)
- Mock or stub `spawnSync` calls for docker compose commands
- Validate command arguments include correct `--profile` flags
- Test different `EMBEDDING_PROVIDER` values and their profile effects
- Verify service startup sequence with and without profiles
- Test backward compatibility with non-profile docker-compose.yml
- Include assertions on diagnostic log output related to profiles
- Ensure tests are idempotent and don't affect other test runs

## Implementation Notes

**Test Scenarios to Cover**:

1. **Ollama Profile Activation**:
   - `EMBEDDING_PROVIDER` unset → `--profile ollama` included
   - `EMBEDDING_PROVIDER=ollama` → `--profile ollama` included
   - `EMBEDDING_PROVIDER=OLLAMA` (case variation) → `--profile ollama` included

2. **Profile Skipping**:
   - `EMBEDDING_PROVIDER=openai` → no profile flags
   - `EMBEDDING_PROVIDER=anthropic` → no profile flags
   - `EMBEDDING_PROVIDER=voyageai` → no profile flags

3. **Edge Cases**:
   - `EMBEDDING_PROVIDER=""` (empty string) → treat as unset
   - Invalid provider value → fallback behavior
   - Multiple providers (if supported in future)

4. **Service List Validation**:
   - With `--profile ollama`: postgres, ollama, maproom-mcp all start
   - Without profiles: postgres, maproom-mcp start; ollama skipped

5. **Backward Compatibility**:
   - Non-profile docker-compose.yml still works
   - Graceful degradation if profiles not supported

**Test Structure Example**:
```javascript
describe('Docker Compose Profile Integration', () => {
  it('should activate ollama profile when EMBEDDING_PROVIDER is unset', () => {
    // Setup: Clear EMBEDDING_PROVIDER
    // Execute: startDockerCompose()
    // Assert: command includes '--profile ollama'
  });

  it('should skip ollama profile when using external provider', () => {
    // Setup: EMBEDDING_PROVIDER=openai
    // Execute: startDockerCompose()
    // Assert: command does NOT include '--profile'
  });

  // ... additional test cases ...
});
```

**Validation Points**:
- Command construction: Verify exact args array
- Service startup: Mock docker compose to verify which services would start
- Diagnostic logs: Assert profile selection is logged correctly
- Error handling: Invalid configurations should produce clear errors

## Dependencies
- **Blocks**: Requires MCPSTART-7001, MCPSTART-7002, and MCPSTART-4001 to be completed first
- **Related**: Extends integration test framework from MCPSTART-4001

## Risk Assessment
- **Risk**: Tests depend on Docker Compose being installed in CI environment
  - **Mitigation**: Mock docker compose calls at spawn level; don't require actual Docker
- **Risk**: Profile behavior differs between Docker Compose versions
  - **Mitigation**: Document tested versions; skip tests if version too old
- **Risk**: Tests become flaky due to container state
  - **Mitigation**: Use mocks instead of real containers; isolated test environments

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/__tests__/integration/profile-startup.test.ts` (new file)
- `/workspace/packages/maproom-mcp/__tests__/integration/docker-compose.test.ts` (if extending existing)
- `/workspace/packages/maproom-mcp/vitest.config.ts` (if test configuration updates needed)
