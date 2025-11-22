# CLAUDE.md - .github Directory

Working with GitHub workflows at `/.github`.

## Active Workflows

```
workflows/
├── build-and-publish-maproom-mcp.yml  # npm publish
└── publish-maproom-mcp-image.yml      # Docker images
```

## Workflows

### `build-and-publish-maproom-mcp.yml`
Publishes `@crewchief/maproom-mcp` to npm.
- Trigger: Version tags or manual dispatch
- Builds TypeScript and publishes package

### `publish-maproom-mcp-image.yml`
Builds and publishes Docker images.
- Multi-platform: linux/amd64, linux/arm64
- Pushes to Docker Hub or GitHub Container Registry

## Debug Failed Workflow

```bash
# View logs
gh run list --workflow=build-and-publish-maproom-mcp.yml
gh run view <run-id> --log

# Re-run
gh run rerun <run-id>
```

## Troubleshooting Workflows

### Test Workflow

**pnpm Version Management:**
- pnpm version is auto-detected from `package.json` packageManager field
- To change pnpm version: Update `package.json` ONLY (not workflow YAML)
- Do NOT add explicit `version:` to `pnpm/action-setup@v4`

**Common Issues:**

#### "Multiple versions of pnpm specified"
- **Cause**: Explicit version in workflow + packageManager in package.json
- **Fix**: Remove explicit `with: version:` from `.github/workflows/test.yml`
- **Prevention**: Never add version field to pnpm/action-setup step

#### "pnpm command not found"
- **Cause**: packageManager field missing or malformed
- **Fix**: Verify `jq -r '.packageManager' package.json` returns valid value
- **Format**: Must be `pnpm@<version>+sha512...`
- **Verify**: `grep -A 3 "pnpm/action-setup" .github/workflows/test.yml` should not show `with: version:`

---

### Docker Build

**Prerequisites:**
- **CRITICAL**: Run `pnpm build` before `docker build`
- daemon-client must be compiled to dist/ directory
- Failure results in "COPY failed: file not found" error

**Common Issues:**

#### "Unsupported URL Type workspace:"
- **Cause**: npm used instead of pnpm (doesn't understand workspace: protocol)
- **Fix**: Verify Dockerfile has `RUN npm install -g pnpm@10.12.1`
- **Verify**: `grep "npm install -g pnpm" packages/maproom-mcp/config/Dockerfile.combined`

#### "daemon-client dist not found" in Docker
- **Cause**: daemon-client dist/ not built or not copied to Docker context
- **Fix**: Run `pnpm build` at repository root before `docker build`
- **Verify**: `ls -la packages/daemon-client/dist/` shows index.js, client.js

#### "pnpm version mismatch" warning
- **Cause**: Dockerfile pnpm version doesn't match package.json
- **Fix**: Update Dockerfile line 41: `RUN npm install -g pnpm@<version>`
- **Check versions**:
  ```bash
  PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
  DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')
  echo "package.json: $PACKAGE_PNPM"
  echo "Dockerfile: $DOCKERFILE_PNPM"
  ```

---

### Best Practices

**Updating pnpm Version:**
1. Update `package.json` packageManager field
2. Update `packages/maproom-mcp/config/Dockerfile.combined` (line 41)
3. Verify versions match with validation script above
4. Test locally before pushing to CI

**Before Pushing:**
- Run `yamllint .github/workflows/*.yml` to validate workflow syntax
- Run `pnpm build` to ensure workspace packages build
- Run local Docker build to verify Dockerfile changes

## Common CI Issues

### Debugging Test Workflow Failures

**Step-by-step diagnosis:**

1. **Check workflow logs** in GitHub Actions:
   ```bash
   gh run list --workflow=test.yml --limit 5
   gh run view <run-id> --log
   ```

2. **Verify pnpm setup step**:
   - Look for "Setup pnpm" in logs
   - Check detected version matches package.json
   - Expected: `pnpm version 10.12.1`

3. **Verify packageManager field**:
   ```bash
   jq -r '.packageManager' package.json
   # Should show: pnpm@10.12.1+sha512...
   ```

4. **Check for explicit version in workflow**:
   ```bash
   grep -A 5 "pnpm/action-setup" .github/workflows/test.yml
   # Should NOT see: with: version:
   ```

5. **Rollback if needed**:
   ```bash
   git log --oneline .github/workflows/test.yml
   git revert <commit-sha>
   git push
   ```

---

### Debugging Docker Build Failures

**Step-by-step diagnosis:**

1. **Verify daemon-client dist/ exists**:
   ```bash
   ls -la packages/daemon-client/dist/
   # Must show: index.js, index.d.ts, client.js, client.d.ts
   ```

2. **Check pnpm version sync**:
   ```bash
   PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
   DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')

   if [ "$PACKAGE_PNPM" != "$DOCKERFILE_PNPM" ]; then
     echo "❌ Version mismatch: $PACKAGE_PNPM vs $DOCKERFILE_PNPM"
   else
     echo "✅ Versions match: $PACKAGE_PNPM"
   fi
   ```

3. **Test local Docker build**:
   ```bash
   pnpm build  # Ensure daemon-client built

   docker build \
     -f packages/maproom-mcp/config/Dockerfile.combined \
     -t maproom-mcp:debug \
     --progress=plain \
     .
   ```

4. **Check for common errors**:
   - "EUNSUPPORTEDPROTOCOL" → pnpm not installed in Dockerfile
   - "COPY failed" → daemon-client dist/ missing (run pnpm build)
   - "workspace: not resolved" → Missing pnpm-workspace.yaml in COPY

5. **Rollback Docker changes**:
   ```bash
   git log --oneline packages/maproom-mcp/config/Dockerfile.combined
   git revert <commit-sha>
   docker build -f packages/maproom-mcp/config/Dockerfile.combined -t rollback .
   ```

---

### CI Health Check

Run these commands to verify CI configuration is correct:

```bash
# Test workflow health
yamllint .github/workflows/test.yml
jq -r '.packageManager' package.json
grep -c "with: version:" .github/workflows/test.yml  # Should be 0

# Docker build health
pnpm build
ls -la packages/daemon-client/dist/ | wc -l  # Should show multiple files
grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined

# Release workflow health
grep -A 10 "pnpm build" .github/workflows/publish-maproom-mcp-image.yml
# Should show pnpm build step before Docker build

echo "✅ All health checks passed"
```

---

### Emergency Rollback Procedures

**If test workflow broken:**
```bash
# Option 1: Revert to previous workflow
git revert <commit-sha-of-fix>
git push

# Option 2: Temporarily add explicit version (not recommended long-term)
# Edit .github/workflows/test.yml:
# - name: Setup pnpm
#   uses: pnpm/action-setup@v4
#   with:
#     version: 10  # Temporary fix while debugging
```

**If Docker build broken:**
```bash
# Revert Dockerfile changes
git revert <commit-sha-of-dockerfile-changes>
git push

# If release is urgent, manually publish previous image:
docker pull <previous-good-image>
docker tag <previous-good-image> <registry>:<new-tag>
docker push <registry>:<new-tag>
```

**Validation after rollback:**
- Test workflow: Trigger manual run in GitHub Actions
- Docker build: Run local build and verify success
- Release workflow: Create test tag and monitor build

## Secrets Used

Set in repository settings (Settings → Secrets and variables → Actions):
- `NPM_TOKEN` - npm publish auth
- `DOCKER_USERNAME` - Docker Hub username
- `DOCKER_PASSWORD` - Docker Hub token
- `GITHUB_TOKEN` - Auto-provided by GitHub
