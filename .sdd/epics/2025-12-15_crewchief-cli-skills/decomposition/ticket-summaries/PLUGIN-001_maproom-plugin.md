# Ticket: Create Maproom Plugin

**Ticket ID:** PLUGIN-001
**Priority:** 1 (High)
**Effort:** M (2-3 days)

## Summary

Create the `maproom` plugin for the crewchief marketplace at `.crewchief/claude-code-plugins/plugins/maproom/`. The plugin provides semantic code search capabilities via the crewchief-maproom CLI, teaching Claude when and how to use maproom search instead of grep/glob.

## Deliverables

1. **Plugin Structure:**
   ```
   .crewchief/claude-code-plugins/plugins/maproom/
   ├── .claude-plugin/
   │   └── plugin.json
   ├── README.md
   └── skills/
       └── maproom-search/
           ├── SKILL.md
           └── references/
               └── search-best-practices.md
   ```

2. **plugin.json** with:
   - name: "maproom"
   - version: "0.1.0"
   - description for plugin discovery
   - author and repository info
   - keywords for discoverability

3. **README.md** with:
   - Plugin description
   - Installation instructions
   - Feature list
   - Usage examples
   - Prerequisites (CLI installed, database indexed)

4. **skills/maproom-search/SKILL.md** with:
   - YAML frontmatter (name: maproom-search, description for skill discovery)
   - Query formulation patterns
   - CLI command examples (search, status, context)
   - Decision tree: when maproom vs grep/glob
   - Error handling guidance

5. **skills/maproom-search/references/search-best-practices.md** with:
   - 10+ query transformation examples
   - Search strategy decision flowchart
   - Task-based search patterns
   - Anti-patterns to avoid

## Dependencies

- None (first ticket to execute)

## Value Proposition

Enables Claude to leverage semantic search for conceptual code exploration. When users ask "How does authentication work?" or "Find the error handling logic", Claude can use maproom search to find implementations by concept rather than relying on exact text matching with grep. Plugin architecture allows users to install only if they use maproom.

## Acceptance Criteria

- [ ] Plugin directory structure matches specification
- [ ] plugin.json has valid name, version, description
- [ ] README.md documents installation and usage
- [ ] SKILL.md follows Claude Code skill format
- [ ] SKILL.md frontmatter has valid name (lowercase, hyphens) and description (<1024 chars)
- [ ] SKILL.md description clearly states when to use this skill
- [ ] Query formulation patterns documented with examples
- [ ] CLI commands include: search, status, context
- [ ] Decision tree explains maproom vs grep/glob choice
- [ ] Error handling covers: no results, database not indexed
- [ ] search-best-practices.md has 10+ concrete examples

## Technical Notes

### plugin.json

```json
{
  "name": "maproom",
  "version": "0.1.0",
  "description": "Semantic code search using the crewchief-maproom CLI. Find code by concept, understand architecture, and explore relationships between code elements.",
  "author": {
    "name": "Daniel Bushman",
    "email": "dbushman@manifoldlogic.com",
    "url": "https://github.com/manifoldlogic/claude-code-plugins"
  },
  "repository": "https://github.com/manifoldlogic/claude-code-plugins",
  "keywords": [
    "maproom",
    "semantic-search",
    "code-search",
    "fts",
    "vector-search"
  ]
}
```

### SKILL.md Frontmatter

```yaml
---
name: maproom-search
description: This skill should be used for semantic code search when exploring unfamiliar codebases, finding implementations by concept (e.g., "authentication", "error handling"), or understanding code architecture. Uses the crewchief-maproom CLI for FTS and vector search. Prefer native Grep for exact text matches and Glob for file patterns.
---
```

### CLI Commands to Document

```bash
# Check index status
crewchief-maproom status --repo <repo>

# Search (FTS mode - always works)
crewchief-maproom search --query "authentication" --repo <repo> --mode fts

# Search (Hybrid mode - requires embeddings)
crewchief-maproom search --query "authentication" --repo <repo> --mode hybrid

# Get context for a chunk
crewchief-maproom context --chunk-id <id> --callers --callees --json
```

### Query Formulation Patterns

Transform user questions:
- "How does X work?" -> "X" (single concept)
- "What handles errors?" -> "error handler" (2 words)
- "Find the authentication logic" -> "authentication" (core concept)

### When to Use Maproom vs Grep

**Use Maproom:**
- Finding implementations by concept
- Understanding architecture
- Discovering related code
- Exploring unfamiliar codebases

**Use Grep:**
- Exact text matching (TODO, FIXME)
- Finding identifier usages
- Regex pattern matching

## Reference Documentation

- `/workspace/crates/maproom/CLAUDE.md` - CLI command reference
- `/workspace/packages/maproom-mcp/CLAUDE.md` - MCP tool docs (for query patterns)
- `/workspace/.crewchief/claude-code-plugins/plugins/github-actions/skills/gh-cli/SKILL.md` - Pattern example
