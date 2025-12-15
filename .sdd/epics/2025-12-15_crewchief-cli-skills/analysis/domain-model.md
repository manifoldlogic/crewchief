# Domain Model: CrewChief CLI Plugins

## Core Entities

### Claude Code Plugin

A modular package in the crewchief marketplace that extends Claude's capabilities. Structure:

```
plugin-name/
├── .claude-plugin/
│   └── plugin.json           # Required: Plugin metadata
├── README.md                 # Required: Plugin documentation
├── agents/                   # Optional: Agent definitions
├── commands/                 # Optional: Slash commands
├── skills/                   # Optional: Skills (our focus)
│   └── skill-name/
│       ├── SKILL.md          # Required for skills
│       ├── scripts/          # Optional: Executable scripts
│       ├── references/       # Optional: Reference docs
│       └── assets/           # Optional: Output files
└── hooks/                    # Optional: Event handlers
```

### plugin.json

Required metadata file defining plugin identity:

```json
{
  "name": "plugin-name",
  "version": "1.0.0",
  "description": "What the plugin does",
  "author": {
    "name": "Author Name",
    "email": "email@example.com",
    "url": "https://github.com/author"
  },
  "repository": "https://github.com/org/repo",
  "keywords": ["keyword1", "keyword2"]
}
```

### Marketplace (marketplace.json)

Registry of available plugins at `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`:

```json
{
  "name": "crewchief",
  "owner": { "name": "...", "email": "..." },
  "plugins": [
    {
      "name": "plugin-name",
      "source": "./plugins/plugin-name",
      "description": "Brief description"
    }
  ]
}
```

### Claude Code Skill

A capability package within a plugin. Consists of:
- **SKILL.md** - Required file with YAML frontmatter (name, description) and markdown instructions
- **scripts/** - Optional executable code for deterministic tasks
- **references/** - Optional documentation loaded on-demand
- **assets/** - Optional files used in output (templates, etc.)

Skills are model-invoked: Claude autonomously decides when to use them based on the description.

### Maproom Index

The semantic search database containing:
- **Repositories** - Named git repositories being tracked
- **Worktrees** - Database records tracking indexed branches (1:1 with git branches, not git worktrees)
- **Files** - Indexed source files
- **Chunks** - Searchable code segments with FTS and optional vector embeddings
- **Edges** - Relationships between chunks (caller/callee, imports, etc.)

Location: `~/.maproom/maproom.db` (SQLite)

### Search Query

User intent expressed as search terms:
- **Query** - 1-4 word conceptual search (e.g., "error handling", "authentication")
- **Mode** - fts (keyword), vector (semantic), or hybrid
- **Filters** - Constraints on results (file_type, repo, worktree)

### Search Result

Matched code chunk:
- **chunk_id** - Unique identifier for context expansion
- **relpath** - Relative file path
- **symbol_name** - Function/class name
- **kind** - Code element type (function, class, heading, etc.)
- **start_line/end_line** - Line range
- **score** - Relevance ranking

### Context Bundle

Related code assembled around a focal chunk:
- **Primary chunk** - The focal code
- **Callers** - Functions that call this code
- **Callees** - Functions called by this code
- **Tests** - Related test code
- **Imports** - Import relationships

Budget-constrained by token count.

### Git Worktree

A filesystem checkout of a git repository at a specific branch:
- **path** - Absolute filesystem path
- **branch** - Git branch name
- **source_branch** - Branch this worktree was created from (for merge tracking)

### Worktree Lifecycle

States a worktree passes through:
1. **Created** - `worktree create <name>` - New branch and checkout
2. **Active** - `worktree use <name>` - Switch to working in this worktree
3. **Merged** - `worktree merge <name>` - Changes merged back to source
4. **Cleaned** - `worktree clean <name>` - Worktree and branch removed

## Entity Relationships

```
Marketplace (crewchief)
    │
    └── contains many ──> Plugins
                              │
                              ├── plugin.json (1:1)
                              ├── README.md (1:1)
                              └── contains many ──> Skills
                                                       │
                                                       ├── SKILL.md (1:1)
                                                       ├── scripts/ (0:n)
                                                       └── references/ (0:n)

Maproom Plugin
    │
    └── maproom-search skill ──> uses ──> crewchief-maproom CLI
                                              │
                                              └── queries ──> Maproom Index
                                                                  │
                                                                  └── contains ──> Chunks

Worktree Plugin
    │
    └── worktree-management skill ──> uses ──> crewchief worktree CLI
                                                   │
                                                   └── manages ──> Git Worktrees
```

## Boundaries

### Maproom Plugin Boundary

**Inside:**
- plugin.json, README.md
- maproom-search skill with SKILL.md
- search-best-practices reference document
- Semantic search, indexing, status, context assembly documentation

**Outside:**
- Embedding provider configuration
- Daemon management
- MCP protocol details

### Worktree Plugin Boundary

**Inside:**
- plugin.json, README.md
- worktree-management skill with SKILL.md
- Create, list, use, clean, merge, copy-ignored operations

**Outside:**
- Agent spawning
- Competition features
- Automatic branch naming

### Plugin vs Native Tool Boundary

**Maproom search (plugin) better for:**
- Conceptual queries, finding implementations, understanding architecture

**Grep/Glob (native) better for:**
- Exact text matches, file name patterns, TODO/FIXME markers

## Interactions

```
User Query
    │
    v
+-------------------+
│ Claude Code       │
│ - Evaluates query │
│ - Checks plugins  │
+-------------------+
    │
    +---> [maproom plugin installed + conceptual?]
    │         │
    │         v
    │     maproom-search skill
    │         │
    │         v
    │     crewchief-maproom CLI
    │         │
    │         v
    │     ~/.maproom/maproom.db
    │
    +---> [Exact text?] ---> Native Grep/Glob tools
    │
    +---> [worktree plugin installed + worktree request?]
              │
              v
          worktree-management skill
              │
              v
          crewchief worktree CLI
              │
              v
          Git repository
```

### MCP vs CLI Capability Comparison

| Capability | MCP (maproom-mcp) | CLI (crewchief-maproom) | Gap? |
|------------|-------------------|------------------------|------|
| Search (FTS) | Yes | Yes | No |
| Search (Vector) | Yes | Yes | No |
| Search (Hybrid) | Yes | Yes | No |
| Status | Yes | Yes | No |
| Open file | Yes | Yes (via Read tool) | No |
| Context assembly | Yes | Yes | No |
| Scan/Index | Via daemon | Yes | No |
| Upsert | Via daemon | Yes | No |
| Generate embeddings | Via daemon | Yes | No |
| DB migrate | No | Yes | N/A |
| Cleanup stale | Via daemon | Yes | No |
| Clean ignored | No | Yes | No |

**Conclusion:** Full CLI parity exists. The CLI can do everything MCP does plus additional database management operations. Plugins should use CLI directly.
