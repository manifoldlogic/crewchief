# Epic: CrewChief CLI Plugins

Created: 2025-12-15

## Vision Statement

Create two Claude Code plugins in the crewchief marketplace: (1) `maproom` plugin for semantic code search with CLI parity to maproom-mcp, and (2) `worktree` plugin for git worktree management. Each plugin can be independently enabled/disabled per project.

## Conceptual Frame

Claude Code provides built-in tools (Read, Grep, Glob, Bash) that work well for many code exploration tasks. However, users working with CrewChief have access to more powerful capabilities via CLI tools that are currently undiscoverable and undocumented for AI agents. This epic bridges that gap by creating Claude Code **plugins** in the crewchief marketplace that teach Claude when and how to use:

1. **Maproom CLI** - Semantic code search that excels at finding concepts, relationships, and understanding codebase architecture
2. **CrewChief Worktree CLI** - Git worktree management for parallel development workflows

The plugin architecture allows each capability to be:
- **Independent**: Users enable only what they need
- **Discoverable**: Plugins integrate with Claude's skill discovery mechanism
- **Versioned**: Each plugin has its own version and update cycle
- **Extensible**: Plugins can include skills, agents, commands, and hooks

## Domain Coherence

**Core Domain Concepts:**

- **Claude Code Plugin** - A modular package containing skills, agents, commands, and/or hooks that extends Claude's capabilities
- **Plugin Marketplace** - A collection of plugins available for installation (crewchief marketplace at `.crewchief/claude-code-plugins/`)
- **plugin.json** - Required metadata file defining plugin name, version, description, and components
- **SKILL.md** - Skill definition file with YAML frontmatter (name, description) and markdown instructions
- **Semantic Search** - Finding code by concept/intent rather than exact text matches
- **Git Worktrees** - Parallel filesystem checkouts sharing a single Git repository

## Directional Clarity

**Desired End State:**
"When this epic succeeds, users can install the `maproom` and `worktree` plugins from the crewchief marketplace. Claude will automatically leverage semantic code search and worktree management through plugin-provided skills that activate when relevant, providing better code exploration and parallel development capabilities than native tools alone."

**Success Signals:**
- [ ] Two separate plugins created (`maproom`, `worktree`) in `.crewchief/claude-code-plugins/plugins/`
- [ ] Plugins registered in marketplace.json
- [ ] Each plugin has valid plugin.json with proper metadata
- [ ] Skills include clear guidance on when to use maproom search vs grep/glob
- [ ] Skills document all CLI commands with examples
- [ ] Skills follow Claude Code SKILL.md format with proper frontmatter

## Scope Boundaries

**In Scope:**
- `maproom` plugin containing skill for: semantic search, indexing, status checking, context assembly
- `worktree` plugin containing skill for: creating, listing, switching (use), cleaning, merging worktrees
- Documentation of when to use maproom search vs grep/glob
- Query formulation best practices with concrete examples
- Reference materials for CLI command syntax
- Plugin registration in crewchief marketplace

**Out of Scope:**
- Agent spawning (`crewchief agent spawn`)
- Competitions (`crewchief competition`)
- Search optimizations (`crewchief optimization`)
- MCP server details (plugins use CLI, not MCP)
- Performance tuning or embedding provider configuration
- VSCode extension integration
- Creating agents or commands (skills only for initial version)

## Plugin Architecture

### Marketplace Location
```
.crewchief/claude-code-plugins/
├── .claude-plugin/
│   └── marketplace.json        # Register new plugins here
└── plugins/
    ├── maproom/                 # NEW: Maproom search plugin
    │   ├── .claude-plugin/
    │   │   └── plugin.json
    │   ├── README.md
    │   └── skills/
    │       └── maproom-search/
    │           ├── SKILL.md
    │           └── references/
    │               └── search-best-practices.md
    └── worktree/                # NEW: Worktree management plugin
        ├── .claude-plugin/
        │   └── plugin.json
        ├── README.md
        └── skills/
            └── worktree-management/
                └── SKILL.md
```

### Plugin Installation
```bash
# Users install plugins via Claude Code
/plugin install maproom@crewchief
/plugin install worktree@crewchief
```

## Derived Tickets

1. **PLUGIN-001: Create Maproom Plugin** - Create plugin with semantic search skill
2. **PLUGIN-002: Create Worktree Plugin** - Create plugin with worktree management skill
3. **PLUGIN-003: Register Plugins in Marketplace** - Update marketplace.json and test installation

## Status

- [x] Research complete
- [x] Analysis complete
- [x] Decomposition complete
- [ ] Tickets created

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| CLI commands may change | Plugins become outdated | Link skills to CLI CLAUDE.md for version-specific docs |
| Overlap with native tools causes confusion | Users unsure when to use which | Include clear decision tree in search best practices |
| Maproom database not indexed | Search returns no results | Include status check workflow before searching |
| Plugin structure changes | Plugins need updates | Follow existing plugin patterns exactly |
