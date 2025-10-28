# Ticket: LOCAL-5004: Document Dual PostgreSQL Architecture

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive documentation explaining the dual PostgreSQL architecture, including when to use each instance, data migration paths, and architectural diagrams.

## Background
The CrewChief project has two PostgreSQL instances with different purposes, but this architecture is not documented anywhere:

1. **Devcontainer PostgreSQL** (`postgres:5432`)
   - Purpose: CrewChief app data + Maproom development/testing data
   - Credentials: `postgres:postgres`
   - Database: `crewchief`
   - Usage: Local development, integration tests, CLI development

2. **Maproom PostgreSQL** (`maproom-postgres:5432`)
   - Purpose: Standalone MCP service data (production-like)
   - Credentials: `maproom:maproom`
   - Database: `maproom`
   - Usage: MCP server, isolated indexing, production deployments

**Current Problems**:
- No documentation of why two databases exist
- Confusion about which database to index into
- Unclear data migration path between instances
- Onboarding difficulty for new developers
- Risk of indexing to wrong database and losing work

**Impact**:
- Developers waste time figuring out database architecture
- Risk of data inconsistency between instances
- Difficulty troubleshooting database connection issues
- Poor onboarding experience for contributors

## Acceptance Criteria
- [ ] Create `docs/architecture/DATABASE_ARCHITECTURE.md` with comprehensive dual-database explanation
- [ ] Update `packages/maproom-mcp/README.md` with database architecture section
- [ ] Update `CLAUDE.md` with database architecture notes for AI assistant context
- [ ] Include architectural diagram showing both PostgreSQL instances and their relationships
- [ ] Document when to use each PostgreSQL instance
- [ ] Provide data migration guide for moving data between instances
- [ ] Explain connection string formats for both instances
- [ ] Document schema differences (if any) between the two instances

## Technical Requirements
- Create new documentation file with clear sections
- Include ASCII diagram or Mermaid diagram showing architecture
- Document connection strings and credentials for both instances
- Provide concrete examples of when to use each database
- Include migration scripts or commands (if applicable)
- Explain Docker network configuration that enables dual databases

## Implementation Notes

### Documentation Structure

**1. DATABASE_ARCHITECTURE.md** (New file: `docs/architecture/DATABASE_ARCHITECTURE.md`)
   - Overview of dual-database architecture
   - Architectural diagram (Mermaid or ASCII)
   - Detailed explanation of each instance
   - Connection string reference
   - When to use which database
   - Data migration procedures
   - Troubleshooting common issues

**2. Maproom MCP README Update** (`packages/maproom-mcp/README.md`)
   - Add "Database Architecture" section
   - Explain which PostgreSQL instance MCP uses
   - Link to full architecture documentation
   - Include connection string examples

**3. CLAUDE.md Update**
   - Add database architecture notes under "Architecture Overview"
   - Help AI assistants understand dual-database context
   - Include quick reference for connection strings

### Key Content to Document

**Devcontainer PostgreSQL**:
- Purpose: Local development, CrewChief CLI, integration testing
- When to use: Running `cargo run`, developing Maproom features, CLI testing
- Connection: `postgresql://postgres:postgres@postgres:5432/crewchief`
- Data lifetime: Ephemeral (can be recreated from migrations)

**Maproom PostgreSQL**:
- Purpose: MCP server, production-like isolated instance
- When to use: Testing MCP tools, production deployments, npx usage
- Connection: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- Data lifetime: Persistent (production data)

**Migration Scenarios**:
1. Exporting data from devcontainer postgres to maproom postgres
2. Syncing schema changes between instances
3. Backing up and restoring each instance independently

**Architectural Diagram** (Example structure):
```
┌─────────────────────────────────────────────────────────┐
│                   Docker Network(s)                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐              ┌──────────────┐       │
│  │ Devcontainer │              │   Maproom    │       │
│  │  PostgreSQL  │              │  PostgreSQL  │       │
│  │              │              │              │       │
│  │ postgres:5432│              │maproom-postgres:     │
│  │              │              │     5432     │       │
│  │ User: postgres              │ User: maproom│       │
│  │ DB: crewchief│              │ DB: maproom  │       │
│  └──────┬───────┘              └──────┬───────┘       │
│         │                             │               │
│         │                             │               │
│  ┌──────▼───────┐              ┌──────▼───────┐      │
│  │ CrewChief CLI│              │ Maproom MCP  │      │
│  │ Development  │              │   Server     │      │
│  │              │              │              │      │
│  │ cargo run    │              │ npm start    │      │
│  │ Integration  │              │ MCP Tools    │      │
│  │   Tests      │              │              │      │
│  └──────────────┘              └──────────────┘      │
│                                                        │
└────────────────────────────────────────────────────────┘
```

## Dependencies
- None - this is documentation work

## Risk Assessment
- **Risk**: Documentation becomes outdated as architecture evolves
  - **Mitigation**: Add reminders in relevant code files to update docs when changing database config
- **Risk**: Diagram becomes too complex or unclear
  - **Mitigation**: Keep diagram simple, focus on key connections, use clear labels
- **Risk**: Migration instructions may not cover all edge cases
  - **Mitigation**: Document common scenarios, add "Additional Help" section for complex migrations

## Files/Packages Affected
- `docs/architecture/DATABASE_ARCHITECTURE.md` - **NEW FILE** - Comprehensive database architecture documentation
- `packages/maproom-mcp/README.md` - Add database architecture section
- `CLAUDE.md` - Update with database architecture notes
- `packages/maproom-mcp/config/docker-compose.yml` - Possibly add inline comments explaining database choice
