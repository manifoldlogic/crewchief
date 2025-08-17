# CrewChief Web UI Docker Setup

This document provides comprehensive instructions for running the CrewChief Web UI using Docker and Docker Compose.

## Quick Start

### Development Setup

1. **Copy and configure environment variables:**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

2. **Start development environment:**
   ```bash
   ./scripts/docker-run.sh up
   ```

3. **Access the application:**
   - Web UI: http://localhost:3456
   - Frontend Dev Server: http://localhost:3000
   - pgAdmin: http://localhost:8080 (with dev-tools profile)
   - Redis Commander: http://localhost:8081 (with dev-tools profile)

### Production Setup

1. **Configure production environment:**
   ```bash
   cp .env.example .env
   # Set production values in .env
   ```

2. **Start production environment:**
   ```bash
   ./scripts/docker-run.sh up -e production -d
   ```

3. **Access the application:**
   - Web UI: http://localhost:3456

## Architecture Overview

The Docker setup includes:

- **PostgreSQL**: Database for both Maproom and Web UI data
- **Redis**: Caching and session storage
- **Web UI**: Node.js/Express backend + React frontend
- **pgAdmin**: Database management (development only)
- **Redis Commander**: Redis management (development only)

## Docker Images

### Production Image (`Dockerfile`)
- Multi-stage build for optimal size
- Node.js 20 Alpine base
- Non-root user for security
- Health checks included
- Security hardened

### Development Image (`Dockerfile.dev`)
- Hot reload support
- Debug port exposed
- Development dependencies included
- Volume mounting for source code

## Scripts

### Build Script (`scripts/docker-build.sh`)

Build Docker images with various options:

```bash
# Build production image
./scripts/docker-build.sh

# Build development image
./scripts/docker-build.sh -t development

# Build with no cache and push
./scripts/docker-build.sh --no-cache --push

# Build and tag as latest
./scripts/docker-build.sh --tag-latest
```

Options:
- `-t, --type`: `development` or `production`
- `-p, --platform`: Target platform (default: `linux/amd64`)
- `--no-cache`: Don't use Docker cache
- `--push`: Push to registry
- `--tag-latest`: Tag as latest

### Run Script (`scripts/docker-run.sh`)

Manage Docker Compose deployments:

```bash
# Start development environment
./scripts/docker-run.sh up

# Start production environment
./scripts/docker-run.sh up -e production -d

# Start with development tools
./scripts/docker-run.sh up --profile dev-tools

# View logs
./scripts/docker-run.sh logs web-ui

# Execute commands in container
./scripts/docker-run.sh exec web-ui pnpm test

# Open shell in container
./scripts/docker-run.sh shell

# Stop services
./scripts/docker-run.sh down

# Clean up everything
./scripts/docker-run.sh clean
```

Commands:
- `up`: Start services
- `down`: Stop and remove services
- `restart`: Restart services
- `logs`: Show service logs
- `ps`: Show running services
- `exec`: Execute command in container
- `shell`: Open shell in web-ui container
- `build`: Build images
- `pull`: Pull latest images
- `clean`: Clean up volumes and networks

Options:
- `-e, --env`: Environment (`development` or `production`)
- `-d, --detach`: Run in background
- `-b, --build`: Build images before starting
- `-p, --pull`: Pull latest images
- `-r, --recreate`: Recreate containers
- `--profile`: Use specific compose profile

## Environment Configuration

### Required Variables

Copy `.env.example` to `.env` and configure:

```bash
# Database
CREWCHIEF_DB_PASSWORD=your_secure_password
POSTGRES_PASSWORD=your_secure_password

# Redis
REDIS_PASSWORD=your_redis_password

# Security (CRITICAL for production)
SESSION_SECRET=your_session_secret_change_in_production
JWT_SECRET=your_jwt_secret_change_in_production
```

### Development vs Production

**Development** (`.env` defaults):
- Exposes debug ports
- Enables hot reload
- Includes development tools
- Uses separate database (`crewchief_dev`)
- Different ports to avoid conflicts

**Production**:
- Optimized for security and performance
- No debug ports exposed
- Minimal attack surface
- Health checks enabled
- Resource limits applied

## Docker Compose Profiles

### Default Profile
Includes core services:
- PostgreSQL
- Redis
- Web UI

### Dev Tools Profile
Additional services for development:
- pgAdmin (database management)
- Redis Commander (Redis management)

Start with dev tools:
```bash
./scripts/docker-run.sh up --profile dev-tools
```

## Health Checks

All services include health checks:

- **PostgreSQL**: `pg_isready` check
- **Redis**: `redis-cli ping` check
- **Web UI**: HTTP health endpoint check

Check service health:
```bash
docker compose ps
```

## Networking

### Development
- Network: `crewchief-dev-network`
- Ports exposed for direct access
- Services communicate via service names

### Production
- Network: `crewchief-network`
- Minimal port exposure
- Internal service communication only

## Data Persistence

### Volumes

**Development:**
- `crewchief-postgres-dev-data`: PostgreSQL data
- `crewchief-redis-dev-data`: Redis data
- `crewchief-pgadmin-data`: pgAdmin configuration
- `crewchief-web-ui-node-modules`: Node.js dependencies

**Production:**
- `crewchief-postgres-data`: PostgreSQL data
- `crewchief-redis-data`: Redis data

### Backup Strategy

**Database Backup:**
```bash
# Create backup
docker compose exec postgres pg_dump -U postgres crewchief > backup.sql

# Restore backup
docker compose exec -T postgres psql -U postgres crewchief < backup.sql
```

**Volume Backup:**
```bash
# Backup volumes
docker run --rm -v crewchief-postgres-data:/data -v $(pwd):/backup alpine tar czf /backup/postgres-backup.tar.gz -C /data .

# Restore volumes
docker run --rm -v crewchief-postgres-data:/data -v $(pwd):/backup alpine tar xzf /backup/postgres-backup.tar.gz -C /data
```

## Security Considerations

### Production Security

1. **Environment Variables:**
   - Use strong passwords
   - Generate secure secrets
   - Never commit `.env` to version control

2. **Network Security:**
   - Use reverse proxy (nginx/traefik)
   - Enable SSL/TLS termination
   - Configure firewalls

3. **Container Security:**
   - Non-root user execution
   - Read-only root filesystem where possible
   - Security scanning with `docker scout`

4. **Database Security:**
   - Strong passwords
   - Limited connection access
   - Regular security updates

### Security Scanning

Run security scans on built images:
```bash
# Scan for vulnerabilities
docker scout quickview crewchief/web-ui:latest

# Detailed security report
docker scout cves crewchief/web-ui:latest
```

## Troubleshooting

### Common Issues

**1. Port Conflicts:**
```bash
# Check what's using ports
lsof -i :3456
lsof -i :5432

# Use different ports in .env
WEB_UI_PORT=3457
CREWCHIEF_DB_PORT=5433
```

**2. Permission Issues:**
```bash
# Fix volume permissions
docker compose exec web-ui chown -R crewchief:nodejs /app
```

**3. Database Connection:**
```bash
# Check database logs
./scripts/docker-run.sh logs postgres

# Test connection
./scripts/docker-run.sh exec postgres psql -U postgres -c "SELECT version();"
```

**4. Memory Issues:**
```bash
# Check container resource usage
docker stats

# Increase Docker memory limit
# Docker Desktop -> Settings -> Resources -> Memory
```

### Debug Mode

Enable debug logging:
```bash
# Set in .env
DEBUG=crewchief:*
LOG_LEVEL=debug

# Or run with debug
DEBUG=crewchief:* ./scripts/docker-run.sh up
```

### Logs and Monitoring

**View logs:**
```bash
# All services
./scripts/docker-run.sh logs

# Specific service
./scripts/docker-run.sh logs web-ui

# Follow logs
./scripts/docker-run.sh logs -f web-ui

# Last 100 lines
./scripts/docker-run.sh logs --tail 100 web-ui
```

**Monitor resources:**
```bash
# Container stats
docker stats

# Service status
./scripts/docker-run.sh ps
```

## Development Workflow

### Hot Reload Development

1. Start development environment:
   ```bash
   ./scripts/docker-run.sh up
   ```

2. Edit source files (automatically reloaded):
   - Frontend: `packages/web-ui/src/client/`
   - Backend: `packages/web-ui/src/server.ts`

3. Run tests:
   ```bash
   ./scripts/docker-run.sh exec web-ui pnpm test
   ```

4. Database operations:
   ```bash
   # Run migrations
   ./scripts/docker-run.sh exec web-ui pnpm db:migrate
   
   # Seed data
   ./scripts/docker-run.sh exec web-ui pnpm db:seed
   ```

### Building for Production

1. Build production image:
   ```bash
   ./scripts/docker-build.sh -t production
   ```

2. Test production locally:
   ```bash
   ./scripts/docker-run.sh up -e production
   ```

3. Push to registry:
   ```bash
   ./scripts/docker-build.sh --push --tag-latest
   ```

## Deployment

### Local Production Testing

Test production setup locally:
```bash
# Build production image
./scripts/docker-build.sh -t production

# Start production environment
./scripts/docker-run.sh up -e production -d

# Check health
curl http://localhost:3456/api/health
```

### CI/CD Integration

Example GitHub Actions workflow:
```yaml
- name: Build Docker Image
  run: ./scripts/docker-build.sh --no-cache --push

- name: Test with Docker
  run: |
    ./scripts/docker-run.sh up -d
    # Run integration tests
    ./scripts/docker-run.sh down
```

## Maintenance

### Regular Maintenance

```bash
# Update images
./scripts/docker-run.sh pull

# Clean up unused resources
./scripts/docker-run.sh clean

# Rebuild images
./scripts/docker-run.sh build --no-cache
```

### Database Maintenance

```bash
# Backup database
./scripts/docker-run.sh exec postgres pg_dump -U postgres crewchief > backup-$(date +%Y%m%d).sql

# Vacuum database
./scripts/docker-run.sh exec postgres psql -U postgres -d crewchief -c "VACUUM ANALYZE;"

# Check database size
./scripts/docker-run.sh exec postgres psql -U postgres -d crewchief -c "SELECT pg_size_pretty(pg_database_size('crewchief'));"
```

This Docker setup provides a robust, scalable, and secure foundation for deploying the CrewChief Web UI in both development and production environments.