# Maproom Search Architecture & Technical Design

## System Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend Layer                        │
├─────────────────────────────────────────────────────────────┤
│  Search Input │ Virtual List │ Animation Controller │ Cache  │
├─────────────────────────────────────────────────────────────┤
│                         API Gateway                          │
├─────────────────────────────────────────────────────────────┤
│                        Backend Services                      │
├──────────────┬────────────┬────────────┬───────────────────┤
│ Hybrid Search│  Neo4j     │  Indexer   │  Learning Engine  │
├──────────────┴────────────┴────────────┴───────────────────┤
│                      Data Layer                              │
├──────────────┬────────────┬────────────┬───────────────────┤
│  PostgreSQL  │   Neo4j    │   Redis    │   File System     │
└──────────────┴────────────┴────────────┴───────────────────┘
```

## Frontend Architecture

### Core Libraries & Rationale

#### Animation & Performance

```typescript
// Primary: Pure CSS with React
- CSS Transitions & Animations (hardware accelerated)
- React 18 Concurrent Features (startTransition, useDeferredValue)
- requestAnimationFrame for custom animations

// Libraries:
- framer-motion (already installed) - declarative animations
- react-window (already installed) - virtual scrolling
- react-intersection-observer - viewport detection

// Why CSS First:
1. GPU accelerated by default
2. No JavaScript overhead
3. Simpler to debug
4. Better battery life
5. Native browser optimizations
```

#### State Management

```typescript
// Search State Architecture
interface SearchState {
  query: string;
  results: SearchResult[];
  isLoading: boolean;
  fadeState: 'idle' | 'fading-out' | 'fading-in';
  scrollPosition: number;
  hasMore: boolean;
}

// Libraries:
- Zustand - lightweight state management
- React Query/TanStack Query - server state caching
- Jotai - atomic state for fine-grained updates
```

### CSS Animation Strategy

#### Smooth Fade Transitions

```css
/* Result container animations */
.search-results {
  position: relative;
}

.results-layer {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  will-change: opacity;
  contain: layout style paint;
}

.results-fade-out {
  animation: fadeOut 300ms ease-in forwards;
}

.results-fade-in {
  animation: fadeIn 300ms ease-out forwards;
}

@keyframes fadeOut {
  from { opacity: 1; }
  to { opacity: 0; pointer-events: none; }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* GPU-accelerated transforms */
.result-row {
  transform: translateZ(0); /* Force GPU layer */
  will-change: transform;
  contain: layout style;
}

/* Large number styling */
.row-number {
  position: sticky;
  right: 20px;
  font-size: 72px;
  font-weight: 900;
  color: #000;
  opacity: 0.1;
  mix-blend-mode: multiply;
  pointer-events: none;
  z-index: 10;
}
```

#### Performance Optimizations

```css
/* Reduce reflow/repaint */
.search-container {
  contain: strict; /* Isolate layout calculations */
  content-visibility: auto; /* Skip offscreen rendering */
}

/* Smooth scrolling */
.virtual-list {
  scroll-behavior: smooth;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch; /* iOS momentum */
}

/* Prefers reduced motion support */
@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

### Virtual Scrolling Implementation

```typescript
import { VariableSizeList } from 'react-window';

const SearchResults = () => {
  // Dynamic row heights for code snippets
  const getItemSize = (index: number) => {
    const result = results[index];
    const lineCount = result.preview.split('\n').length;
    return 100 + (lineCount * 20); // Base + lines
  };

  return (
    <VariableSizeList
      height={window.innerHeight - 60} // Full height minus input
      itemCount={results.length}
      itemSize={getItemSize}
      width="100%"
      overscanCount={5} // Render 5 extra items
      onScroll={handleScroll}
    >
      {ResultRow}
    </VariableSizeList>
  );
};
```

## Backend Architecture

### Hybrid Search Design

```typescript
interface HybridSearcher {
  // Combine multiple search strategies
  search(query: string): Promise<SearchResult[]> {
    const [semantic, lexical, graph] = await Promise.all([
      this.semanticSearch(query),
      this.lexicalSearch(query),
      this.graphSearch(query)
    ]);
    
    return this.fuseResults(semantic, lexical, graph);
  }
  
  // Intelligent result fusion
  fuseResults(...resultSets): SearchResult[] {
    // Reciprocal Rank Fusion (RRF)
    const scores = new Map();
    
    resultSets.forEach((results, sourceIndex) => {
      results.forEach((result, rank) => {
        const score = 1 / (rank + 60); // RRF constant
        const current = scores.get(result.id) || 0;
        scores.set(result.id, current + score * weights[sourceIndex]);
      });
    });
    
    return Array.from(scores.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([id]) => resultMap.get(id));
  }
}
```

### Neo4j Code Graph Schema

```cypher
// Node Types
(:File {path, language, size, modified})
(:Function {name, signature, lineStart, lineEnd})
(:Class {name, extends, implements})
(:Variable {name, type, scope})
(:Import {source, symbols})

// Relationships
(:Function)-[:CALLS]->(:Function)
(:Function)-[:RETURNS]->(:Type)
(:Class)-[:EXTENDS]->(:Class)
(:Class)-[:IMPLEMENTS]->(:Interface)
(:File)-[:IMPORTS]->(:File)
(:Function)-[:DEFINED_IN]->(:File)
(:Variable)-[:USED_BY]->(:Function)

// Example Queries
// Find all callers of a function
MATCH (caller:Function)-[:CALLS]->(target:Function {name: $functionName})
RETURN caller, target

// Find impact of changing a file
MATCH (file:File {path: $filePath})<-[:IMPORTS]-(dependent:File)
RETURN dependent

// Find unused functions
MATCH (f:Function)
WHERE NOT (f)<-[:CALLS]-()
RETURN f
```

### Real-time Indexing Pipeline

```typescript
class IncrementalIndexer {
  private queue = new PriorityQueue<IndexTask>();
  private watcher: FSWatcher;
  
  async processChanges() {
    // Watch for file changes
    this.watcher = chokidar.watch('**/*.{ts,tsx,js,jsx}', {
      ignored: ['node_modules', '.git'],
      persistent: true
    });
    
    this.watcher.on('change', (path) => {
      // High priority for recently edited files
      this.queue.enqueue({
        path,
        priority: Priority.HIGH,
        timestamp: Date.now()
      });
    });
    
    // Process queue
    while (true) {
      const task = await this.queue.dequeue();
      await this.indexFile(task.path);
      
      // Update Neo4j relationships
      await this.updateGraph(task.path);
      
      // Invalidate cache
      await this.cache.invalidate(task.path);
    }
  }
}
```

## Performance Architecture

### Caching Strategy

```typescript
// Multi-tier caching
class CacheManager {
  private l1: Map<string, CacheEntry> = new Map(); // In-memory
  private l2: Redis; // Redis cache
  private l3: CDN; // Edge cache for static results
  
  async get(key: string): Promise<any> {
    // L1: Memory cache (< 1ms)
    if (this.l1.has(key)) {
      return this.l1.get(key);
    }
    
    // L2: Redis cache (< 10ms)
    const redisResult = await this.l2.get(key);
    if (redisResult) {
      this.l1.set(key, redisResult);
      return redisResult;
    }
    
    // L3: CDN for common queries
    const cdnResult = await this.l3.get(key);
    if (cdnResult) {
      await this.promote(key, cdnResult);
      return cdnResult;
    }
    
    return null;
  }
}
```

### Stream Processing

```typescript
// Server-Sent Events for progressive results
class SearchStreamer {
  async *streamResults(query: string) {
    const streams = [
      this.cacheStream(query),     // Instant cached results
      this.lexicalStream(query),   // Fast exact matches
      this.semanticStream(query),  // Slower semantic matches
      this.graphStream(query)      // Complex relationship queries
    ];
    
    // Merge streams as results arrive
    for await (const result of mergeAsyncIterators(streams)) {
      yield result;
    }
  }
}

// Client-side consumption
const eventSource = new EventSource(`/api/search/stream?q=${query}`);
eventSource.onmessage = (event) => {
  const result = JSON.parse(event.data);
  addResultWithAnimation(result);
};
```

## WebGL Fallback Architecture (If Needed)

```typescript
// Only if CSS performance insufficient
class WebGLRenderer {
  private gl: WebGL2RenderingContext;
  private textRenderer: TextRenderer;
  
  render(results: SearchResult[]) {
    // Render to offscreen canvas
    this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, this.framebuffer);
    
    // Batch render all text
    this.textRenderer.renderBatch(results.map(r => ({
      text: r.content,
      x: r.x,
      y: r.y,
      highlight: r.matches
    })));
    
    // Apply fade shader
    this.applyFadeTransition(this.fadeAmount);
    
    // Blit to screen
    this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, null);
    this.gl.drawArrays(this.gl.TRIANGLES, 0, 6);
  }
}
```

## WASM Integration (Nuclear Option)

```rust
// Rust search engine compiled to WASM
#[wasm_bindgen]
pub struct SearchEngine {
    index: TantivyIndex,
    graph: PetgraphStructure,
}

#[wasm_bindgen]
impl SearchEngine {
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        // Parallel search across cores
        let results = rayon::join(
            || self.index.search(query),
            || self.graph.search(query)
        );
        
        self.merge_results(results)
    }
}
```

## Monitoring & Observability

```typescript
// Performance monitoring
class PerformanceMonitor {
  metrics = {
    searchLatency: new Histogram(),
    animationFPS: new Gauge(),
    memoryUsage: new Gauge(),
    cacheHitRate: new Counter()
  };
  
  trackSearch(query: string) {
    const start = performance.now();
    
    return {
      complete: () => {
        const duration = performance.now() - start;
        this.metrics.searchLatency.observe(duration);
        
        // Track if we're meeting SLA
        if (duration > 100) {
          console.warn(`Slow search: ${query} took ${duration}ms`);
        }
      }
    };
  }
  
  trackAnimation() {
    let lastTime = performance.now();
    let frames = 0;
    
    const measure = () => {
      frames++;
      const now = performance.now();
      
      if (now - lastTime >= 1000) {
        this.metrics.animationFPS.set(frames);
        frames = 0;
        lastTime = now;
      }
      
      requestAnimationFrame(measure);
    };
    
    measure();
  }
}
```

## Library Recommendations

### Essential Libraries

```json
{
  "dependencies": {
    // Already installed - use these
    "framer-motion": "^12.23.12",        // Animations
    "react-window": "^1.8.11",           // Virtual scrolling
    "@apollo/client": "^3.13.9",         // GraphQL client
    "lucide-react": "^0.539.0",          // Icons
    
    // Need to add
    "tanstack-query": "^5.x",            // Server state
    "zustand": "^4.x",                    // Client state
    "fuse.js": "^7.x",                   // Fuzzy search
    "comlink": "^4.x",                   // Web Workers
    "react-intersection-observer": "^9.x" // Viewport detection
  },
  
  "devDependencies": {
    // Performance testing
    "lighthouse": "^11.x",               // Performance audits
    "bundlesize": "^0.18.x"              // Bundle monitoring
  }
}
```

### Neo4j Integration

```yaml
# docker-compose addition
neo4j:
  image: neo4j:5-community
  environment:
    NEO4J_AUTH: neo4j/password
    NEO4J_PLUGINS: '["apoc", "graph-data-science"]'
  ports:
    - "7474:7474"  # Browser
    - "7687:7687"  # Bolt
  volumes:
    - neo4j_data:/data
```

## Deployment Architecture

```yaml
# Kubernetes deployment for scale
apiVersion: apps/v1
kind: Deployment
metadata:
  name: maproom-search
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: search-api
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
      - name: neo4j
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
```

## Success Metrics Dashboard

```typescript
// Real-time metrics display
const MetricsDashboard = () => {
  return (
    <div className="metrics-grid">
      <MetricCard 
        title="Search Latency P95"
        value={metrics.searchLatency.p95}
        target={100}
        unit="ms"
      />
      <MetricCard 
        title="Animation FPS"
        value={metrics.fps.current}
        target={60}
        unit="fps"
      />
      <MetricCard 
        title="Cache Hit Rate"
        value={metrics.cacheHitRate}
        target={60}
        unit="%"
      />
      <MetricCard 
        title="Maproom Usage"
        value={metrics.maproomUsageRate}
        target={80}
        unit="%"
      />
    </div>
  );
};
```

This architecture provides a clear path from CSS-first implementation to WebGL/WASM if needed, with careful performance monitoring at each stage.
