# TESTENV: Test Environment Infrastructure

## Project Summary

Implement a robust test environment that enables integration tests to run reliably without requiring external dependencies or manual setup. This involves two complementary approaches:

1. **SQL Test Fixtures** - Pre-indexed database fixtures for fast, deterministic tests
2. **Dockerized Daemon** - Optional containerized Rust daemon for true E2E testing

## Problem Statement

The MCP server integration tests require indexed data in the database to verify search, ranking, and schema functionality. Currently:

- Tests that need indexed data fail because the Rust daemon can't connect to the database
- The daemon runs on the host but the database is in Docker (network mismatch)
- No standardized way to provide test fixtures for consistent, fast test runs
- CI environments need a reproducible way to run tests

## Proposed Solution

### Phase 1: SQL Test Fixtures (Fast Path)
Create pre-indexed SQL fixtures that can be loaded during test setup:
- Leverage existing `create_fixture.sh` pattern from `crates/maproom`
- Create a dedicated test corpus fixture with representative chunks
- Load fixtures in `ensure-test-db.ts` after schema initialization
- Most tests run against fixtures (fast, deterministic)

### Phase 2: Dockerized Daemon (E2E Path)
Add the Rust daemon to Docker Compose for full integration testing:
- New `maproom-daemon` service in docker-compose.yml
- Consistent container-to-container networking
- Optional profile for E2E tests that need real indexing
- Works identically in dev and CI environments

## Relevant Agents

- **docker-engineer** - Docker Compose configuration
- **database-engineer** - SQL fixtures and schema
- **typescript-engineer** - Test setup and helper functions

## Planning Documents

- [Analysis](planning/analysis.md) - Problem space and research
- [Architecture](planning/architecture.md) - Solution design
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security considerations
- [Plan](planning/plan.md) - Implementation phases and tickets
