# Ticket: TESTISO-1004: Create manual validation script

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - script execution validated successfully
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-implementation
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a bash script that validates test database isolation by starting both databases, running tests, and verifying no cross-contamination between test and dev databases.

## Background
With docker infrastructure (TESTISO-1001) and test configuration (TESTISO-1002, TESTISO-1003) in place, we need to verify that test and dev databases are truly isolated. This ticket creates a one-time validation tool that developers can run to confirm the setup works correctly.

The script will validate isolation by:
1. Starting both databases via Docker Compose
2. Running tests against the test database
3. Comparing data counts to verify no cross-contamination

This implements Phase 3 (Manual Validation) from `/workspace/.crewchief/projects/TESTISO_test-database-isolation/planning/plan.md`.

## Acceptance Criteria
- [x] Script created at `/workspace/scripts/validate-test-isolation.sh`
- [x] Script is executable (chmod +x)
- [x] Script validates all critical paths:
  - Docker Compose starts both databases
  - Both databases show as healthy
  - Tests run successfully using test database
  - Database counts can be checked independently
  - Test and dev data are isolated
- [x] Script exits with status 0 on success, non-zero on failure
- [x] Clear output messages show validation progress and results

## Technical Requirements

**File to Create**: `/workspace/scripts/validate-test-isolation.sh`

**Script Must**:
- Use `set -e` to exit on any error
- Add timeout for health checks (30 seconds)
- Navigate using `git rev-parse --show-toplevel` for path resolution
- Handle missing tables gracefully (fresh databases)
- Provide clear numbered steps with status indicators (✅ ❌ ⚠️)
- Query both databases independently for chunk counts
- Validate isolation by comparing counts
- Exit with appropriate status code

**Script Template Structure**:
```bash
#!/bin/bash
set -e

# Step 1: Start Docker Compose infrastructure
# Step 2: Wait for databases to be healthy (with timeout)
# Step 3: Run integration tests with TEST_MAPROOM_DATABASE_URL
# Step 4: Query both databases for chunk counts
# Step 5: Validate isolation (different counts = isolated)
```

**Database Query**:
```bash
docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks"
docker exec maproom-postgres-test psql -U maproom -d maproom_test -t -c "SELECT COUNT(*) FROM maproom.chunks"
```

**Error Handling**:
- Default to count "0" if table doesn't exist (`2>/dev/null || echo "0"`)
- Timeout if health check exceeds 30 seconds
- Clear error messages for troubleshooting
- Graceful handling of empty databases (expected on first run)

## Implementation Notes

**Script Location**:
Place in `/workspace/scripts/` (project root), not in package-specific directory. This is a project-level validation tool that operates across multiple services.

**Path Resolution Strategy**:
- Use `git rev-parse --show-toplevel` to find project root
- Navigate to correct directories for docker-compose and pnpm commands
- Ensure script works when executed from any directory

**Output Clarity**:
- Number each step (1-5) for easy progress tracking
- Use ✅ for success, ❌ for errors, ⚠️ for warnings
- Show actual chunk counts for manual verification
- Explain ambiguous results (e.g., matching counts could be normal)

**Isolation Validation Logic**:
- Different counts = clearly isolated ✅
- Both zero = fresh databases, still isolated (separate instances) ✅
- Same non-zero count = potentially shared, but could be coincidence ⚠️
- Script should explain each scenario

**Docker Compose Path**:
Use `packages/maproom-mcp/config/docker-compose.yml` consistently throughout script.

**Test Execution**:
Run from `packages/maproom-mcp` directory with:
```bash
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test pnpm test:integration
```

## Dependencies

**Depends on**:
- TESTISO-1001 (postgres-test service defined in docker-compose.yml)
- TESTISO-1002 (vitest.config.ts configured with TEST_MAPROOM_DATABASE_URL)
- TESTISO-1003 (package.json test:integration script updated)

**Blocks**:
- TESTISO-1005 (CI validation should use similar approach)

## Risk Assessment

- **Risk**: Script fails on fresh databases (no tables yet)
  - **Mitigation**: Handle missing tables gracefully with `2>/dev/null || echo "0"`, default to count 0
  - **Recovery**: Script explains this is expected behavior on first run

- **Risk**: Path issues when running from different directories
  - **Mitigation**: Use `git rev-parse --show-toplevel` to find project root, navigate explicitly
  - **Validation**: Test script execution from multiple directories before completing ticket

- **Risk**: Timeout too short for slow machines
  - **Mitigation**: 30 second timeout is generous for health checks
  - **Recovery**: User can increase timeout in script if needed (well-commented)

- **Risk**: Script doesn't detect actual isolation issues
  - **Mitigation**: Checks multiple indicators (health, tests passing, separate chunk counts)
  - **Enhancement**: Future iterations can add more sophisticated checks if needed

## Files/Packages Affected

**Created**:
- `/workspace/scripts/validate-test-isolation.sh` (new file)

**No changes required to**:
- Existing test suite
- Docker configuration
- Package.json scripts

## Validation Steps

After implementation, verify:

```bash
# Make script executable
chmod +x /workspace/scripts/validate-test-isolation.sh

# Run script
/workspace/scripts/validate-test-isolation.sh

# Expected output:
# - Both databases healthy
# - Tests pass
# - Chunk counts shown
# - Isolation validated
# - Exit status 0

# Test error handling
docker compose down
/workspace/scripts/validate-test-isolation.sh  # Should fail gracefully with clear error
```

## Planning References
- Planning doc: `/workspace/.crewchief/projects/TESTISO_test-database-isolation/planning/plan.md` (Phase 3)
- Quality strategy: `/workspace/.crewchief/projects/TESTISO_test-database-isolation/planning/quality-strategy.md` (Milestone 1)

## Success Definition

Ticket complete when:
- Script created at `/workspace/scripts/validate-test-isolation.sh`
- Script is executable (chmod +x)
- Script runs successfully and validates isolation
- Clear output shows validation progress with numbered steps
- Script handles errors gracefully (timeouts, missing tables)
- Exit status correctly indicates success (0) or failure (non-zero)
- Script works when executed from any directory
