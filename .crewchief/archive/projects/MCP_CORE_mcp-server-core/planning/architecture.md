# MCP_CORE Architecture: MCP Server Implementation

## Architecture Overview

```
MCP Client → JSON-RPC → Tool Router → Tool Handlers → Database/Files
                              ↓
                     Validation & Error Handling
```

## Core Components

### 1. MCP Server Base
```typescript
class MaproomMCPServer {
  private tools: Map<string, ToolHandler>;
  private db: DatabasePool;

  async handleRequest(request: MCPRequest): Promise<MCPResponse> {
    try {
      const tool = this.tools.get(request.tool);
      if (!tool) throw new Error(`Unknown tool: ${request.tool}`);

      const validated = await tool.validate(request.params);
      const result = await tool.execute(validated);

      return { success: true, result };
    } catch (error) {
      return { success: false, error: error.message };
    }
  }
}
```

### 2. Tool Implementations

#### Search Tool (Enhanced)
```typescript
class SearchTool {
  async execute(params: SearchParams): Promise<SearchResult> {
    const pipeline = new SearchPipeline(this.db);
    const results = await pipeline.search(
      params.query,
      {
        k: params.k || 10,
        scope: params.scope,
        mode: params.mode || 'auto',
        filter: params.filter
      }
    );

    return {
      hits: results.map(r => ({
        score: r.score,
        chunk_id: r.chunk_id,
        relpath: r.relpath,
        symbol_name: r.symbol_name,
        preview: r.preview,
        start_line: r.start_line,
        end_line: r.end_line
      }))
    };
  }
}
```

#### Context Tool
```typescript
class ContextTool {
  async execute(params: ContextParams): Promise<ContextBundle> {
    const assembler = new ContextAssembler(this.db);
    return await assembler.assemble(
      params.chunk_id,
      params.budget_tokens || 6000,
      params.expand || {}
    );
  }
}
```

#### Open Tool
```typescript
class OpenTool {
  async execute(params: OpenParams): Promise<FileContent> {
    const { relpath, range, worktree, commit } = params;

    // Check if commit is checked out
    const isCheckedOut = await this.isCommitCheckedOut(commit);

    let content: string;
    if (isCheckedOut) {
      content = await fs.readFile(path.join(worktree, relpath), 'utf-8');
    } else {
      content = await this.getFileFromGit(commit, relpath);
    }

    if (range) {
      content = this.extractRange(content, range.start, range.end);
    }

    return { content, relpath, range };
  }
}
```

#### Upsert Tool
```typescript
class UpsertTool {
  async execute(params: UpsertParams): Promise<UpsertResult> {
    const { paths, commit, worktree } = params;

    // Spawn indexer process
    const result = await spawn('crewchief-maproom', [
      'upsert',
      '--paths', paths.join(','),
      '--commit', commit,
      '--worktree', worktree
    ]);

    return {
      updated_files: result.files_count,
      updated_chunks: result.chunks_count,
      duration_ms: result.duration
    };
  }
}
```

#### Explain Tool
```typescript
class ExplainTool {
  async execute(params: ExplainParams): Promise<SymbolCard> {
    const chunk = await this.db.getChunk(params.chunk_id);

    // Check cache
    const cached = await this.cache.get(`explain:${params.chunk_id}`);
    if (cached) return cached;

    // Generate explanation
    const card = await this.generateCard(chunk);

    // Cache result
    await this.cache.set(`explain:${params.chunk_id}`, card);

    return card;
  }
}
```

### 3. Validation Layer
```typescript
const SearchSchema = z.object({
  query: z.string().min(1),
  k: z.number().optional().default(10),
  scope: z.object({
    repo: z.string().optional(),
    worktree: z.string().optional(),
    paths: z.array(z.string()).optional(),
    languages: z.array(z.string()).optional()
  }).optional(),
  mode: z.enum(['auto', 'code', 'text']).optional(),
  filter: z.enum(['all', 'code', 'docs', 'config']).optional()
});
```

### 4. Error Handling
```typescript
class ErrorHandler {
  handle(error: Error): MCPError {
    if (error instanceof ValidationError) {
      return {
        code: 'INVALID_PARAMS',
        message: error.message,
        details: error.errors
      };
    }

    if (error instanceof DatabaseError) {
      return {
        code: 'DATABASE_ERROR',
        message: 'Database operation failed',
        details: process.env.DEBUG ? error.stack : undefined
      };
    }

    return {
      code: 'INTERNAL_ERROR',
      message: 'An unexpected error occurred'
    };
  }
}
```

## Configuration

```yaml
mcp:
  server:
    port: 3333
    host: localhost
    timeout_ms: 5000

  tools:
    search:
      enabled: true
      max_results: 50
    context:
      enabled: true
      max_budget: 10000
    open:
      enabled: true
      max_file_size: 1048576
    upsert:
      enabled: true
      max_paths: 100
    explain:
      enabled: false  # Experimental

  database:
    pool_size: 10
    connection_timeout: 5000
```

## Performance Optimizations

- Connection pooling for database
- Response streaming for large results
- Caching for expensive operations
- Parallel tool execution where possible
- Request debouncing

## Security Considerations

- Input validation on all parameters
- Path traversal prevention
- Rate limiting per client
- Authentication token support
- Audit logging