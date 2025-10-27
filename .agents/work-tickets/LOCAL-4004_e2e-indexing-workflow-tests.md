# Ticket: LOCAL-4004: Run E2E tests for full indexing workflow

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create and execute comprehensive end-to-end tests that validate the complete workflow from starting the Docker stack to indexing a repository and performing searches. This is the final validation that all components work together correctly before release.

## Background
Phase 4 of the LOCAL project requires comprehensive end-to-end testing to validate that the entire system works correctly from startup to search. Individual components have been tested (stack startup, Ollama integration, etc.), but we need to verify the complete user journey: start Docker → index repository → search → retrieve context → persist data.

This ticket is critical for MVP release as it validates the entire value proposition: developers can install with npx, start the stack, index their code, and perform semantic searches with confidence that everything works correctly.

## Acceptance Criteria
- [ ] E2E test suite created (automated, can run in CI/CD)
- [ ] All workflow steps pass successfully (startup → scan → search → context → restart)
- [ ] Indexing completes without errors for test repository (50-100 files)
- [ ] Search returns relevant results (FTS, vector, and hybrid modes)
- [ ] Context assembly works correctly with token budgets respected
- [ ] Data persists across Docker restarts (no re-indexing required)
- [ ] MCP tools functional via stdio transport
- [ ] Test suite runs successfully in GitHub Actions CI/CD pipeline
- [ ] Test execution time documented (should complete within 5-10 minutes)
- [ ] Test results provide clear diagnostics on failure

## Technical Requirements

### Test Repository Setup
- Use a known codebase subset (e.g., 50-100 files from crewchief)
- Mix of file types: TypeScript (.ts), Rust (.rs), Markdown (.md), JSON (.json)
- Known structure for validation (specific functions/classes to search for)
- Test data committed to repository under `tests/fixtures/sample-repo/`

### Workflow Steps to Test

**1. Stack Startup**
- `docker compose up -d` starts all services
- All containers reach healthy state within timeout (90 seconds)
- Ollama has `nomic-embed-text` model ready (verify via API)
- PostgreSQL schema initialized (verify tables exist)

**2. Repository Scanning**
- Scan test repository: `crewchief-maproom scan --repo test-repo --root ./tests/fixtures/sample-repo`
- Files parsed correctly (verify TypeScript, Rust, Markdown parsing)
- Chunks created and stored in database (verify count matches expectations)
- Embeddings generated for all chunks (verify no NULL embeddings)
- Progress logging visible and accurate
- Indexing statistics match expected values

**3. Search Functionality**
- **FTS search**: finds exact function/class names
- **Vector search**: finds semantically similar code (e.g., "error handling")
- **Hybrid search**: combines both correctly, results ranked appropriately
- **Filters work**: `file_type`, `recency`, `worktree` filters apply correctly
- **Relevance**: top results are genuinely relevant to query

**4. Context Assembly**
- Retrieve code chunk with `context` tool
- Related chunks included (imports, callers, callees)
- Token budget respected (output stays within limit)
- Output is useful for LLM consumption (properly formatted, complete functions)

**5. Data Persistence**
- `docker compose down` stops all services
- `docker compose up` restarts stack
- Previously indexed data still accessible via search
- No re-indexing required (verify database state unchanged)

**6. MCP Integration**
- MCP tools respond correctly via stdio transport
- Error handling graceful (test invalid inputs)
- Response format matches MCP specification

### Test Implementation
- Use Rust integration test framework (`tests/e2e_workflow_test.rs`)
- Use `testcontainers` crate for Docker orchestration in tests
- Provide setup/teardown for each test run
- Parallel test execution disabled for E2E tests (use `#[serial]`)
- Clear test output with timing information

### CI/CD Integration
- GitHub Actions workflow (`.github/workflows/e2e-tests.yml`)
- Runs on Ubuntu latest
- Docker Compose installed
- Test timeout: 15 minutes
- Artifacts: test logs, indexing statistics, failure diagnostics

## Implementation Notes

### Test Structure
```rust
// tests/e2e_workflow_test.rs
#[test]
#[serial]
fn test_complete_indexing_workflow() {
    // 1. Setup: Start Docker stack
    // 2. Verify: All services healthy
    // 3. Index: Scan test repository
    // 4. Search: Validate all search modes
    // 5. Context: Test context assembly
    // 6. Persist: Restart and verify data
    // 7. Cleanup: docker compose down
}
```

### Key Validation Points
- **Database state**: Query PostgreSQL directly to verify chunks, embeddings, files
- **Ollama health**: Hit `/api/tags` endpoint to verify model loaded
- **Search quality**: Define expected results for known queries
- **Performance**: Log timing for each phase (startup, indexing, search)

### Test Data Requirements
- Sample repository with known content
- Expected search results documented
- Known relationships for context assembly tests
- Mix of simple and complex code structures

### Error Scenarios to Test
- Ollama not ready (startup timeout)
- PostgreSQL connection failure
- Invalid repository path
- Empty search results
- Malformed MCP requests

### References
- Rust integration testing: https://doc.rust-lang.org/book/ch11-03-test-organization.html
- GitHub Actions: https://docs.github.com/en/actions
- Testcontainers Rust: https://github.com/testcontainers/testcontainers-rs
- MCP specification: https://modelcontextprotocol.io/

## Dependencies
- **LOCAL-3001**: Test npx startup flow (completed) - provides confidence in stack startup
- **LOCAL-2005**: Ollama integration tests (completed) - provides embedding test foundation
- **LOCAL-1005**: Configure health checks (completed) - needed for startup validation
- **LOCAL-3004**: Health check script (completed) - provides programmatic health verification
- Docker Compose installed on test runner
- Test repository fixtures prepared

## Risk Assessment

- **Risk**: E2E tests are flaky due to Docker timing issues
  - **Mitigation**: Use health checks with retries, generous timeouts, clear logging of failure points

- **Risk**: Test suite takes too long (>10 minutes), slows down CI/CD
  - **Mitigation**: Optimize test repository size (50-100 files max), run only critical paths in E2E, use parallel Docker builds

- **Risk**: Test failures difficult to diagnose
  - **Mitigation**: Comprehensive logging at each step, save Docker logs as artifacts, include database dumps on failure

- **Risk**: Tests pass locally but fail in CI/CD
  - **Mitigation**: Use consistent Docker Compose version, document CI environment requirements, test in container locally first

- **Risk**: Search relevance expectations too brittle
  - **Mitigation**: Test for presence of expected results, not exact ranking order; allow fuzzy matching

## Files/Packages Affected
- `tests/e2e_workflow_test.rs` (new) - main E2E test suite
- `tests/fixtures/sample-repo/` (new) - test repository fixtures
- `.github/workflows/e2e-tests.yml` (new) - CI/CD workflow
- `crates/maproom/Cargo.toml` (modified) - add `testcontainers`, `serial_test` dev dependencies
- `tests/test_helpers.rs` (new) - shared test utilities (Docker helpers, assertions)
- `docker-compose.yml` (verify) - ensure test-friendly configuration
- `README.md` (modified) - document how to run E2E tests locally
