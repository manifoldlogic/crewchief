# Ticket: TESTISO-1006: Update documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-implementation
- verify-ticket
- commit-ticket

## Summary
Create comprehensive documentation for test database isolation setup, including developer guides for dual-database architecture, troubleshooting procedures, and volume management.

## Background
With complete test database isolation infrastructure in place (TESTISO-1001 through 1005), developers need clear documentation on how to use the dual-database setup, run tests against the test database, troubleshoot common issues, reset test database, and understand volume management.

This ticket updates existing documentation and creates new guides to ensure smooth developer onboarding and operational clarity.

References Phase 5 (Documentation) from the TESTISO project plan.

## Acceptance Criteria
- [x] `packages/maproom-mcp/README.md` updated with database setup section
- [x] New comprehensive guide created: `docs/development/TEST_DATABASE_SETUP.md`
- [x] Documentation includes database port allocation (5433 dev, 5434 test)
- [x] Documentation includes how to start databases
- [x] Documentation includes how to run tests
- [x] Documentation includes environment variable priority (TEST_MAPROOM_DATABASE_URL)
- [x] Documentation includes troubleshooting common issues
- [x] Documentation includes volume management and reset procedures
- [x] Examples provided for common workflows
- [x] Documentation reviewed for clarity and completeness

## Technical Requirements

### File 1: `packages/maproom-mcp/README.md`

Add **Database Setup Section** (after installation, before usage) containing:
- Development Database (port 5433, usage for dev/manual work)
- Test Database (port 5434, usage for automated tests/CI)
- Starting Databases commands
- Running Tests commands
- Schema Initialization (manual init.sql execution for both databases)
- Link to comprehensive TEST_DATABASE_SETUP.md guide

### File 2: `docs/development/TEST_DATABASE_SETUP.md`

Create **Comprehensive New Guide** with sections:

1. **Overview** - Dual-database architecture explanation
2. **Why Separate Databases?** - Problem/solution narrative
3. **Configuration** - Environment variable priority and hostname resolution table
4. **Common Workflows** - Running tests, resetting test database, running tests against dev database
5. **Troubleshooting** - Connection refused, relation does not exist, port conflicts, CI failures
6. **Volume Management** - List, inspect, remove commands
7. **CI/CD Configuration** - GitHub Actions service container setup
8. **Architecture Details** - Links to planning docs
9. **Validation Script** - Reference to validate-test-isolation.sh

## Implementation Notes

**README.md Updates**:
- Add concise "Database Setup" section for quick reference
- Focus on common tasks developers need daily
- Link to comprehensive guide for detailed information
- Use clear code blocks for all commands

**TEST_DATABASE_SETUP.md**:
- Comprehensive troubleshooting guide covering all common scenarios
- Explain WHY not just HOW for each concept
- Include copy-pasteable commands for every operation
- Use warning symbols (⚠️) for dangerous operations
- Include table for hostname resolution across contexts

**Documentation Style**:
- Use code blocks for all commands
- Clear section headers for easy scanning
- Include examples of expected output
- Progressive disclosure (basics first, advanced topics later)

**Cross-references**:
- Link between README and detailed guide
- Reference architecture docs for design rationale
- Point to validation script for testing isolation
- Link to previous tickets (TESTISO-1001 through 1005)

## Dependencies
**Depends on**:
- TESTISO-1001: Add postgres-test service to docker-compose (infrastructure)
- TESTISO-1002: Update vitest config for test database (configuration)
- TESTISO-1003: Update package.json test scripts (scripts)
- TESTISO-1004: Create validation script (assumed - testing)
- TESTISO-1005: Update CI configuration (assumed - CI/CD)

**Blocks**: Nothing - Final ticket in TESTISO project

## Risk Assessment
- **Risk**: Documentation becomes stale as code changes
  - **Mitigation**: Include "last updated" dates, update docs when changing infrastructure, periodically test commands
- **Risk**: Examples don't work on all platforms
  - **Mitigation**: Test on macOS, Linux, Windows (WSL), provide platform-specific notes where needed
- **Risk**: Too much detail overwhelming for new developers
  - **Mitigation**: README has quick start, detailed guide has deep dive, use progressive disclosure

## Files/Packages Affected
**Modified**:
- `packages/maproom-mcp/README.md`

**Created**:
- `docs/development/TEST_DATABASE_SETUP.md`

## Planning References
- Planning document: `/workspace/.crewchief/projects/TESTISO_test-database-isolation/planning/plan.md` (Phase 5)
- All previous tickets: TESTISO-1001 through 1005 (implementation details)
- Validation script: `/workspace/scripts/validate-test-isolation.sh`
