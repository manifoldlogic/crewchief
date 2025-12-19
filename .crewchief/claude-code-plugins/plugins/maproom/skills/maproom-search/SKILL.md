---
name: maproom-search
description: This skill should be used for semantic code search when exploring unfamiliar codebases, finding implementations by concept (e.g., "authentication", "error handling"), or understanding code architecture. Uses the crewchief-maproom CLI for FTS and vector search. Prefer native Grep for exact text matches and Glob for file patterns.
---

# Maproom Search Skill

## Overview

Maproom provides semantic code search capabilities for exploring unfamiliar codebases and finding implementations by concept rather than exact text matches. It uses SQLite-based full-text search (FTS) and optional vector embeddings for semantic similarity.

## Decision Tree

### Use maproom when:
- Searching for functionality by concept ("authentication", "error handling")
- Exploring unfamiliar codebases to understand architecture
- Finding implementations without knowing exact names
- Discovering related code across multiple files
- Understanding code relationships (callers, callees, tests)

### Use Grep when:
- Searching for exact text matches or known identifiers
- Finding literal strings (TODO, FIXME, specific error messages)
- Pattern matching with regex across files
- You know the exact text you're looking for

### Use Glob when:
- Finding files by extension or name pattern
- Locating files matching wildcard patterns (*.ts, **/*.test.js)
- File discovery rather than content search

## Query Formulation Patterns

Transform natural language questions into effective search queries by extracting 2-3 core technical terms:

**Examples:**
- "How does authentication work?" → `authentication`
- "Find the database connection logic" → `database connection`
- "Where is error handling?" → `error handling`
- "What handles WebSocket disconnect?" → `WebSocket disconnect`
- "Show me the user registration flow" → `user registration`

**Best Practices:**
- Use 2-3 keywords maximum
- Remove question words (how, what, where, when, why)
- Prefer code-like terminology over natural language
- Try variations if initial query returns few results

## Command Selection Guidance

### Check Repository Status First
Always verify the repository is indexed before searching:

```bash
crewchief-maproom status --repo <repo>
```

Use `status` without `--repo` to list all indexed repositories.

### Use `search` for Full-Text Search
FTS search works on all indexed repositories (no embeddings required):

```bash
crewchief-maproom search --repo <repo> --query "<query>" [--k N]
```

This is the default and most reliable search method. Use for:
- Initial exploration of unfamiliar code
- Fast keyword-based searches
- When embeddings are not available

### Use `vector-search` for Semantic Similarity
Vector search requires pre-computed embeddings but provides semantic matching:

```bash
crewchief-maproom vector-search --repo <repo> --query "<query>" [--k N] [--threshold 0.7]
```

Use for:
- Finding conceptually similar code
- When exact keywords may vary
- After confirming embeddings are available

## SearchMode Awareness

Maproom automatically detects the appropriate search mode based on query structure:

- **Code mode**: Detects single words or code patterns (`authentication`, `UserAuth::login()`)
- **Text mode**: Detects natural language queries
- **Auto mode**: Handles mixed queries intelligently

The system optimizes search automatically - no manual mode override needed. Simply formulate your query naturally or use code terms, and maproom will adapt.

## CLI Command Reference

### Status and Discovery
```bash
# List all indexed repositories
crewchief-maproom status

# Check specific repository status
crewchief-maproom status --repo <repo>
```

### Full-Text Search
```bash
# Basic FTS search (returns top 10 results)
crewchief-maproom search --repo <repo> --query "<query>"

# Limit number of results
crewchief-maproom search --repo <repo> --query "<query>" --k 20
```

### Vector Search
```bash
# Semantic search (returns top 10 results)
crewchief-maproom vector-search --repo <repo> --query "<query>"

# Adjust similarity threshold and result count
crewchief-maproom vector-search --repo <repo> --query "<query>" --k 15 --threshold 0.7
```

### Context Expansion
```bash
# Get full context for a specific chunk (use chunk_id from search results)
crewchief-maproom context --chunk-id <id>

# Include specific relationships
crewchief-maproom context --chunk-id <id> --callers --callees --tests

# Output as JSON for programmatic use
crewchief-maproom context --chunk-id <id> --json
```

### Indexing
```bash
# Scan and index a repository
crewchief-maproom scan --repo <repo> --path /path/to/repo

# Re-index an existing repository
crewchief-maproom scan --repo <repo> --path /path/to/repo
```

## Error Handling

### No Results Found
- Try a broader query with fewer keywords
- Verify the repository is indexed using `status`
- Consider using different terms (e.g., "auth" vs "authentication")
- Check if the code exists in the indexed files

### Database Not Indexed
```
Error: Repository not found
```
Solution: Index the repository first:
```bash
crewchief-maproom scan --repo <repo> --path /path/to/repo
```

### Embeddings Missing
```
Error: Vector search unavailable
```
Solution: Use full-text search instead:
```bash
crewchief-maproom search --repo <repo> --query "<query>"
```
Vector search requires pre-computed embeddings, but FTS works on all indexed repositories.

### Repository Not Found
```
Error: Repository '<repo>' not found
```
Solution: List available repositories:
```bash
crewchief-maproom status
```
Use the exact repository name shown in the status output.

### Chunk ID Not Found
```
Error: Chunk not found
```
Solution: Verify the chunk ID from search results is correct and the repository hasn't been re-indexed (which generates new chunk IDs).

## Reference

For comprehensive search strategies and advanced patterns, see:
- [Search Best Practices](./references/search-best-practices.md)

For CLI implementation details and command-line options:
- `crewchief-maproom --help`
- `crewchief-maproom search --help`
- `crewchief-maproom vector-search --help`
