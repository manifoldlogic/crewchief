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

## Secrets Used

Set in repository settings (Settings → Secrets and variables → Actions):
- `NPM_TOKEN` - npm publish auth
- `DOCKER_USERNAME` - Docker Hub username
- `DOCKER_PASSWORD` - Docker Hub token
- `GITHUB_TOKEN` - Auto-provided by GitHub
