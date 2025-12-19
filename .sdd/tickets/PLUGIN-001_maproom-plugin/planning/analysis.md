# Analysis: Maproom Plugin

## Problem Definition

Claude Code provides built-in tools (Read, Grep, Glob, Bash) for code exploration, but these tools are optimized for exact text matching and file pattern discovery. When users ask conceptual questions like "How does authentication work?" or "Find the error handling logic", Claude must rely on keyword guessing with Grep, which often produces suboptimal results.

The crewchief-maproom CLI provides semantic code search via FTS (full-text search) and optional vector embeddings, but Claude has no way to discover when or how to use this capability. The maproom plugin bridges this gap by teaching Claude when semantic search is appropriate and how to formulate effective queries.

## Context

### Current Situation

1. **Maproom CLI exists and is fully functional** - Located at `crates/maproom/`, the Rust CLI provides:
   - FTS search with BM25 ranking
   - Vector search via sqlite-vec (when embeddings are generated)
   - Hybrid search combining FTS and vector with Reciprocal Rank Fusion
   - Context assembly (callers, callees, tests, imports)
   - Index status checking

2. **MCP server exists but is overkill for plugins** - The `packages/maproom-mcp` server wraps the CLI via a daemon, but plugins can invoke CLI directly via Bash, avoiding daemon lifecycle complexity.

3. **Plugin architecture is established** - The crewchief marketplace at `.crewchief/claude-code-plugins/` contains existing plugins (workstream, github-actions, sdd, claude-code-dev) that provide proven patterns.

4. **No existing maproom plugin** - The capability exists but is not discoverable by Claude through the plugin/skill mechanism.

### Why This Work is Needed

- Users with indexed codebases cannot leverage semantic search because Claude defaults to Grep
- Conceptual queries produce poor results with exact text matching
- Users must manually invoke maproom commands, defeating the purpose of AI assistance
- The decision of when to use maproom vs grep is nuanced and should be automated

## Existing Solutions

### Industry Approaches

1. **GitHub Copilot** - Uses vector embeddings for semantic code search within VS Code, but limited to IDE context
2. **Sourcegraph** - Enterprise code search with natural language queries, but requires separate infrastructure
3. **Cursor** - AI-native IDE with semantic search, but tightly coupled to their editor

### Codebase Approaches

1. **MCP Protocol (maproom-mcp)** - Provides semantic search via Model Context Protocol, but:
   - Requires daemon management
   - Overkill for simple CLI invocation
   - Full CLI parity exists

2. **Native Grep Tool** - Claude's built-in search, optimal for:
   - Exact text matching (TODO, FIXME, error messages)
   - Identifier usage search
   - Regex pattern matching

3. **Native Glob Tool** - Claude's file pattern matching, optimal for:
   - Finding files by name pattern
   - Locating specific file types

## Current State

### Maproom CLI Commands (from actual CLI source code)

```bash
# Check index status (discover repos, check embeddings)
crewchief-maproom status --repo <repo>

# FTS search (always works, no embeddings needed)
crewchief-maproom search --query "authentication" --repo <repo>

# Vector search (requires embeddings, semantic similarity)
crewchief-maproom vector-search --query "authentication" --repo <repo>

# Context assembly
crewchief-maproom context --chunk-id <id> --callers --callees --json
```

### Search Capabilities

| Command | Search Type | Best For | Requirements |
|---------|-------------|----------|--------------|
| search | FTS (BM25) | Keyword matches, identifiers, exact text | Database indexed |
| vector-search | Vector similarity | Conceptual understanding, semantic queries | Embeddings generated |

**Note:** The CLI uses separate commands for different search types. The daemon interface also supports a `mode` parameter ("fts"/"vector"/"hybrid"), but the TypeScript daemon client does not expose this. For plugin purposes, we document the CLI commands which Claude can invoke via Bash.

### Query Formulation (from MCP tool descriptions)

Transform user questions to effective queries:
- "How does X work?" -> "X"
- "What handles errors?" -> "error handler"
- "Find the authentication logic" -> "authentication"

Best practices:
- 2-3 words work best
- Use concepts, not sentences
- Avoid special characters
- Try variations if results are sparse

## Research Findings

### Finding 1: CLI Has Full Parity with MCP

All MCP tools (search, open, context, status) have CLI counterparts. The CLI additionally provides database management commands (migrate, cleanup-stale, clean-ignored) that MCP does not expose. This means the plugin can use CLI directly without MCP complexity.

### Finding 2: Query Formulation is Critical

The MCP search tool description contains extensive query formulation guidance that should be included in the skill. Transformation patterns and 2-3 word query best practices are essential for effective search.

### Finding 3: Tool Selection Decision Tree

Clear patterns exist for when to use each tool:

**Use Maproom:**
- Finding implementations by concept
- Understanding architecture
- Discovering related code
- Exploring unfamiliar codebases

**Use Grep:**
- Exact text matching
- Finding identifier usages
- Regex pattern matching

### Finding 4: SearchMode Auto-Detection and Query Understanding

Maproom has intelligent query understanding through SearchMode auto-detection (Code/Text/Auto). This is distinct from execution mode (FTS vs vector):

**SearchMode Detection (Query Understanding):**
- **Code mode:** Detects code patterns (::, ->, =>, function calls), short identifiers (1-2 words), camelCase/snake_case
- **Text mode:** Natural language queries with 4+ words
- **Auto mode:** Ambiguous queries (2-3 words without clear code patterns)

**Execution Backend (Search Mechanism):**
- **FTS (search command):** Always works, no embeddings needed, keyword matching with BM25 ranking
- **Vector (vector-search command):** Requires embeddings, semantic similarity via cosine distance
- **Hybrid (daemon only):** Combines FTS + vector via Reciprocal Rank Fusion

**Key Insight:** SearchMode auto-detection optimizes how the system executes searches internally. The skill should teach query formulation (2-3 words, concepts) and help Claude choose between `search` and `vector-search` commands based on:
- Data availability (embeddings exist?)
- Query type (exact identifier vs conceptual understanding)
- NOT by defaulting to one mode

**Examples of SearchMode Detection:**
```
"User::authenticate"           → Code mode (:: operator)
"authenticate_user"            → Code mode (snake_case identifier)
"authentication"               → Code mode (single word)
"user authentication"          → Auto mode (2 words, no code patterns)
"how to authenticate a user"   → Text mode (natural language, 5 words)
```

The system automatically detects query intent and optimizes execution. The skill should leverage this intelligence, not override it.

### Finding 5: Context Assembly Enables Deep Understanding

The context command can retrieve callers, callees, tests, and imports for any chunk, enabling comprehensive code understanding beyond simple search results.

## Constraints

### Technical Constraints

1. **Database must be pre-indexed** - User must run `crewchief-maproom scan` before search works
2. **CLI must be installed** - `crewchief-maproom` binary must be available in PATH
3. **Repository context required** - Search requires `--repo` parameter

### Business Constraints

1. **Single skill per plugin** - Keep plugin focused on search capability only
2. **CLI-based invocation** - Use Bash tool, not MCP protocol

### Resource Constraints

1. **Description length limit** - SKILL.md frontmatter description must be <1024 chars
2. **Progressive disclosure** - SKILL.md body should be <5k words; put details in references

## Success Criteria

### Functional Criteria

- [ ] Plugin directory structure matches `.crewchief/claude-code-plugins/plugins/maproom/`
- [ ] plugin.json contains valid name, version, description, author, repository, keywords
- [ ] README.md documents installation, features, prerequisites
- [ ] SKILL.md has valid frontmatter (name: maproom-search, description <1024 chars)
- [ ] SKILL.md documents query formulation patterns with examples
- [ ] SKILL.md documents CLI commands (search, status, context)
- [ ] SKILL.md includes decision tree for maproom vs grep/glob
- [ ] SKILL.md includes error handling (no results, database not indexed)
- [ ] search-best-practices.md contains 10+ query transformation examples

### Quality Criteria

- [ ] Description clearly states when skill should be triggered
- [ ] Instructions use imperative form (verb-first)
- [ ] CLI command examples are copy-paste ready
- [ ] Error handling guidance is actionable

### Integration Criteria

- [ ] Plugin can be installed via `/plugin install maproom@crewchief`
- [ ] Skill activates for conceptual code queries
- [ ] Claude successfully invokes maproom CLI via Bash tool
