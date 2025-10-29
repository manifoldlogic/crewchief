# Ticket: DKRHUB-1901: Test Workflow with Pre-Release Tag

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Test the complete GitHub Actions workflow end-to-end using a pre-release tag (v1.1.10-rc1) to verify all steps execute successfully before the production v1.1.10 release.

## Background
Before tagging the production v1.1.10 release, we must validate that the entire workflow functions correctly:
- Triggers properly on tag push
- All build steps complete
- Multi-platform images build successfully
- Images push to Docker Hub
- Security scan passes
- No secrets leaked in logs

This is a critical validation step to prevent releasing broken infrastructure.

Reference: DKRHUB_PLAN.md Phase 1, Task DKRHUB-1007 (lines 281-316)

## Acceptance Criteria
- [ ] Test tag created: `v1.1.10-rc1` and pushed to repository
- [ ] GitHub Actions workflow triggers automatically on tag push
- [ ] All workflow steps complete without errors (checkout, QEMU, buildx, login, version, metadata, build-push, trivy, upload)
- [ ] Images appear on Docker Hub at `crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Both AMD64 and ARM64 images exist (verified via `docker manifest inspect`)
- [ ] Images can be pulled successfully: `docker pull crewchief/maproom-mcp:1.1.10-rc1`
- [ ] No credentials visible in GitHub Actions logs (DOCKERHUB_USERNAME, DOCKERHUB_TOKEN redacted)
- [ ] Trivy scan results uploaded to GitHub Security tab
- [ ] Build completes in <20 minutes

## Technical Requirements
- Create git tag: `git tag -a v1.1.10-rc1 -m "Test release for workflow validation"`
- Push tag: `git push origin v1.1.10-rc1`
- Monitor workflow at: `https://github.com/danielbushman/crewchief/actions`
- Verification commands:
  ```bash
  # Pull and test image
  docker pull crewchief/maproom-mcp:1.1.10-rc1

  # Verify platforms
  docker manifest inspect crewchief/maproom-mcp:1.1.10-rc1 | jq '.manifests[].platform'

  # Check metadata
  docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{json .Config.Labels}}' | jq

  # Test run
  docker run --rm crewchief/maproom-mcp:1.1.10-rc1 --help
  ```

## Implementation Notes
**Test Strategy** (from DKRHUB_QUALITY_STRATEGY.md):
- This is a Level 1 test (Image Build Validation)
- Validates workflow infrastructure before production release
- Non-blocking: Test tag can be deleted if issues found

**What to Check**:
1. GitHub Actions logs:
   - No errors in any step
   - Secrets properly redacted
   - Build time reasonable (<20 min)
   - Cache utilization (should see "Cache hit" messages)

2. Docker Hub:
   - Tag exists: 1.1.10-rc1
   - Two manifests (AMD64, ARM64)
   - Image size ~300MB
   - Metadata labels present

3. GitHub Security:
   - Trivy results uploaded
   - No critical vulnerabilities (0 critical required)

**Rollback Plan**:
If test fails:
1. Review workflow logs to identify failure point
2. Fix issue in workflow YAML
3. Delete test tag: `git tag -d v1.1.10-rc1 && git push origin :refs/tags/v1.1.10-rc1`
4. Delete test images from Docker Hub (via web UI)
5. Re-run test with new tag (v1.1.10-rc2)

**Success Criteria**:
All checks pass → Proceed to Phase 2 (docker-compose updates)
Any failures → Fix and retest before proceeding

## Dependencies
- DKRHUB-1001 through DKRHUB-1006: All workflow steps must be implemented
- Prerequisite: GitHub Secrets configured
- Prerequisite: Docker Hub account exists

## Risk Assessment
- **Risk**: Test tag pollutes production Docker Hub
  - **Mitigation**: Use -rc1 suffix; clearly labeled as test; can be deleted
- **Risk**: Workflow failures block progress
  - **Mitigation**: This is intentional - better to find issues now than in production release
- **Risk**: Secrets exposed during test
  - **Mitigation**: GitHub automatically redacts secret values in logs; verify manually

## Files/Packages Affected
- None (testing only, no code changes)
- Creates test artifacts: git tag v1.1.10-rc1, Docker images on Docker Hub
