# Active Projects

This directory contains all currently active projects with their planning documentation and work tickets.

## Active Projects

### DKRHUB_docker-hub-publishing - Docker Hub Publishing
**Status:** 🔄 In Progress (26 tickets)
**Goal:** Publish CrewChief Maproom images to Docker Hub for public distribution

**Quick Links:**
- [Project Overview](./DKRHUB_docker-hub-publishing/README.md)
- [Planning Docs](./DKRHUB_docker-hub-publishing/planning/)
- [Active Tickets](./DKRHUB_docker-hub-publishing/tickets/)

---

### LOCAL_local-deployment - Local Deployment
**Status:** 🔄 In Progress (21 tickets)
**Goal:** Enable fully local deployment with Docker Compose and local embedding models

**Quick Links:**
- [Project Overview](./LOCAL_local-deployment/README.md)
- [Planning Docs](./LOCAL_local-deployment/planning/)
- [Active Tickets](./LOCAL_local-deployment/tickets/)
- [Archived Tickets](./LOCAL_local-deployment/archive/tickets/) (27 completed)

---

### MCPSTART_mcp-provider-startup-fix - MCP Provider Startup Fix
**Status:** 🔄 In Progress (23 tickets)
**Goal:** Fix MCP provider initialization and startup reliability issues

**Quick Links:**
- [Project Overview](./MCPSTART_mcp-provider-startup-fix/README.md)
- [Planning Docs](./MCPSTART_mcp-provider-startup-fix/planning/)
- [Active Tickets](./MCPSTART_mcp-provider-startup-fix/tickets/)

---

### MCP_mcp-core-features - MCP Core Features
**Status:** 🔄 In Progress (11 tickets)
**Goal:** Core MCP server functionality and features

**Quick Links:**
- [Active Tickets](./MCP_mcp-core-features/tickets/)
- [Archived Tickets](./MCP_mcp-core-features/archive/tickets/) (4 completed)

---

### BINPKG_binary-packaging - Binary Packaging
**Status:** 🔄 In Progress (22 tickets)
**Goal:** Build and publish cross-platform Rust binaries for @crewchief/maproom-mcp to npm

**Quick Links:**
- [Project Overview](./BINPKG_binary-packaging/README.md)
- [Planning Docs](./BINPKG_binary-packaging/planning/)
- [Active Tickets](./BINPKG_binary-packaging/tickets/)

---

### DBFALLBK_database-fallback - Database Connection Fallback
**Status:** 🔄 In Progress (7 tickets)
**Goal:** Implement robust database connection fallback logic

**Quick Links:**
- [Project Overview](./DBFALLBK_database-fallback/README.md)
- [Active Tickets](./DBFALLBK_database-fallback/tickets/)

---

### DOCKER_docker-perl-openssl - Docker Perl OpenSSL Fix
**Status:** 🔄 In Progress (1 ticket)
**Goal:** Add Perl to Docker image for vendored OpenSSL compilation

**Quick Links:**
- [Project Overview](./DOCKER_docker-perl-openssl/README.md)
- [Active Tickets](./DOCKER_docker-perl-openssl/tickets/)

---

### MAPROOM_MIGRATIONS_migration-fixes - Migration Runner Fixes
**Status:** 🔄 In Progress (2 tickets)
**Goal:** Fix migration runner transaction handling and concurrent index creation

**Quick Links:**
- [Project Overview](./MAPROOM_MIGRATIONS_migration-fixes/README.md)
- [Active Tickets](./MAPROOM_MIGRATIONS_migration-fixes/tickets/)

---

## Project Structure

Each project follows this structure:

```
{SLUG}_{descriptive-name}/
├── README.md                  # Project overview and status
├── planning/                  # Strategic planning documents
│   ├── analysis.md           # Problem analysis
│   ├── architecture.md       # Technical design
│   ├── plan.md               # Implementation phases
│   ├── quality-strategy.md   # Testing approach
│   └── security-review.md    # Security considerations
├── tickets/                   # Active work tickets
│   └── {SLUG}-NNN_description.md
└── archive/                   # Completed tickets (optional)
    └── tickets/
```

**Naming Convention:** `{SLUG}_{descriptive-name}`
- SLUG: UPPERCASE project slug (matches ticket prefix)
- descriptive-name: lowercase-with-dashes, clear project purpose

## Working with Projects

### Finding Work
```bash
# List all active projects
ls -1 .agents/projects/

# See all tickets for a project
ls -1 .agents/projects/DKRHUB_docker-hub-publishing/tickets/

# Count remaining tickets
ls -1 .agents/projects/DKRHUB_docker-hub-publishing/tickets/ | wc -l
```

### Reading Project Context
```bash
# Get project overview
cat .agents/projects/DKRHUB_docker-hub-publishing/README.md

# Read planning docs
cat .agents/projects/DKRHUB_docker-hub-publishing/planning/architecture.md
cat .agents/projects/DKRHUB_docker-hub-publishing/planning/plan.md
```

### Working on Tickets

1. **Pick a ticket** from the project's `tickets/` directory
2. **Read context** from planning docs if needed
3. **Complete work** following ticket acceptance criteria
4. **Mark checkboxes** in the ticket as you progress:
   - `[x] Task completed` - when implementation done
   - `[x] Tests pass` - when tests succeed
   - `[x] Verified` - after verify-ticket agent checks

### When Project is Complete

When all tickets are done and knowledge is synthesized to `/docs`:

```bash
# Move project to archive
mv .agents/projects/{SLUG}_{descriptive-name} .agents/archive/projects/
```

Update this README to remove the archived project.

## Statistics

| Project | Active Tickets | Archived Tickets | Total |
|---------|----------------|------------------|-------|
| DKRHUB_docker-hub-publishing | 26 | 0 | 26 |
| LOCAL_local-deployment | 21 | 27 | 48 |
| MCPSTART_mcp-provider-startup-fix | 23 | 0 | 23 |
| MCP_mcp-core-features | 11 | 4 | 15 |
| BINPKG_binary-packaging | 22 | 0 | 22 |
| DBFALLBK_database-fallback | 7 | 0 | 7 |
| DOCKER_docker-perl-openssl | 1 | 0 | 1 |
| MAPROOM_MIGRATIONS_migration-fixes | 2 | 0 | 2 |
| **Total** | **113** | **31** | **144** |

---

For completed projects, see [Archive](../archive/README.md).
