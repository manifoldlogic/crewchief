# Ticket: DKRHUB-1006: Add Security Scanning with Trivy

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Integrate Trivy security scanning into the workflow to automatically detect vulnerabilities in published images and upload results to GitHub Security tab.

## Background
Security scanning is a critical quality gate that prevents publishing images with known vulnerabilities. Trivy scans for:
- OS package vulnerabilities (Alpine packages)
- Application dependencies (npm packages)
- Misconfigurations
- Exposed secrets

This ticket adds automated security scanning as part of the CI/CD pipeline.

Reference: DKRHUB_PLAN.md Phase 1, Task DKRHUB-1006 (lines 247-279)

## Acceptance Criteria
- [ ] Trivy scan step added using `aquasecurity/trivy-action@master`
- [ ] Scan targets the published image `crewchief/maproom-mcp:${{ steps.version.outputs.full }}`
- [ ] Output format set to `sarif` for GitHub Security
- [ ] Severity filter set to `CRITICAL,HIGH`
- [ ] Scan results uploaded to GitHub Security tab using `github/codeql-action/upload-sarif@v2`
- [ ] Exit code configured to fail build on critical vulnerabilities (exit-code: 1)
- [ ] Upload step runs even if scan fails (if: always())

## Technical Requirements
- Step 1: "Run Trivy security scan"
  - uses: `aquasecurity/trivy-action@master`
  - inputs:
    - image-ref: `${{ env.DOCKER_HUB_REPO }}:${{ steps.version.outputs.full }}`
    - format: `sarif`
    - output: `trivy-results.sarif`
    - severity: `CRITICAL,HIGH`
    - exit-code: `1` (fail build on findings)

- Step 2: "Upload Trivy results to GitHub Security"
  - uses: `github/codeql-action/upload-sarif@v2`
  - if: `always()` (run even if scan fails)
  - inputs:
    - sarif_file: `trivy-results.sarif`

## Implementation Notes
**Severity Levels**:
- CRITICAL: Immediate action required (block release)
- HIGH: Should fix before release (review required)
- MEDIUM/LOW: Track and fix in future releases

**Exit Code Strategy**:
- exit-code: 1 causes build failure if CRITICAL or HIGH found
- This is a blocking quality gate - images won't publish with vulnerabilities
- Override possible via workflow_dispatch for emergency releases (not recommended)

**SARIF Upload**:
- SARIF (Static Analysis Results Interchange Format) is GitHub's standard
- Results appear in repository Security tab under "Code scanning alerts"
- Provides detailed vulnerability information with remediation advice

**Performance Impact**:
- Trivy scan adds ~2-3 minutes to workflow
- Acceptable overhead for security assurance

Reference DKRHUB_SECURITY_REVIEW.md lines 368-402 for vulnerability response process.

## Dependencies
- DKRHUB-1005: Image must be built and pushed before scanning
- Prerequisite: Repository must have security-events: write permission (configured in DKRHUB-1001)

## Risk Assessment
- **Risk**: False positives blocking releases
  - **Mitigation**: Review findings; can temporarily adjust severity filter if needed
- **Risk**: Trivy database outdated
  - **Mitigation**: Trivy auto-updates vulnerability database on each run
- **Risk**: Scan timeout on large images
  - **Mitigation**: Alpine-based image (~300MB) scans quickly; not a concern

## Files/Packages Affected
- `.github/workflows/publish-maproom-mcp-image.yml` (add Trivy scan and upload steps)
