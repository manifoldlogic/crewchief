# CrewChief DevContainer with Claude Code Support

This folder contains the development container configuration for CrewChief, providing a fully-featured, consistent development environment using VS Code Dev Containers with integrated Claude Code support in dangerous mode for safe AI-assisted development.

## 🚀 Quick Start

1. **Prerequisites**:
   - [Docker Desktop](https://www.docker.com/products/docker-desktop)
   - [Visual Studio Code](https://code.visualstudio.com/) OR [Cursor](https://cursor.sh/)
   - [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) (for VS Code)
   - For Cursor: Remote development support is built-in

2. **Open in DevContainer**:
   
   **VS Code:**
   - Open this repository in VS Code
   - Press `F1` and select "Dev Containers: Reopen in Container"
   - Or click the green button in the bottom-left corner and select "Reopen in Container"
   
   **Cursor:**
   - Open this repository in Cursor
   - Press `Cmd+Shift+P` (Mac) or `Ctrl+Shift+P` (Windows/Linux)
   - Select "Remote-Containers: Reopen in Container"
   - See [CURSOR_SETUP.md](./.devcontainer/CURSOR_SETUP.md) for Cursor-specific help

3. **Wait for Setup**:
   - The container will build and run post-create scripts
   - This includes installing dependencies, building binaries, and setting up databases

## 📦 What's Included

### Languages & Runtimes
- **Node.js 20** with pnpm
- **Rust** (latest stable) with cargo
- **TypeScript** support
- **Python 3** (for scripting)

### Databases & Services
- **PostgreSQL 15** - Main database
- **Redis 7** - Caching and sessions
- **pgAdmin** - Database management UI (port 5050)
- **Redis Commander** - Redis UI (port 8081)

### Development Tools
- **Claude Code** - AI assistant with dangerous mode enabled (network-restricted)
- **tmux** - Terminal multiplexer with custom config
- **Git** - Latest version with extensions
- **Docker-in-Docker** - Run Docker commands inside the container
- **GitHub CLI** - GitHub integration
- **ripgrep, fd, bat, exa** - Modern CLI tools
- **iptables/ipset** - Network control for Claude Code dangerous mode

### VS Code Extensions
- ESLint & Prettier
- TypeScript & Rust language support
- TailwindCSS IntelliSense
- Vitest & Playwright test runners
- Docker & GitHub Actions support
- GitLens & Git Graph
- Markdown support with Mermaid

## 🛠️ Configuration

### Environment Variables
The container sets up these environment variables automatically:
```bash
NODE_ENV=development
DATABASE_URL=postgresql://postgres:postgres@postgres:5432/crewchief
REDIS_URL=redis://redis:6379
CREWCHIEF_MAPROOM_BIN=/usr/local/bin/crewchief-maproom
CLAUDE_CONFIG_DIR=/home/vscode/.claude
CLAUDE_DANGEROUS_MODE=true
ANTHROPIC_API_KEY=<set in host environment>
```

### Port Forwarding
These ports are automatically forwarded:
- `3000` - Frontend development server
- `3456` - Web UI default port
- `3500` - Backend API server
- `5432` - PostgreSQL (internal)
- `6379` - Redis (internal)
- `5050` - pgAdmin web interface
- `8081` - Redis Commander

### Shell Aliases
Useful aliases are configured:
```bash
claude     # Run Claude Code in dangerous mode
webui      # Start Web UI dev server
ccdev      # Run CrewChief CLI in dev mode
maproom    # Run Maproom commands
ta         # tmux attach
tn         # tmux new session
```

## 📁 File Structure

```
.devcontainer/
├── devcontainer.json    # Main configuration
├── docker-compose.yml   # Services definition
├── Dockerfile           # Container image
├── tmux.conf           # tmux configuration
├── pgadmin-servers.json # pgAdmin pre-configured servers
└── scripts/
    ├── post-create.sh   # Runs after container creation
    ├── post-start.sh    # Runs when container starts
    ├── post-attach.sh   # Runs when you attach to container
    ├── init-db.sql      # Database initialization
    └── init-claude-firewall.sh # Claude Code network restrictions
```

## 🤖 Claude Code Dangerous Mode

This devcontainer includes Claude Code with dangerous mode enabled, allowing it to execute commands and modify files directly. To ensure safety:

### Network Configuration
The container uses iptables firewall rules for security:
- ✅ **Internet Access**: Full internet access enabled
- ❌ **Host Access**: Blocked for security (except DNS)
- ✅ **Container Network**: Full access to services (PostgreSQL, Redis)
- ✅ **Domain Approval**: Claude Code's built-in approval system handles domain access

### Usage
1. **Set your API key** in host environment:
   ```bash
   export ANTHROPIC_API_KEY="your-api-key"
   ```

2. **Start Claude Code**:
   ```bash
   claude
   ```

3. **Verify firewall** (run in container):
   ```bash
   sudo /usr/local/bin/init-claude-firewall.sh
   ```

### Security Features
- Network isolation via iptables/ipset
- Restricted to allowed domains only
- Runs in containerized environment
- All changes isolated to workspace
- Persistent configuration in volume

### Dangerous Mode Capabilities
When running in dangerous mode, Claude Code can:
- Execute shell commands
- Create and modify files
- Install packages
- Run tests and builds
- Access local services

## 🔧 Common Tasks

### Running the Web UI
```bash
cd packages/web-ui
pnpm dev
# or use the alias:
webui
```

### Running the CLI
```bash
# Development mode with hot reload
tsx packages/cli/src/cli/index.ts --help

# Or use the alias
ccdev --help
```

### Database Operations
```bash
# Access PostgreSQL
psql -h postgres -U postgres -d crewchief

# Run migrations
cd packages/web-ui
pnpm run db:migrate

# Access pgAdmin
# Open http://localhost:5050
# Login: admin@crewchief.local / admin
```

### Running Tests
```bash
# All tests
pnpm test

# Unit tests only
pnpm test:unit

# E2E tests
pnpm test:e2e

# With coverage
pnpm test:coverage
```

### Building Maproom
```bash
cd crates/maproom
cargo build --release
sudo cp target/release/crewchief-maproom /usr/local/bin/
```

## 🐛 Troubleshooting

### Container Won't Start
- Ensure Docker Desktop is running
- Check available disk space
- Try: `docker system prune` to clean up

### Database Connection Issues
- PostgreSQL takes a few seconds to start
- Check logs: `docker logs <container-id>`
- Verify connection: `pg_isready -h postgres -p 5432`

### Port Conflicts
- If ports are already in use, modify `.devcontainer/devcontainer.json`
- Change the `forwardPorts` section to use different ports

### Slow Performance on macOS
- Increase Docker Desktop memory allocation
- Use cached mounts (already configured)
- Consider using native development for better performance

### Extension Issues
- Reload VS Code window: `Ctrl+Shift+P` → "Developer: Reload Window"
- Rebuild container: `Ctrl+Shift+P` → "Dev Containers: Rebuild Container"

## 🔄 Updating the DevContainer

To update the DevContainer configuration:

1. Make changes to files in `.devcontainer/`
2. Rebuild the container:
   - Press `F1` → "Dev Containers: Rebuild Container"
   - Or `Ctrl+Shift+P` → "Dev Containers: Rebuild Container"

## 📝 Notes

- Git credentials are automatically forwarded from your host
- SSH keys are mounted from your host's `~/.ssh` directory
- The container uses volume mounts for cargo and pnpm caches for faster rebuilds
- tmux sessions persist across container restarts
- Your workspace is mounted at `/workspace`

## 🔗 Resources

- [Dev Containers Documentation](https://code.visualstudio.com/docs/remote/containers)
- [devcontainer.json Reference](https://containers.dev/implementors/json_reference/)
- [Docker Compose in Dev Containers](https://code.visualstudio.com/docs/remote/create-dev-container#_use-docker-compose)