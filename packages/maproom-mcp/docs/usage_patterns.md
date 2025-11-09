# Maproom MCP Server - Usage Patterns

This document provides comprehensive usage patterns for the Maproom MCP server, covering beginner to advanced workflows for both Claude Desktop and VS Code clients.

## Table of Contents

- [Quick Start](#quick-start)
- [Tool Reference](#tool-reference)
- [Beginner Patterns](#beginner-patterns)
- [Intermediate Patterns](#intermediate-patterns)
- [Advanced Patterns](#advanced-patterns)
- [Client-Specific Tips](#client-specific-tips)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Prerequisites

1. **PostgreSQL Database**: Running and accessible
2. **Indexed Repository**: Use the maproom CLI to index your codebase
3. **MCP Server Built**: Run `pnpm build` in `packages/maproom-mcp`
4. **Client Configured**: Set up Claude Desktop or VS Code with the server

### First Steps

Always start with the `status` tool to verify what's indexed:

```
Tool: status
Parameters: {}
```

This shows you:
- Available repositories
- Indexed worktrees
- File and chunk counts
- Last indexing time

## Tool Reference

### 1. status

**Purpose**: Check maproom index status
**When to Use**: First tool to call; verify what's indexed
**Parameters**:
- `repo` (optional): Filter to specific repository

**Example**:
```json
{
  "repo": "crewchief"
}
```

**Returns**: Repository and worktree statistics

---

### 2. search

**Purpose**: Semantic code search across indexed repositories
**When to Use**: Finding code by concept or functionality
**Parameters**:
- `repo` (required): Repository name
- `query` (required): Search query (1-3 words work best)
- `worktree` (optional): Limit to specific worktree
- `filter` (optional): File type filter (`all`, `code`, `docs`, `config`)
- `k` (optional): Number of results (default: 10, max: 20)

**Example**:
```json
{
  "repo": "crewchief",
  "query": "authentication flow",
  "filter": "code",
  "k": 5
}
```

**Returns**: Array of chunks with:
- `chunk_id`: UUID for use in context/explain tools
- `relpath`: File path for use in open tool
- `worktree`: Worktree name for use in open tool
- `start_line`, `end_line`: Line range
- `score`: Relevance score
- `content`: Code snippet

**Tips**:
- Keep queries simple: 2-3 words work best
- Use conceptual terms: "authentication", "database connection"
- Add context words: "user login validation" not just "login"
- Filter by type for faster, more relevant results

---

### 3. open

**Purpose**: Retrieve specific code sections from indexed files
**When to Use**: After getting search results; view full file content
**Parameters**:
- `relpath` (required): Relative path from search results
- `worktree` (required): Worktree name from search results
- `range` (optional): Object with `start` and `end` line numbers
- `context` (optional): Number of context lines before/after (default: 0)

**Example**:
```json
{
  "relpath": "packages/cli/src/auth/login.ts",
  "worktree": "main",
  "range": {
    "start": 10,
    "end": 50
  },
  "context": 5
}
```

**Returns**: Code content with metadata

**Tips**:
- Copy `relpath` and `worktree` exactly from search results
- Use `range` to focus on specific functions
- Add `context` to see surrounding code
- Omit `range` to get entire file (if small)

---

### 4. context

**Purpose**: Retrieve contextually relevant code sections around a chunk
**When to Use**: Understanding code relationships and dependencies
**Parameters**:
- `chunk_id` (required): UUID from search results
- `budget_tokens` (optional): Max tokens in bundle (default: 6000, range: 1000-20000)
- `expand` (optional): Expansion configuration object
  - `callers` (boolean): Include chunks that call this function
  - `callees` (boolean): Include chunks called by this function
  - `tests` (boolean): Include test chunks
  - `docs` (boolean): Include documentation
  - `config` (boolean): Include related config files
  - `max_depth` (integer): Traversal depth (1-5, default: 2)

**Example**:
```json
{
  "chunk_id": "550e8400-e29b-41d4-a716-446655440000",
  "budget_tokens": 8000,
  "expand": {
    "callers": true,
    "callees": true,
    "tests": true,
    "max_depth": 2
  }
}
```

**Returns**: Context bundle with:
- Target chunk
- Related chunks (imports, callers, callees, tests)
- Total token count
- Relationship types

**Tips**:
- Start with default budget (6000 tokens)
- Increase budget for complex functions
- Enable `tests` to see how code is tested
- Use `max_depth: 1` for immediate dependencies only
- Use `max_depth: 3` for deep architectural understanding

---

### 5. upsert

**Purpose**: Update the code index for specific files
**When to Use**: After making code changes; keeping index current
**Parameters**:
- `paths` (required): Array of file paths to index
- `commit` (required): Git commit hash (use `"HEAD"` for current)
- `repo` (required): Repository name
- `worktree` (required): Worktree name
- `root` (required): Absolute path to repository root

**Example**:
```json
{
  "paths": [
    "packages/cli/src/auth/login.ts",
    "packages/cli/src/auth/validate.ts"
  ],
  "commit": "HEAD",
  "repo": "crewchief",
  "worktree": "main",
  "root": "/absolute/path/to/crewchief"
}
```

**Returns**: Indexing statistics

**Tips**:
- Index only changed files for speed
- Use `"HEAD"` for current working directory
- Provide absolute paths for reliability
- Re-index after significant refactoring

---

### 6. explain

**Purpose**: Generate detailed symbol card for a code chunk (EXPERIMENTAL)
**When to Use**: Understanding complex functions or classes
**Parameters**:
- `chunk_id` (required): Chunk ID from search results

**Example**:
```json
{
  "chunk_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Returns**: Markdown-formatted explanation with:
- Symbol metadata (name, type, language)
- Code relationships
- Code preview
- Usage examples
- Related symbols

**Tips**:
- Use after finding interesting chunks in search
- Combines well with context tool
- Cache-friendly for repeated queries
- May require additional configuration

---

## Beginner Patterns

### Pattern 1: Find and View Code

**Goal**: Find a specific feature and view its implementation

**Steps**:
1. Check what's indexed
   ```json
   Tool: status
   ```

2. Search for the feature
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "authentication"
   }
   ```

3. Open the most relevant result
   ```json
   Tool: open
   {
     "relpath": "src/auth/login.ts",
     "worktree": "main"
   }
   ```

**Use Cases**:
- Finding where a feature is implemented
- Locating a specific function
- Viewing configuration files
- Understanding file structure

---

### Pattern 2: Search with Filters

**Goal**: Find specific types of files

**Steps**:
1. Search code files only
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "database connection",
     "filter": "code"
   }
   ```

2. Search documentation
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "API reference",
     "filter": "docs"
   }
   ```

3. Search configuration
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "postgres settings",
     "filter": "config"
   }
   ```

**Use Cases**:
- Finding implementation vs. documentation
- Locating configuration files
- Filtering out test files
- Narrowing search scope

---

### Pattern 3: View Code with Context

**Goal**: See code with surrounding context

**Steps**:
1. Search for target code
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "error handler"
   }
   ```

2. Open with context lines
   ```json
   Tool: open
   {
     "relpath": "src/utils/errors.ts",
     "worktree": "main",
     "range": { "start": 20, "end": 40 },
     "context": 10
   }
   ```

**Use Cases**:
- Understanding function context
- Seeing imports and dependencies
- Viewing related functions
- Getting complete picture

---

## Intermediate Patterns

### Pattern 4: Explore Related Code

**Goal**: Understand how code relates to other parts

**Steps**:
1. Search for main function
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "process payment"
   }
   ```

2. Get context bundle (note the chunk_id from search results)
   ```json
   Tool: context
   {
     "chunk_id": "chunk-id-from-search",
     "budget_tokens": 8000,
     "expand": {
       "callers": true,
       "callees": true,
       "tests": true
     }
   }
   ```

3. Explore caller implementations
   ```json
   Tool: open
   {
     "relpath": "caller-file-from-context",
     "worktree": "main"
   }
   ```

**Use Cases**:
- Understanding call graphs
- Finding all callers of a function
- Seeing what a function calls
- Locating related tests

---

### Pattern 5: Multi-Branch Search

**Goal**: Search across different branches/worktrees

**Steps**:
1. Check available worktrees
   ```json
   Tool: status
   {
     "repo": "your-repo"
   }
   ```

2. Search main branch
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "authentication",
     "worktree": "main"
   }
   ```

3. Search feature branch
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "authentication",
     "worktree": "feature-oauth"
   }
   ```

**Use Cases**:
- Comparing implementations across branches
- Finding feature branch code
- Reviewing merged vs. unmerged code
- Multi-version analysis

---

### Pattern 6: Incremental Indexing

**Goal**: Keep index up-to-date during development

**Steps**:
1. Make code changes

2. Index changed files
   ```json
   Tool: upsert
   {
     "paths": [
       "src/auth/login.ts",
       "src/auth/session.ts"
     ],
     "commit": "HEAD",
     "repo": "your-repo",
     "worktree": "main",
     "root": "/absolute/path/to/repo"
   }
   ```

3. Verify changes are searchable
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "new function name"
   }
   ```

**Use Cases**:
- Maintaining search accuracy during development
- Testing new code discoverability
- Incremental updates without full re-index
- Quick iteration cycles

---

## Advanced Patterns

### Pattern 7: Deep Architecture Exploration

**Goal**: Understand system architecture and relationships

**Steps**:
1. Find entry points
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "main entry index",
     "filter": "code"
   }
   ```

2. Get deep context
   ```json
   Tool: context
   {
     "chunk_id": "entry-point-chunk-id",
     "budget_tokens": 15000,
     "expand": {
       "callers": false,
       "callees": true,
       "tests": false,
       "config": true,
       "max_depth": 3
     }
   }
   ```

3. Explain key components
   ```json
   Tool: explain
   {
     "chunk_id": "core-component-chunk-id"
   }
   ```

4. Map out architecture by searching key patterns
   ```json
   Tool: search
   { "repo": "your-repo", "query": "router handler" }
   Tool: search
   { "repo": "your-repo", "query": "service layer" }
   Tool: search
   { "repo": "your-repo", "query": "database model" }
   ```

**Use Cases**:
- Onboarding to new codebase
- Architectural documentation
- System design understanding
- Dependency mapping

---

### Pattern 8: Test Coverage Analysis

**Goal**: Find tests for specific functionality

**Steps**:
1. Search for implementation
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "user validation"
   }
   ```

2. Get context with tests enabled
   ```json
   Tool: context
   {
     "chunk_id": "validation-chunk-id",
     "expand": {
       "tests": true,
       "callers": false,
       "callees": false
     }
   }
   ```

3. Open test files
   ```json
   Tool: open
   {
     "relpath": "tests/auth/validation.test.ts",
     "worktree": "main"
   }
   ```

4. Search for additional test patterns
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "validation test describe"
   }
   ```

**Use Cases**:
- Finding existing tests before writing new ones
- Understanding test patterns
- Test coverage analysis
- Learning by example

---

### Pattern 9: Refactoring Support

**Goal**: Safely refactor code with full context

**Steps**:
1. Find all usages
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "legacy function name"
   }
   ```

2. Get caller context for each result
   ```json
   Tool: context
   {
     "chunk_id": "usage-chunk-id",
     "expand": {
       "callers": true,
       "max_depth": 2
     }
   }
   ```

3. Verify test coverage
   ```json
   Tool: context
   {
     "chunk_id": "target-chunk-id",
     "expand": {
       "tests": true
     }
   }
   ```

4. After refactoring, re-index
   ```json
   Tool: upsert
   {
     "paths": ["all/changed/files.ts"],
     "commit": "HEAD",
     "repo": "your-repo",
     "worktree": "main",
     "root": "/absolute/path"
   }
   ```

5. Verify refactoring
   ```json
   Tool: search
   {
     "repo": "your-repo",
     "query": "new function name"
   }
   ```

**Use Cases**:
- Safe renaming of functions
- Understanding impact of changes
- Finding all call sites
- Maintaining test coverage

---

### Pattern 10: Cross-Repository Learning

**Goal**: Learn patterns from multiple projects

**Steps**:
1. Index multiple repositories

2. Search pattern in repo A
   ```json
   Tool: search
   {
     "repo": "project-a",
     "query": "error handling pattern"
   }
   ```

3. Search same pattern in repo B
   ```json
   Tool: search
   {
     "repo": "project-b",
     "query": "error handling pattern"
   }
   ```

4. Compare implementations
   ```json
   Tool: open
   { "relpath": "src/errors.ts", "worktree": "main" }
   ```

5. Get detailed explanations
   ```json
   Tool: explain
   { "chunk_id": "chunk-from-each-repo" }
   ```

**Use Cases**:
- Learning best practices
- Standardizing patterns across projects
- Finding reusable components
- Technology comparison

---

## Client-Specific Tips

### Claude Desktop

**Strengths**:
- Natural language interaction
- Multi-step reasoning
- Conversation context
- Iterative exploration

**Best Practices**:
1. Describe your goal in natural language first
2. Let Claude decide which tools to use
3. Provide feedback on results
4. Ask follow-up questions
5. Request explanations of code

**Example Conversation**:
```
You: "I need to understand how authentication works in this codebase"

Claude: [Uses status tool to check indexed repos]
Claude: [Uses search tool with query="authentication"]
Claude: [Analyzes results and uses context tool]
Claude: [Explains the authentication flow based on context]

You: "Can you show me the login handler specifically?"

Claude: [Uses open tool to show relevant code]
Claude: [Explains the code]
```

**Tips**:
- Ask open-ended questions
- Request explanations of search results
- Combine code search with analysis
- Iterate based on findings

---

### VS Code

**Strengths**:
- Direct IDE integration
- Quick tool access
- Side-by-side viewing
- Workspace awareness

**Best Practices**:
1. Use Command Palette for quick access (Cmd/Ctrl+Shift+P)
2. Keep Output panel open for logs
3. Use workspace settings for project-specific config
4. Combine with VS Code's built-in search
5. Use MCP for semantic search, grep for exact matches

**Workflow**:
1. Open Command Palette
2. Type "MCP: Call Tool"
3. Select tool (e.g., "search")
4. Enter parameters in JSON format
5. View results in Output panel
6. Open files directly in editor

**Tips**:
- Create keyboard shortcuts for frequent tools
- Use workspace .vscode/settings.json for team sharing
- Monitor MCP logs for debugging
- Combine MCP search with VS Code navigation

---

## Best Practices

### Search Query Optimization

**Do**:
- Use 1-3 words
- Focus on concepts ("authentication flow")
- Include context ("user login validation")
- Use filter parameter to narrow scope
- Try variations if no results

**Don't**:
- Use full sentences
- Use exact code syntax
- Include special characters
- Use too many words
- Give up after first try

**Examples**:
- Good: "authentication flow"
- Good: "database connection pool"
- Good: "error handler middleware"
- Bad: "how does the authentication system work in this application"
- Bad: "function authenticateUser(req, res) {"

---

### Performance Optimization

1. **Limit search results**: Use `k` parameter
2. **Filter by file type**: Use `filter` parameter
3. **Control context budget**: Adjust `budget_tokens`
4. **Target specific worktrees**: Use `worktree` parameter
5. **Index incrementally**: Use `upsert` for changed files only

---

### Workflow Efficiency

1. **Always start with `status`** to verify indexing
2. **Use `search` before `open`** to find the right file
3. **Use `context` for relationships**, not just `open`
4. **Re-index after changes** to maintain accuracy
5. **Combine tools** for comprehensive understanding

---

### Team Collaboration

1. **Share configurations**: Commit .vscode/settings.json (without credentials)
2. **Document environment variables**: README with MAPROOM_DATABASE_URL setup
3. **Use shared database**: Team-wide PostgreSQL instance
4. **Index main branches**: Keep primary branches indexed
5. **Standardize queries**: Document common search patterns

---

## Troubleshooting

### No Search Results

**Possible Causes**:
1. Repository not indexed
2. Worktree parameter mismatch
3. Query too specific
4. Wrong filter applied

**Solutions**:
1. Run `status` tool to verify indexing
2. Check worktree names match indexed ones
3. Simplify query to 1-2 words
4. Try `filter: "all"` to broaden search
5. Re-index if code is new

---

### Server Connection Issues

**Possible Causes**:
1. MAPROOM_DATABASE_URL incorrect
2. PostgreSQL not running
3. Network connectivity
4. Server not started

**Solutions**:
1. Verify MAPROOM_DATABASE_URL format
2. Check PostgreSQL is running (`pg_isready`)
3. Test database connection (`psql`)
4. Check server logs (MAPROOM_MCP_LOG_FILE)
5. Restart MCP server
6. Verify firewall settings

---

### Slow Performance

**Possible Causes**:
1. Large result sets
2. Deep context expansion
3. Unoptimized queries
4. Database performance

**Solutions**:
1. Reduce `k` parameter in search
2. Lower `budget_tokens` in context
3. Reduce `max_depth` in context expansion
4. Use `filter` to narrow search scope
5. Index only necessary files
6. Check PostgreSQL performance
7. Add database indexes if needed

---

### Unexpected Results

**Possible Causes**:
1. Stale index
2. Wrong worktree
3. Misunderstood query semantics
4. Filter applied incorrectly

**Solutions**:
1. Re-index with `upsert` tool
2. Verify worktree parameter
3. Try different query terms
4. Check filter parameter
5. Use `status` to verify index state
6. Review search result scores

---

### Tool Errors

**Possible Causes**:
1. Missing required parameters
2. Invalid parameter values
3. Database errors
4. Server configuration issues

**Solutions**:
1. Check tool schema in documentation
2. Verify all required parameters are provided
3. Validate parameter types (string, number, object)
4. Check server logs for detailed errors
5. Test database connection separately
6. Review server configuration
7. Ensure server is built (`pnpm build`)

---

## Additional Resources

- **MCP Architecture**: See `/workspace/.agents/archive/projects/MCP_CORE_mcp-server-core/planning/MCP_CORE_ARCHITECTURE.md`
- **Example Configurations**: See `examples/` directory
- **Integration Tests**: See `tests/integration/` for working examples
- **Server Logs**: Check MAPROOM_MCP_LOG_FILE for debugging
- **PostgreSQL Docs**: https://www.postgresql.org/docs/

---

## Getting Help

If you encounter issues not covered here:

1. Check server logs for detailed error messages
2. Verify database connection and schema
3. Test each tool in isolation
4. Review example configurations
5. Check MCP protocol compatibility
6. Consult integration test examples
7. File an issue with:
   - MCP server version
   - Client version
   - Error messages from logs
   - Steps to reproduce
   - Expected vs. actual behavior
