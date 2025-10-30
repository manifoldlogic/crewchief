# CONTEXT_ASM Architecture: Context Assembly Engine

## System Design

### Core Flow
```
Chunk ID + Budget → Graph Traversal → Priority Queue → Token Counter → Assembly
                           ↓                ↓              ↓
                     Relationships    Importance      Truncation
```

## Components

### 1. Context Assembler
```typescript
interface ContextAssembler {
  assemble(
    chunkId: number,
    budget: number,
    options: ExpandOptions
  ): Promise<ContextBundle>;
}

interface ExpandOptions {
  callers: boolean;
  callees: boolean;
  tests: boolean;
  docs: boolean;
  config: boolean;
  maxDepth: number;
}
```

### 2. Graph Walker
```sql
-- Recursive CTE for relationship traversal
WITH RECURSIVE related AS (
  -- Start with target chunk
  SELECT id, 0 as depth, 1.0 as relevance
  FROM maproom.chunks WHERE id = $1

  UNION ALL

  -- Follow edges up to max depth
  SELECT
    CASE
      WHEN e.src_chunk_id = r.id THEN e.dst_chunk_id
      ELSE e.src_chunk_id
    END as id,
    r.depth + 1,
    r.relevance * 0.7  -- Decay factor
  FROM related r
  JOIN maproom.chunk_edges e ON (
    e.src_chunk_id = r.id OR e.dst_chunk_id = r.id
  )
  WHERE r.depth < $2
)
SELECT DISTINCT c.*, r.relevance
FROM related r
JOIN maproom.chunks c ON c.id = r.id
ORDER BY r.relevance DESC;
```

### 3. Priority Ranker
```typescript
class PriorityRanker {
  rank(chunk: Chunk, relationship: Relationship): number {
    let score = 1.0;

    // Relationship type weights
    if (relationship.type === 'test_of') score *= 1.5;
    if (relationship.type === 'calls') score *= 1.2;
    if (relationship.type === 'imports') score *= 1.1;

    // Distance decay
    score *= Math.pow(0.7, relationship.distance);

    // Importance signals
    score *= chunk.importance_score || 1.0;

    // Same directory bonus
    if (this.sameDirectory(chunk, target)) score *= 1.3;

    return score;
  }
}
```

### 4. Token Budget Manager
```typescript
class TokenBudgetManager {
  private used: number = 0;
  private reserved: Map<string, number> = new Map();

  reserve(category: string, tokens: number): boolean {
    if (this.used + tokens > this.budget) return false;
    this.reserved.set(category, tokens);
    this.used += tokens;
    return true;
  }

  allocate(): BudgetAllocation {
    return {
      primary: this.budget * 0.4,    // 40% for main chunk
      tests: this.budget * 0.2,      // 20% for tests
      callers: this.budget * 0.15,   // 15% for callers
      callees: this.budget * 0.15,   // 15% for callees
      config: this.budget * 0.1,     // 10% for config
    };
  }
}
```

### 5. Assembly Strategies

#### Default Strategy
```typescript
class DefaultAssemblyStrategy {
  async assemble(target: Chunk, budget: number): Promise<ContextItem[]> {
    const items: ContextItem[] = [];

    // 1. Primary chunk (full or truncated)
    items.push(await this.getPrimary(target, budget * 0.4));

    // 2. Direct test
    const test = await this.findTest(target);
    if (test) items.push(await this.formatChunk(test, 'test'));

    // 3. One caller, one callee
    const caller = await this.findTopCaller(target);
    if (caller) items.push(await this.formatChunk(caller, 'caller'));

    const callee = await this.findTopCallee(target);
    if (callee) items.push(await this.formatChunk(callee, 'callee'));

    // 4. Config if relevant
    if (this.needsConfig(target)) {
      const config = await this.findConfig(target);
      if (config) items.push(await this.formatChunk(config, 'config'));
    }

    return items;
  }
}
```

#### React Strategy
```typescript
class ReactAssemblyStrategy extends DefaultAssemblyStrategy {
  async assemble(target: Chunk, budget: number): Promise<ContextItem[]> {
    const items = await super.assemble(target, budget);

    // Add React-specific context
    if (this.isComponent(target)) {
      const route = await this.findRoute(target);
      if (route) items.push(await this.formatChunk(route, 'route'));

      const hooks = await this.findUsedHooks(target);
      hooks.forEach(hook => items.push(
        await this.formatChunk(hook, 'hook')
      ));
    }

    return items;
  }
}
```

### 6. Content Formatter
```typescript
class ContentFormatter {
  format(chunk: Chunk, role: string): ContextItem {
    const content = this.getContent(chunk);

    return {
      relpath: chunk.file.relpath,
      range: {
        start: chunk.start_line,
        end: chunk.end_line
      },
      role: role,
      reason: this.getReason(chunk, role),
      content: this.truncateIfNeeded(content),
      tokens: this.countTokens(content)
    };
  }

  private truncateIfNeeded(content: string): string {
    // Keep signature and docstring
    // Truncate body if too large
    // Add truncation marker
  }
}
```

## Database Schema Extensions

```sql
-- Cache for expensive traversals
CREATE TABLE maproom.context_cache (
  chunk_id BIGINT REFERENCES maproom.chunks(id),
  options_hash TEXT,
  bundle JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  PRIMARY KEY (chunk_id, options_hash)
);

-- Precomputed test links
CREATE INDEX idx_test_links ON maproom.test_links(target_chunk_id);
```

## Configuration

```yaml
context:
  default_budget: 6000
  strategies:
    default:
      primary_ratio: 0.4
      test_ratio: 0.2
      max_neighbors: 3
    react:
      include_routes: true
      include_hooks: true
      component_patterns: ["*.tsx", "components/**"]
  cache:
    enabled: true
    ttl_seconds: 3600
    max_entries: 1000
```

## Performance Optimizations

### Caching
- Cache assembled bundles by (chunk_id, options_hash)
- Cache graph traversals
- Cache token counts

### Streaming
- Stream content from files
- Progressive assembly
- Early termination on budget exhaustion

### Parallel Loading
```typescript
const [primary, tests, callers, callees] = await Promise.all([
  this.loadPrimary(chunkId),
  this.loadTests(chunkId),
  this.loadCallers(chunkId),
  this.loadCallees(chunkId)
]);
```

## Error Handling

### Graceful Degradation
- Missing relationships: Continue without
- File read errors: Skip item
- Token counting errors: Use estimates
- Budget exceeded: Truncate intelligently