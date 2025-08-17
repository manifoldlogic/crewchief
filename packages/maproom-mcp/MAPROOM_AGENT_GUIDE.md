# Maproom Search Guide for AI Agents

## Quick Start

**ALWAYS follow this sequence:**
1. Run `status` tool first to see what's indexed
2. Use `search` with simple terms (1-3 words)
3. Use `open` with exact paths from search results

## Search Strategy

### What Maproom Does Well
- **Semantic search**: Finds code by concept, not just text matching
- **Cross-file understanding**: Discovers related code across the codebase
- **Fast exploration**: Better than grep for understanding unfamiliar code

### Effective Search Queries

#### ✅ GOOD Queries (1-3 words, conceptual)
- `authentication`
- `error handling`
- `database connection`
- `message bus`
- `worktree create`
- `React hooks`

#### ❌ BAD Queries (too specific, too many terms)
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
| Wrong repo name | Use `status` to check, default is often `crewchief` |

## When Search Returns No Results

1. **Check status first**: Ensure files are indexed
2. **Simplify query**: Remove special characters, use fewer words
3. **Try variations**:
   - Lowercase: `UserAuth` → `userauth`
   - Split terms: `handleError` → `handle error`
   - Conceptual: `authenticate` → `auth` or `login`
4. **Check filters**: Try without filters first, then add `filter:"code"` or `filter:"docs"`

## Tool Usage Patterns

### Finding Code
```
1. status                          # See what's available
2. search repo:"crewchief" query:"authentication"  # Find relevant code
3. open relpath:"src/auth.ts" worktree:"main"     # View specific file
```

### Exploring Concepts
```
1. search repo:"crewchief" query:"message"         # Broad search
2. search repo:"crewchief" query:"message bus"     # Refine
3. search repo:"crewchief" query:"MessageBus" k:20 # Get more results
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
4. **Increase k for more results**: Default is 10, try k:20 or k:30
5. **Filter by type**: Use `filter:"code"` to exclude docs, or `filter:"docs"` for documentation only

## Index Management

The index is usually automatic, but if results seem stale:
- Run `status` to check last index time
- Use `upsert` only if files were recently changed and not indexed
- Most changes are auto-indexed within seconds

## Default Repository

For the CrewChief codebase, always use:
- `repo:"crewchief"`

## Remember

- **Semantic search > exact match**: Think concepts, not syntax
- **Simple > complex**: 2-3 words usually work best
- **Status first**: Always check what's indexed before searching
- **Learn from hints**: Read the hints when searches fail - they're tailored to your query