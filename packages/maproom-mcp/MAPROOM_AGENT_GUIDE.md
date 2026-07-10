# Maproom Search Guide for AI Agents

## Quick Start

**ALWAYS follow this sequence:**
1. Run `status` tool first to see what's indexed
2. Use `search` with simple terms (1-3 words)
3. Use `open` with exact paths from search results

## Repository Scope (required)

Every `search` call requires exactly one of three scope forms:

| Form | When to use |
|------|-------------|
| `repo: "name"` | Focused query — one repo. Always prefer this for targeted searches. |
| `repos: ["a", "b"]` | Cross-repo query in a single call (requires maproom >= 0.3.0). Results grouped by repo, up to `limit` hits each. |
| `allRepos: true` | Search every indexed repo (requires maproom >= 0.3.0). Use with caution — may sweep 170k+ chunks. Always prefer `repo` or `repos` for targeted queries. |

Omitting all three forms is an error.

## Search Strategy

### What Maproom Does Well
- **Semantic search**: Finds code by concept, not just text matching
- **Cross-file understanding**: Discovers related code across the codebase
- **Cross-repo search**: Single-call multi-repo queries (no N-process overhead) via maproom >= 0.3.0
- **Fast exploration**: Better than grep for understanding unfamiliar code

### Effective Search Queries

#### Good Queries (1-3 words, conceptual)
- `authentication`
- `error handling`
- `database connection`
- `message bus`
- `worktree create`
- `React hooks`

#### Bad Queries (too specific, too many terms)
- `function handleAuthenticationErrorInUserLogin`
- `const getUserByIdFromDatabase async function`
- `import React from 'react'`
- `authentication_handler_service_implementation`

### Common Mistakes & Solutions

| Mistake | Solution |
|---------|----------|
| Using underscores | Replace with spaces: `user_auth` → `user auth` |
| Too many terms | Use 2-3 key words: `handle user authentication error` → `auth error` |
| Exact code syntax | Search concepts: `async function getData` → `getData` |
| Wrong repo name | Use `status` to check available repos |
| Forgetting scope | Every search needs exactly one of: repo, repos, allRepos |

## When Search Returns No Results

1. **Check status first**: Ensure files are indexed
2. **Simplify query**: Remove special characters, use fewer words
3. **Try variations**:
   - Lowercase: `UserAuth` → `userauth`
   - Split terms: `handleError` → `handle error`
   - Conceptual: `authenticate` → `auth` or `login`
4. **Check filters**: Try without filters first, then add `filter:"code"` or `filter:"docs"`

## Tool Usage Patterns

### Finding Code (single repo)
```
1. status                          # See what's available
2. search repo:"crewchief" query:"authentication"  # Find relevant code
3. open relpath:"src/auth.ts" worktree:"main"     # View specific file
```

### Cross-Repo Search (single call, maproom >= 0.3.0)
```
# Search two repos in one call — results grouped by repo
search repos:["crewchief","specs"] query:"authentication" limit:5

# Each hit carries a repo field so you always know the source
# Use single-repo calls when you know where to look — they're faster
```

### All-Repos Sweep (use sparingly)
```
# Only when you genuinely need to search everywhere
search allRepos:true query:"authentication" limit:3
# limit is per-repo cap; prefer repo/repos for targeted queries
```

### Exploring Concepts
```
1. search repo:"crewchief" query:"message"         # Broad search
2. search repo:"crewchief" query:"message bus"     # Refine
3. search repo:"crewchief" query:"MessageBus" limit:20 # Get more results
```

### Understanding Architecture
```
1. search repo:"crewchief" query:"main entry"      # Find entry points
2. search repo:"crewchief" query:"config" filter:"config"  # Find configuration
3. search repo:"crewchief" query:"test" filter:"code"      # Find tests
```

## Pro Tips

1. **Start broad, then narrow**: `auth` → `auth login` → `login user`
2. **Use status liberally**: It's fast and shows what's searchable
3. **Copy paths exactly**: When using `open`, copy relpath and worktree from search results
4. **Increase limit for more results**: Default is 20, try limit:30 for more
5. **Filter by type**: Use `filter:"code"` to exclude docs, or `filter:"docs"` for documentation only
6. **Prefer repo over repos/allRepos**: Single-repo calls are faster; use multi-repo only when cross-repo comparison is genuinely needed

## Index Management

The index is managed by the `maproom` CLI (not the MCP server). If results seem stale:
- Run `status` to check last index time
- Use `maproom scan` CLI command to re-index if files were recently changed
- Most changes auto-indexed within seconds via pm2 file watchers

Note: `scan`, `upsert`, and `explain` are CLI-only operations; they are not available as MCP tools.

## Remember

- **Semantic search > exact match**: Think concepts, not syntax
- **Simple > complex**: 2-3 words usually work best
- **Status first**: Always check what's indexed before searching
- **Scope required**: Provide repo, repos, or allRepos — omitting all is an error
- **Single-repo preferred**: Use repo:"name" unless cross-repo comparison is needed
- **Learn from hints**: Read the hints when searches fail - they're tailored to your query