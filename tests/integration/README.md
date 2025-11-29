# Integration Tests

This directory contains integration tests for the CrewChief project.

## Available Tests

### DKRHUB-2902: Production Configuration (Image Pull)

**Script**: `test-production-docker-hub.sh`
**Purpose**: Verify that docker-compose.yml successfully pulls pre-built images from Docker Hub
**Status**: Ready for execution (currently blocked by DKRHUB-1901)

#### Quick Start

```bash
# Run the test (requires Docker Hub images to be published)
./test-production-docker-hub.sh

# Run with specific version
MAPROOM_VERSION=1.1.10-rc1 ./test-production-docker-hub.sh
```

#### Current Status

The test infrastructure is complete and production-ready. However, execution is currently **BLOCKED** because Docker Hub images have not been published yet.

**Blocker**: DKRHUB-1901 - Images must be published to Docker Hub first

#### How to Unblock

1. Execute DKRHUB-1901 test plan (requires GitHub push access)
2. Create and push tag: `git tag -a v1.1.10-rc1 -m "Test release" && git push origin v1.1.10-rc1`
3. Monitor GitHub Actions: https://github.com/danielbushman/crewchief/actions
4. Verify images published: `docker pull crewchief/maproom-mcp:1.1.10-rc1`
5. Re-run this test

#### Test Coverage

The test validates:
- ✅ Clean Docker environment setup
- ✅ Image pull from Docker Hub (not local build)
- ✅ All three services start (maproom-mcp, maproom-postgres, maproom-ollama)
- ✅ Health checks pass within 60 seconds
- ✅ No build errors in docker-compose output
- ✅ Successful service startup logs
- ✅ Images from crewchief/maproom-mcp repository

#### Documentation

- **Test Results**: `DKRHUB-2902_TEST_RESULTS.md` - Detailed test execution summary
- **Test Script**: `test-production-docker-hub.sh` - Automated test implementation
- **Ticket**: `.crewchief/work-tickets/DKRHUB-2902_test-production-config-image-pull.md`

#### Exit Codes

- `0` - All tests passed
- `1` - Tests failed (see output for details)
- `2` - Tests blocked (dependency not met)

## Test Requirements

- Docker 24.0+
- Internet connection (for image pull)
- ~500MB disk space
- No conflicting services on ports 5433, 11434

## Running All Tests

```bash
# Future: Run all integration tests
# ./run-all-tests.sh
```
