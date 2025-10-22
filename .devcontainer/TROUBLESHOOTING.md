# DevContainer Troubleshooting Guide

## Docker Connection Issues

### Error: "Failed to reopen folder in container: Error running `docker info`"

This error occurs when Docker client can't connect to Docker daemon. The client is installed but the server isn't running.

#### Solution Steps:

1. **Start Docker Desktop**
   - Open Docker Desktop application on macOS
   - Wait for the whale icon in menu bar to stop animating
   - The icon should be steady when Docker is ready

2. **Verify Docker is Running**
   ```bash
   docker info
   ```
   You should see both "Client:" and "Server:" sections populated.

3. **Check Docker Context** (if still not working)
   ```bash
   docker context ls
   docker context use desktop-linux
   ```

4. **Restart Docker Desktop** (if needed)
   - Click Docker whale icon in menu bar
   - Select "Quit Docker Desktop"
   - Reopen Docker Desktop
   - Wait for it to fully start (30-60 seconds)

5. **Check Docker Socket** (advanced)
   ```bash
   ls -la /var/run/docker.sock
   # On macOS, it might be:
   ls -la ~/.docker/run/docker.sock
   ```

6. **Reset Docker Desktop** (last resort)
   - Open Docker Desktop
   - Go to Settings → Troubleshoot
   - Click "Clean / Purge data"
   - Restart Docker Desktop

### Common Causes:
- Docker Desktop not running
- Docker Desktop still starting up
- Wrong Docker context selected
- Docker Desktop needs update
- macOS security permissions blocking Docker

### Prevention:
- Always ensure Docker Desktop is running before opening DevContainer
- Check the Docker whale icon is not animating (loading)
- Keep Docker Desktop updated

## Cursor-Specific Issues

### Terminal Shows Host Paths
See [CURSOR_SETUP.md](./CURSOR_SETUP.md) for Cursor-specific configuration.

### Extensions Not Loading
1. Reload window: `Cmd+Shift+P` → "Developer: Reload Window"
2. Check extensions are installed in container, not just locally

### Container Build Fails

#### pgvector Image Pull Issues
If you see errors about `pgvector/pgvector:pg15`:
```bash
# Pull the image manually
docker pull pgvector/pgvector:pg15

# Or use fallback to regular postgres
# Edit docker-compose.yml and change:
# image: pgvector/pgvector:pg15
# to:
# image: postgres:15
```

#### Port Conflicts
If ports are already in use:
1. Check what's using the port:
   ```bash
   lsof -i :3000  # or whatever port
   ```
2. Either stop the conflicting service or change ports in `devcontainer.json`

#### Disk Space Issues
```bash
# Clean up Docker resources
docker system prune -a --volumes
# Warning: This removes all unused containers, images, and volumes
```

## Network Issues

### Can't Access Services
1. Verify services are running:
   ```bash
   docker compose ps
   ```

2. Check logs:
   ```bash
   docker compose logs postgres
   ```

3. Test connectivity from within container:
   ```bash
   # Inside container
   pg_isready -h postgres -p 5432
   ```

### Claude Code Network Restrictions
If Claude Code can't access needed domains:
```bash
# Inside container
sudo /usr/local/bin/init-claude-firewall.sh
```

## Database Issues

### PostgreSQL Won't Start
1. Check logs:
   ```bash
   docker compose logs postgres
   ```

2. Common fixes:
   - Remove volume and recreate: `docker volume rm crewchief_postgres-data`
   - Check permissions on init scripts
   - Verify no other PostgreSQL is running on port 5432

### Migrations Fail
```bash
# For Maproom
PG_DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief" \
crewchief-maproom db
```

## Performance Issues

### Slow on macOS
1. Increase Docker Desktop memory:
   - Docker Desktop → Settings → Resources
   - Increase Memory to at least 8GB
   - Increase CPUs to at least 4

2. Use cached mounts (already configured in our setup)

3. Exclude large directories from file watching in VS Code/Cursor settings

### Build Takes Too Long
1. Use Docker BuildKit:
   ```bash
   export DOCKER_BUILDKIT=1
   ```

2. Leverage cache by not changing early Dockerfile layers

3. Use `.dockerignore` to exclude unnecessary files

## Getting Help

1. Check container logs:
   ```bash
   docker compose logs -f devcontainer
   ```

2. Inspect running container:
   ```bash
   docker exec -it <container-name> /bin/bash
   ```

3. Report issues with full error messages and:
   - OS version
   - Docker Desktop version
   - Cursor/VS Code version
   - Output of `docker version` and `docker info`