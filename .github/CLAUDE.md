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

## Secrets Used

Set in repository settings (Settings → Secrets and variables → Actions):
- `NPM_TOKEN` - npm publish auth
- `DOCKER_USERNAME` - Docker Hub username
- `DOCKER_PASSWORD` - Docker Hub token
- `GITHUB_TOKEN` - Auto-provided by GitHub
