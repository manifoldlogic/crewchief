# Architecture: Maproom Plugin

## Overview

The maproom plugin is a Claude Code plugin that provides semantic code search capabilities through the crewchief-maproom CLI. It follows the established plugin architecture in the crewchief marketplace, containing a single skill (`maproom-search`) that teaches Claude when and how to use semantic search instead of relying on native tools like Grep for conceptual code queries.

```
.crewchief/claude-code-plugins/plugins/maproom/
├── .claude-plugin/
│   └── plugin.json           # Plugin metadata
├── README.md                 # User documentation
└── skills/
    └── maproom-search/
        ├── SKILL.md          # Skill instructions
        └── references/
            └── search-best-practices.md  # Detailed examples
```

## Design Decisions

### Decision 1: CLI-First with MCP Awareness

**Context:** Maproom functionality is available via both MCP protocol (maproom-mcp daemon) and CLI (crewchief-maproom binary). Skills need to choose an invocation method.

**Decision:** Document CLI invocation via Bash tool as the primary approach, with awareness that MCP exists for supported environments.

**Rationale:**
- CLI is universally available (works in any environment with the binary)
- CLI provides full search capabilities (`search`, `vector-search`, `status`, `context`)
- Simpler skill design (just bash commands, no daemon lifecycle)
- MCP provides performance benefits (20-50x faster) but adds complexity
- Users with MCP already installed will benefit from the same query formulation guidance
- The skill focuses on WHEN and HOW to search, which applies to both interfaces

**Clarification:**
- The skill documents CLI commands that Claude invokes via Bash
- MCP integration is separate (handled by MCP server if installed)
- Both interfaces benefit from the same query formulation patterns

### Decision 2: Single Skill Per Plugin

**Context:** Maproom provides multiple capabilities (search, indexing, context). Could bundle all as separate skills or consolidate.

**Decision:** Single `maproom-search` skill covering search, status, and context operations.

**Rationale:**
- These operations are interconnected in a typical workflow
- Status check precedes search; context follows search results
- Keeps skill discovery simple (one description to match)
- Avoids confusion about which skill handles which command
- Indexing (`scan`) is administrative, not Claude-driven

### Decision 3: Progressive Disclosure Structure

**Context:** Query formulation guidance is extensive (~50 examples could be provided). Need to balance discoverability vs context size.

**Decision:** Put essential patterns in SKILL.md, detailed examples in `references/search-best-practices.md`.

**Rationale:**
- SKILL.md body is loaded when skill activates (~2-3k tokens)
- References loaded only when Claude needs more examples
- Keeps initial context lean for frequent activation
- Follows Claude Code skill architecture best practice

### Decision 4: Leverage SearchMode Auto-Detection and Teach Command Choice

**Context:** Maproom has two layers of intelligence:
1. **SearchMode auto-detection** (Code/Text/Auto) - Query understanding layer that optimizes execution
2. **Execution backends** (FTS via `search`, vector via `vector-search`) - Actual search mechanisms

**Decision:** The skill teaches query formulation and command selection, NOT mode overrides.

**What the Skill Teaches:**
1. **Query Formulation (Primary):** How to ask good questions (2-3 words, concepts, not sentences)
2. **Command Selection:** When to use `search` vs `vector-search` based on:
   - Data availability (check `status` for embeddings)
   - Query type (identifier lookup vs conceptual understanding)
   - Performance needs (FTS is faster, vector is more semantic)
3. **SearchMode Awareness:** Understanding that the system auto-detects Code/Text/Auto to optimize results internally
4. **Tool Choice:** When to use maproom vs Grep vs Glob

**Rationale:**
- SearchMode auto-detection is intelligent and works well - no need to override
- FTS and vector serve different purposes, not just different "modes" of the same thing
- User's vision: help Claude leverage ALL capabilities, not bias toward one
- Query formulation is more important than mode selection
- Empowers informed choices, not blind defaults

**What the Skill Does NOT Do:**
- Default to one mode
- Add `--mode` flags to commands (doesn't exist in CLI)
- Override the intelligent query understanding system
- Treat this as "simple vs advanced" - both are valuable tools

### Decision 5: Repository Name from Working Directory

**Context:** Search requires `--repo` parameter. Claude needs to know how to determine this.

**Decision:** Document that repo name is typically the git repository root directory name. Include workflow to check available repos via status.

**Rationale:**
- Matches how users typically name repositories when indexing
- Status command without `--repo` shows all available repos
- Avoids hardcoding repository names in skill

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Invocation | CLI via Bash | Full parity with MCP, simpler lifecycle |
| Search Backend | crewchief-maproom | Existing optimized Rust implementation |
| Database | SQLite (~/.maproom/maproom.db) | Established maproom architecture |
| Plugin Format | Claude Code Plugin | Marketplace integration, versioning |
| Documentation | Markdown | Standard for skills, references |

## Component Design

### Plugin Metadata (plugin.json)

**Responsibilities:**
- Define plugin identity (name, version, description)
- Provide author and repository information
- Enable discovery via keywords

**Interface:**
```json
{
  "name": "maproom",
  "version": "0.1.0",
  "description": "Semantic code search using crewchief-maproom CLI...",
  "author": { "name": "...", "email": "...", "url": "..." },
  "repository": "...",
  "keywords": ["maproom", "semantic-search", ...]
}
```

### User Documentation (README.md)

**Responsibilities:**
- Describe plugin purpose and features
- Document prerequisites (CLI installed, database indexed)
- Provide installation instructions
- List usage examples

**Structure:**
1. Introduction
2. Features
3. Prerequisites
4. Installation
5. Usage Examples
6. Troubleshooting

### Search Skill (skills/maproom-search/SKILL.md)

**Responsibilities:**
- Define when skill should activate (frontmatter description)
- Provide query formulation guidance
- Document CLI command syntax
- Include decision tree (maproom vs grep/glob)
- Specify error handling workflows

**Structure:**
1. YAML Frontmatter (name, description)
2. Overview section
3. Decision tree (when to use)
4. Query formulation patterns
5. CLI command reference
6. Error handling
7. Reference to best-practices.md

### Reference Document (references/search-best-practices.md)

**Responsibilities:**
- Provide 10+ query transformation examples
- Document search strategy patterns
- Cover task-based search workflows
- List anti-patterns to avoid

**Structure:**
1. Query transformation examples table
2. Strategy patterns by task type
3. Common anti-patterns
4. Advanced techniques

## Data Flow

```
User Question (conceptual)
    |
    v
+-------------------+
| Claude Code       |
| 1. Parse query    |
| 2. Check plugins  |
| 3. Match skill    |
+-------------------+
    |
    | skill description matches
    v
+-------------------+
| maproom-search    |
| SKILL.md loaded   |
+-------------------+
    |
    v
+-------------------+
| Claude formulates |
| query (2-3 words) |
+-------------------+
    |
    v
+-------------------+
| Bash Tool         |
| crewchief-maproom |
| search --query    |
+-------------------+
    |
    v
+-------------------+
| ~/.maproom/       |
| maproom.db        |
+-------------------+
    |
    v
+-------------------+
| JSON results      |
| -> Claude         |
| -> User           |
+-------------------+
```

### Typical Workflow

1. **User asks conceptual question**: "How does authentication work in this codebase?"
2. **Claude activates maproom-search skill** (description matches)
3. **Claude checks status**: `crewchief-maproom status --repo <repo>` (discovers embeddings available)
4. **Claude formulates query**: "authentication" (extracts core concept, 2-3 words)
5. **Claude chooses command**:
   - If semantic understanding needed + embeddings available: `vector-search`
   - If identifier search or no embeddings: `search` (FTS)
6. **System auto-detects SearchMode**: "authentication" → Code mode (single word identifier)
7. **Claude reads results**: Uses chunk_ids for context expansion
8. **Claude assembles context**: `crewchief-maproom context --chunk-id <id> --callers --callees`
9. **Claude responds**: Explains authentication with code references

## Integration Points

### With crewchief-maproom CLI

The skill documents CLI commands as they actually exist (verified from source code):

```bash
# Status (check available repos and embedding status)
crewchief-maproom status --repo <repo>

# FTS search (keyword matching, always works)
crewchief-maproom search --query "<query>" --repo <repo> [--k N]

# Vector search (semantic similarity, requires embeddings)
crewchief-maproom vector-search --query "<query>" --repo <repo> [--k N] [--threshold 0.7]

# Context (expand from search results)
crewchief-maproom context --chunk-id <id> [--callers] [--callees] [--tests] [--json]
```

**Note:** The CLI uses separate commands (`search` vs `vector-search`) rather than a `--mode` flag. The daemon interface has a `mode` parameter, but the CLI does not.

### With Claude Code Plugin System

Plugin follows marketplace conventions:
- Location: `.crewchief/claude-code-plugins/plugins/maproom/`
- Registration: Via marketplace.json in marketplace root
- Installation: `/plugin install maproom@crewchief`

### With Native Tools

Skill includes decision tree clarifying when NOT to use maproom:
- Exact text: Use Grep
- File patterns: Use Glob
- Known identifiers: Use Grep

## Performance Considerations

### Query Latency

- FTS search: ~50-200ms (SQLite FTS5, local database)
- Hybrid search: ~200-500ms (includes vector similarity)
- Context assembly: ~100-300ms (graph traversal)

**Implication:** CLI invocation is fast enough for interactive use. No caching needed at skill level.

### Context Size

- SKILL.md: ~2-3k tokens (loaded on activation)
- search-best-practices.md: ~1-2k tokens (loaded on demand)

**Implication:** Total skill footprint is small. Progressive disclosure works well.

## Maintainability

### Documentation Linkage

The skill references `crates/maproom/CLAUDE.md` for authoritative CLI documentation. This provides:
- Single source of truth for command syntax
- Automatic updates when CLI changes
- Version-specific documentation

### Versioning Strategy

- Plugin version in plugin.json (semver)
- Version 0.1.0 for initial release
- Increment minor for new features, patch for fixes
- Major version for breaking changes

### Update Workflow

When CLI commands change:
1. Update `crates/maproom/CLAUDE.md` (source of truth)
2. Verify skill examples still work
3. Update skill if needed
4. Bump plugin version
