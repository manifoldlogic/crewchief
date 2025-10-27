# Maproom LOCAL - Quick Start

## TL;DR

```bash
cd /workspace/config
docker compose up -d
docker compose logs -f
```

Wait 3-6 minutes for first startup (Ollama downloads model), then all services will be healthy.

## Essential Commands

```bash
# Start services
docker compose up -d

# View logs
docker compose logs -f

# Check status
docker compose ps

# Stop services
docker compose down

# Reset everything (deletes data!)
docker compose down -v
```

## Using the Control Script

```bash
# Start
./maproom-ctl.sh start

# Check health
./maproom-ctl.sh health

# View logs
./maproom-ctl.sh logs

# Stop
./maproom-ctl.sh stop
```

## Service Ports

- **Maproom MCP**: http://localhost:3000
- **Ollama API**: http://localhost:11434
- **PostgreSQL**: Internal only (not exposed)

## First Startup Timeline

1. **PostgreSQL** (30s): Database initialization
2. **Ollama** (2-5 min): Downloads nomic-embed-text model (~300MB)
3. **Maproom** (30s): Starts after dependencies are healthy

**Total**: 3-6 minutes first time, 30-60s subsequent startups

## Customization

Create `.env` file:

```env
OLLAMA_PORT=11434
MAPROOM_PORT=3000
HOST_WORKSPACE=/path/to/your/code
RUST_LOG=info
```

## Troubleshooting

### Services won't start
```bash
docker compose logs
```

### Port conflicts
```bash
MAPROOM_PORT=3001 docker compose up -d
```

### Reset everything
```bash
docker compose down -v
docker compose up -d
```

## More Info

- Full documentation: [README.md](README.md)
- Control script help: `./maproom-ctl.sh help`
