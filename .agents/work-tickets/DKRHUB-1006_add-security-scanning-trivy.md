# Ticket: DKRHUB-1006: Add Security Scanning with Trivy

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

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
- [x] Trivy scan step added using `aquasecurity/trivy-action@master`
- [x] Scan targets the published image `crewchief/maproom-mcp:${{ steps.version.outputs.full }}`
- [x] Output format set to `sarif` for GitHub Security
- [x] Severity filter set to `CRITICAL,HIGH`
- [x] Scan results uploaded to GitHub Security tab using `github/codeql-action/upload-sarif@v2`
- [x] Exit code configured to fail build on critical vulnerabilities (exit-code: 1)
- [x] Upload step runs even if scan fails (if: always())

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

**Combined Dockerfile Scanning**:
Trivy scans the combined image containing both Rust and Node.js components:
- Rust runtime dependencies: libgcc, libssl3, ca-certificates
- Node.js dependencies: pg, pino, zod, execa
- Alpine base packages
- Comprehensive coverage of all runtime components

Reference DKRHUB_SECURITY_REVIEW.md lines 368-402 for vulnerability response process.

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1005**: Image must be built and pushed before scanning
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

## Implementation Notes

**Changes Made**:
- Replaced placeholder steps (lines 115-117) with two new security scanning steps
- Step 8 (lines 115-123): "Run Trivy security scan"
  - Uses `aquasecurity/trivy-action@master`
  - Scans image: `crewchief/maproom-mcp:${{ steps.version.outputs.full }}`
  - Outputs SARIF format to `trivy-results.sarif`
  - Filters for CRITICAL,HIGH severity only
  - Configured with `exit-code: 1` to fail build on findings
- Step 9 (lines 125-130): "Upload Trivy results to GitHub Security"
  - Uses `github/codeql-action/upload-sarif@v2`
  - Runs with `if: always()` to upload even if scan fails
  - Uploads `trivy-results.sarif` file

**YAML Validation**:
- Syntax validated using Python YAML parser - confirmed valid

**Verification Commands**:
```bash
# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/publish-maproom-mcp-image.yml'))"

# View the implemented steps
sed -n '115,130p' .github/workflows/publish-maproom-mcp-image.yml
```

**Integration Points**:
- Depends on Step 7 (build-and-push) completing successfully to have an image to scan
- Uses `env.DOCKER_HUB_REPO` and `steps.version.outputs.full` from earlier steps
- Requires `security-events: write` permission (already configured at line 23)
- Results will appear in GitHub Security tab under "Code scanning alerts"

**Expected Behavior**:
1. After image is pushed to Docker Hub, Trivy scans it for vulnerabilities
2. If CRITICAL or HIGH vulnerabilities found, workflow fails (exit-code: 1)
3. SARIF results always uploaded to GitHub Security tab (even on failure)
4. Security alerts visible in repository Security → Code scanning alerts

**All Acceptance Criteria Met**:
- ✓ Trivy scan step added using `aquasecurity/trivy-action@master`
- ✓ Scan targets `crewchief/maproom-mcp:${{ steps.version.outputs.full }}`
- ✓ Output format set to `sarif`
- ✓ Severity filter set to `CRITICAL,HIGH`
- ✓ Scan results uploaded using `github/codeql-action/upload-sarif@v2`
- ✓ Exit code configured to fail build (exit-code: 1)
- ✓ Upload step runs even if scan fails (if: always())
