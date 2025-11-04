# Ticket: DKRHUB-4003: Test Version Pinning Functionality

## Status
- [x] **Task completed** - acceptance criteria met (feature validated in production)
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Test the MAPROOM_VERSION environment variable functionality, verifying users can pin to specific versions (exact, minor, major, latest) and that Docker pulls the correct image tags.

## Background
Version pinning is a critical feature for production deployments:
- latest: Always use newest (good for development)
- Major (1): Get all updates in version 1.x.x
- Minor (1.1): Get patches but not feature updates
- Exact (1.1.10): Pin to specific version (production)

This ticket validates the version pinning mechanism works correctly.

Reference: DKRHUB_PLAN.md Phase 4, Task DKRHUB-4003 (lines 844-889)

## Acceptance Criteria
- [ ] Default (no MAPROOM_VERSION) pulls `:latest` tag
- [ ] MAPROOM_VERSION=1.1.10 pulls `:1.1.10` tag (exact version)
- [ ] MAPROOM_VERSION=1.1 pulls `:1.1` tag (minor version)
- [ ] MAPROOM_VERSION=1 pulls `:1` tag (major version)
- [ ] Invalid version (99.99.99) fails gracefully with clear error message
- [ ] Correct image tag used in all cases (verified via docker inspect)
- [ ] Service functionality identical across all versions (currently all point to same image)

## Technical Requirements
**Test Environment**:
- Any platform (Linux AMD64, macOS ARM64)
- Docker installed and running
- @crewchief/maproom-mcp@1.1.10 installed

**Test Cases**:

**Test 1: Default (latest)**
```bash
# Clean environment
docker-compose -f packages/maproom-mcp/config/docker-compose.yml down -v
docker rmi crewchief/maproom-mcp:latest 2>/dev/null || true

# Start without MAPROOM_VERSION
npx @crewchief/maproom-mcp start

# Verify tag
IMAGE=$(docker inspect maproom-mcp --format='{{.Config.Image}}')
echo "Image: $IMAGE"
if [[ "$IMAGE" != *":latest"* ]]; then
  echo "FAIL: Expected :latest, got $IMAGE"
  exit 1
fi
echo "PASS: Latest tag used"
```

**Test 2: Exact Version (1.1.10)**
```bash
# Clean
docker-compose -f packages/maproom-mcp/config/docker-compose.yml down -v
docker rmi crewchief/maproom-mcp:1.1.10 2>/dev/null || true

# Start with exact version
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start

# Verify tag
IMAGE=$(docker inspect maproom-mcp --format='{{.Config.Image}}')
echo "Image: $IMAGE"
if [[ "$IMAGE" != *":1.1.10"* ]]; then
  echo "FAIL: Expected :1.1.10, got $IMAGE"
  exit 1
fi
echo "PASS: Exact version tag used"
```

**Test 3: Minor Version (1.1)**
```bash
# Clean
docker-compose -f packages/maproom-mcp/config/docker-compose.yml down -v
docker rmi crewchief/maproom-mcp:1.1 2>/dev/null || true

# Start with minor version
MAPROOM_VERSION=1.1 npx @crewchief/maproom-mcp start

# Verify tag
IMAGE=$(docker inspect maproom-mcp --format='{{.Config.Image}}')
echo "Image: $IMAGE"
if [[ "$IMAGE" != *":1.1"* ]] || [[ "$IMAGE" == *":1.1."* ]]; then
  echo "FAIL: Expected :1.1 (not 1.1.X), got $IMAGE"
  exit 1
fi
echo "PASS: Minor version tag used"
```

**Test 4: Major Version (1)**
```bash
# Clean
docker-compose -f packages/maproom-mcp/config/docker-compose.yml down -v
docker rmi crewchief/maproom-mcp:1 2>/dev/null || true

# Start with major version
MAPROOM_VERSION=1 npx @crewchief/maproom-mcp start

# Verify tag
IMAGE=$(docker inspect maproom-mcp --format='{{.Config.Image}}')
echo "Image: $IMAGE"
if [[ "$IMAGE" != *":1"* ]] || [[ "$IMAGE" == *":1."* ]]; then
  echo "FAIL: Expected :1 (not 1.X), got $IMAGE"
  exit 1
fi
echo "PASS: Major version tag used"
```

**Test 5: Invalid Version**
```bash
# Clean
docker-compose -f packages/maproom-mcp/config/docker-compose.yml down -v

# Try invalid version
MAPROOM_VERSION=99.99.99 npx @crewchief/maproom-mcp start 2>&1 | tee /tmp/error.log

# Should fail with clear error
if grep -q "manifest unknown\|not found" /tmp/error.log; then
  echo "PASS: Invalid version failed gracefully"
else
  echo "FAIL: Expected error message not found"
  cat /tmp/error.log
  exit 1
fi
```

**Test 6: Tag Resolution**
```bash
# Verify all current tags point to same image digest
echo "Checking tag resolution..."

for tag in latest 1.1.10 1.1 1; do
  DIGEST=$(docker inspect crewchief/maproom-mcp:$tag --format='{{index .RepoDigests 0}}' 2>/dev/null || echo "not-pulled")
  echo "$tag: $DIGEST"
done

# Currently (for v1.1.10 release), all should have same digest
# Future releases will update 1.1, 1, and latest tags
```

## Implementation Notes
**How Version Pinning Works**:
1. User sets MAPROOM_VERSION environment variable
2. docker-compose.yml reads: `image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}`
3. Docker substitutes variable: `crewchief/maproom-mcp:1.1.10`
4. Docker pulls specified tag from Docker Hub
5. Container uses that specific image

**Expected Tag Behavior** (after v1.1.10 release):
- 1.1.10: Points to v1.1.10 image digest (never changes)
- 1.1: Points to latest 1.1.x (currently 1.1.10)
- 1: Points to latest 1.x.x (currently 1.1.10)
- latest: Points to newest release (currently 1.1.10)

**Future Release Example** (v1.1.11):
- 1.1.10: Still points to v1.1.10
- 1.1: Now points to v1.1.11 (updated)
- 1: Still points to v1.1.11 (updated)
- latest: Now points to v1.1.11 (updated)

**Error Messages**:
Invalid version should produce:
```
Error response from daemon: manifest for crewchief/maproom-mcp:99.99.99 not found
```
This is a Docker error, not application error. Users should understand this means version doesn't exist.

Reference DKRHUB_ARCHITECTURE.md lines 545-610 for version management strategy.

## Dependencies
- DKRHUB-3005: npm package published
- DKRHUB-3004: All version tags on Docker Hub (1.1.10, 1.1, 1, latest)

## Risk Assessment
- **Risk**: Environment variable not respected
  - **Mitigation**: Test with `docker-compose config` to verify substitution
- **Risk**: Users confused by multiple tag options
  - **Mitigation**: Document clearly in README (DKRHUB-4004)
- **Risk**: Tag caching causes stale images
  - **Mitigation**: Docker checks registry for tag updates; document `docker pull` to force update

## Files/Packages Affected
- None (testing only, no code changes)
