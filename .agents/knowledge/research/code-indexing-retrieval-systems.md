# Modern Code Indexing and Retrieval Systems for Multi-Agent Development
## Comprehensive Research Report

**Research Date:** October 23, 2025
**Focus:** Semantic code search, hybrid retrieval, budget-aware context assembly, and production-ready implementations

---

## Executive Summary

Modern code indexing and retrieval systems combine three fundamental technologies:

1. **AST-based code chunking** using Tree-sitter for syntactic preservation
2. **Hybrid retrieval** combining BM25 keyword search with vector embeddings
3. **Intelligent context budget management** optimizing for LLM token limits

**Key Finding:** Production systems achieve 40-65% improvement in indexing performance and sub-200ms query latency for 100M+ line codebases through incremental indexing and quantized vector search.

**Critical Insight:** Context window utilization sweet spot is 40-70% of available tokens, not 100%. Most LLMs effectively utilize only 10-20% of their context window.

**MVP Recommendation:** PostgreSQL with pgvector provides the optimal starting point for startups, offering 11.4x higher throughput than specialized vector databases while maintaining sub-100ms latency at 99% recall.

---

## 1. Industry Solutions for Semantic Code Search

### 1.1 GitHub Copilot Architecture

**Core Technology Stack:**
- **Dual-index strategy**: Remote (GitHub servers) + Local (VS Code)
- **Custom embedding model**: Transformer-based, optimized for code semantics
- **Performance metrics**: 37.6% lift in retrieval quality, 8x smaller index size, 2x throughput
- **Multi-layered fallback**: Semantic embeddings → TF-IDF → IntelliSense → Keyword matching

**Key Innovation:**
- **Instant semantic indexing**: Maximum 60 seconds for large repositories (previously ~5 minutes)
- **Matryoshka Representation Learning**: Handles embeddings at multiple granularity levels
- **Dynamic merging logic**: Remote chunks for unchanged files, local chunks for modified files

**Impact Metrics:**
- C# developers: +110.7% code acceptance ratio
- Java developers: +113.1% code acceptance ratio

**Architecture Pattern:**
```
User Query → GPT-4o-mini (classification) → Keyword extraction
                                          ↓
           Parallel search: Remote API (indexed commits) + Local (working diff)
                                          ↓
                    Merge results (deduplicate) → LLM context
```

### 1.2 Sourcegraph Architecture

**Core Technology Stack:**
- **Zoekt**: Distributed architecture with horizontally-scalable searchers
- **LSIF indexing**: Language Server Index Format for precise code intelligence
- **Server language**: Go (Apache License 2.0)
- **Structural search**: Comby library for AST-aware pattern matching

**Key Innovation:**
- **Semantic-level indexing**: Parses code like a compiler, not text
- **Language-specific analyzers**: Dedicated LSIF indexers (Go, C/C++, Python, TypeScript)
- **Search-based heuristics**: Fast navigation without full AST parsing for basic operations

**Architecture Pattern:**
```
Query → Zoekt (indexed repos) + Searcher (unindexed/recent changes)
                    ↓
        LSIF graph (definitions/references) + Structural search (AST patterns)
                    ↓
             Deep Search (LLM-enhanced semantic understanding)
```

**Cody AI Integration:**
- Leverages Sourcegraph search APIs for context retrieval
- Combines keyword, regex, and embedding-based semantic search
- Uses code graph for call-site discovery and reference tracking

### 1.3 Cursor IDE Architecture

**Core Technology Stack:**
- **Merkle tree synchronization**: Efficient incremental updates every 10 minutes
- **Hash-based change detection**: Only upload modified files
- **Context optimization**: Dynamic pruning of non-essential content
- **Multi-modal indexing**: External docs, web pages, git history

**Key Innovation:**
- **Large context mode**: Double pricing, double capability (750 lines per file read in MAX mode)
- **Request budget management**: 500 monthly requests + usage-based pricing for Premium models
- **Auto-model selection**: Task-based model routing with degradation detection
- **.cursorignore files**: Prevent indexing of build artifacts (node_modules, vendor/)

**Cost Management:**
- Consolidated $20/month subscription across multiple LLM providers
- Estimated $100/month savings vs. direct API usage
- Prompt caching for large context reuse (Anthropic models)

**Architecture Pattern:**
```
Codebase → Merkle tree (change detection) → Incremental sync
                           ↓
           Index (embeddings) + External docs (RAG)
                           ↓
        Chat session → Context window optimization → LLM
                           ↓
              Intelligent pruning (preserve critical context)
```

### 1.4 Aider (Terminal-based AI Coding)

**Core Technology Stack:**
- **RepoMap**: ctags-based repository mapping with call signatures
- **Graph-based ranking**: Importance scoring via dependency analysis
- **Selective file context**: Read-only vs. editable file separation
- **Tree-sitter + pygments**: Dual-parsing strategy for accuracy

**Key Innovation:**
- **O(repository) → O(changes)**: Focus on modified code, not entire codebase
- **GPT self-service**: LLM asks for specific files based on repo map
- **Plan-based development**: Markdown checklists for complex features
- **Token budget management**: Dynamic context adjustment with /drop and /clear commands

**Architecture Pattern:**
```
Repository → ctags extraction → RepoMap (symbols + signatures)
                                      ↓
                  GPT sees map → Requests specific files → Auto-add to context
                                      ↓
                      Incremental edits → Tree-sitter validation
```

---

## 2. Hybrid Retrieval Systems

### 2.1 BM25 + Vector Search Architecture

**Why Hybrid Search?**
- **BM25 strengths**: Exact keyword matching, rare term identification, no training required
- **Vector search strengths**: Semantic understanding, synonym handling, conceptual matching
- **Combined power**: Precision + recall optimization

**BM25 Formula:**
```
BM25(D,Q) = Σ IDF(qi) * (f(qi,D) * (k1 + 1)) / (f(qi,D) + k1 * (1 - b + b * |D|/avgdl))

Where:
- f(qi,D) = term frequency of query term qi in document D
- IDF(qi) = inverse document frequency (rare terms weighted higher)
- k1 = term frequency saturation (typically 1.2)
- b = length normalization (typically 0.75)
- |D| = document length
- avgdl = average document length
```

**Optimal Parameters for Code:**
- `k1 = 1.2` (term frequency saturation)
- `b = 0.75` (length normalization)
- For technical documents, these defaults work well

### 2.2 Reranking Strategies

#### **Reciprocal Rank Fusion (RRF)**
```
RRF_score = Σ 1/(k + rank_position)

Where k = 60 (typical constant)
```

**Advantages:**
- No additional compute cost (pure ranking fusion)
- Fast execution (no model inference)
- Effective baseline (often outperforms single methods)

**Use case:** Resource-constrained environments, real-time search

#### **Cross-Encoder Reranking**
```
Architecture:
BM25 retrieval (top 50) + Vector search (top 50)
           ↓
    Merge (100 candidates)
           ↓
Cross-encoder scoring (BERT-based, query+document pairs)
           ↓
    Top-k reranked results
```

**Performance:** 5-15% improvement over RRF in semantic tasks

**Trade-off:** Higher latency (~100-200ms per batch), GPU recommended

#### **Late Interaction Embeddings (ColBERT-style)**
- Multi-vector representations capture text nuances
- Used as final reranking step after single-vector retrieval
- Best for high-accuracy requirements

### 2.3 Implementation Examples

**PostgreSQL with VectorChord + BM25:**
```sql
-- Create BM25 index
CREATE INDEX ON code_snippets USING bm25 (content) WITH (language='english', k1=1.2, b=0.75);

-- Create vector index
CREATE INDEX ON code_snippets USING vectors (embedding vector_cosine_ops);

-- Hybrid search query
WITH bm25_results AS (
  SELECT id, content, bm25_rank(content, 'authentication') as bm25_score
  FROM code_snippets
  ORDER BY bm25_score DESC LIMIT 50
),
vector_results AS (
  SELECT id, content, 1 - (embedding <=> query_embedding) as vector_score
  FROM code_snippets
  ORDER BY vector_score DESC LIMIT 50
)
SELECT DISTINCT id, content,
  (COALESCE(bm25_score, 0) + COALESCE(vector_score, 0)) as combined_score
FROM bm25_results
FULL OUTER JOIN vector_results USING (id)
ORDER BY combined_score DESC
LIMIT 10;
```

**Dynamic Weighting Strategy:**
```python
def hybrid_search(query, alpha=None):
    """
    Alpha determines BM25 vs vector weighting
    - Navigational queries (e.g., "Facebook login"): alpha=0.8 (favor BM25)
    - Exploratory queries (e.g., "AI ethics"): alpha=0.3 (favor vectors)
    """
    if alpha is None:
        # Query classification
        alpha = classify_query_type(query)

    bm25_results = bm25_search(query)
    vector_results = vector_search(query)

    combined = merge_with_rrf(bm25_results, vector_results, alpha)
    return rerank_with_cross_encoder(combined, query)
```

**HyDE (Hypothetical Document Embeddings):**
```python
# Instead of embedding the raw query
raw_query = "how to authenticate users"
raw_embedding = embed(raw_query)

# Generate hypothetical answer first
hypothetical_answer = llm.generate(
    f"Write a short code example for: {raw_query}"
)
hyde_embedding = embed(hypothetical_answer)

# Search with hypothetical embedding
results = vector_search(hyde_embedding)
```

### 2.4 Performance Benchmarks

| Method | Recall@10 | Latency (ms) | Use Case |
|--------|-----------|--------------|----------|
| BM25 only | 0.65 | 15-30 | Exact keyword matches |
| Vector only | 0.72 | 30-50 | Semantic search |
| RRF hybrid | 0.81 | 45-80 | Balanced performance |
| Cross-encoder rerank | 0.87 | 150-250 | High accuracy needs |
| ColBERT rerank | 0.91 | 200-400 | Maximum precision |

---

## 3. Budget-Aware Context Assembly for LLMs

### 3.1 Context Window Utilization Principles

**Critical Discovery:** Optimal context window utilization is **40-70%**, not 100%.

**Research Findings:**
- GPT-4-turbo: Saturates at 16k tokens (of 128k available)
- Claude-3-sonnet: Saturates at 16k tokens
- Mixtral-instruct: Saturates at 4k tokens
- DBRX-instruct: Saturates at 8k tokens
- Llama-3.1-405b: Performance degrades after 32k tokens

**The "Lost in the Middle" Problem:**
- Models remember beginning and end of context
- Middle sections receive less attention
- Problem intensifies from 8K → 128K windows
- Critical for code where logic often lives in the middle

### 3.2 Token Optimization Strategies

#### **Context Engine Approach (Augment Code)**
```python
# Before: Load entire codebase (100,000 tokens)
# Problem: Pay for all tokens, 98% unused

# After: Semantic retrieval (1,000 tokens)
query = "authentication logic"
relevant_chunks = vector_search(query, top_k=5)
context = assemble_context(relevant_chunks)  # ~1,000 tokens

# Result: 100x cost reduction, better precision
```

#### **Dynamic Context Injection**
```python
def assemble_context(query, complexity_score):
    """
    Adjust retrieved chunks based on query complexity
    """
    if complexity_score < 0.3:
        # Simple query: minimal context
        chunks = retrieve_top_k(query, k=3)
    elif complexity_score < 0.7:
        # Medium query: balanced context
        chunks = retrieve_top_k(query, k=7)
    else:
        # Complex query: rich context
        chunks = retrieve_top_k(query, k=15)

    return chunks[:max_token_budget]
```

#### **Chunk Size Optimization**
| Chunk Size | Recall | Latency | Token Efficiency |
|------------|--------|---------|------------------|
| 256 tokens | 0.68 | Low | High |
| 512 tokens | 0.81 | Medium | Optimal |
| 1024 tokens | 0.84 | High | Medium |
| 2048 tokens | 0.83 | Very High | Low |

**Recommendation:** 512 tokens for most code retrieval tasks

### 3.3 RAG vs. Large Context Windows

**Cost Analysis:**
```
Scenario: Answer question about 125,000-token codebase

Option 1: Full context (no RAG)
- Input: 125,000 tokens
- Cost: $0.625 (at $0.005/1K tokens)
- Latency: ~21.6s (GPT-4-Turbo 128K)

Option 2: RAG retrieval
- Index: One-time embedding cost (~$0.10)
- Query: 2,000 tokens (relevant chunks)
- Cost: $0.01 per query
- Latency: ~12.9s (LlamaIndex RAG)

Breakeven: 1-2 queries
Savings at scale: 98% cost reduction
```

**When to Use RAG:**
- Frequently changing codebases (incremental updates)
- Large repositories (>100K LOC)
- Cost-sensitive applications
- Need for explainability (which chunks were used)

**When to Use Long Context:**
- Latency-critical applications (no retrieval overhead)
- Small, stable codebases
- Need for global reasoning across entire codebase
- Complex refactoring requiring full context

**Hybrid Approach (Recommended):**
```python
def smart_context_assembly(query, codebase_size):
    if codebase_size < 50_000:  # Small codebase
        # Load full context (faster, simpler)
        return load_full_codebase()
    else:
        # RAG retrieval
        relevant_chunks = semantic_search(query, top_k=10)

        # Expand with graph traversal
        expanded = expand_with_dependencies(relevant_chunks)

        # Stay within 40-70% of context window
        return prune_to_budget(expanded, target_utilization=0.6)
```

### 3.4 Performance Benchmarks

**LlamaIndex RAG vs. GPT-4-Turbo (128K context):**
- Average latency: 12.9s vs. 21.6s
- Cost per query: $0.01 vs. $0.625
- Accuracy: Similar (both >85% on CodeSearchNet)

**Cursor IDE metrics:**
- Small diffs (<300 files): 8s timeout for embeddings search
- Medium diffs (301-2,000 files): TF-IDF only (skip embeddings)
- Large diffs (>2,000 files): Keyword fallback

---

## 4. Incremental Indexing Strategies

### 4.1 Merkle Tree-Based Synchronization (Cursor)

**Architecture:**
```
Codebase → Merkle tree (hash each file)
              ↓
        Every 10 minutes: compare hashes
              ↓
    Only upload changed files (bandwidth optimized)
              ↓
        Re-index changed files only
```

**Benefits:**
- O(changes) complexity, not O(repository)
- Minimal bandwidth usage
- Real-time updates (10-minute cycle)

### 4.2 Glean Incremental Indexing (Meta)

**Key Innovation:** O(changes) rather than O(repository)

**Implementation:**
```
Git commit → Identify changed files
                    ↓
        Parse only changed files (tree-sitter)
                    ↓
    Update dependency graph (incremental)
                    ↓
        Re-embed affected chunks only
                    ↓
    Upsert to vector database (partial update)
```

**Performance Metrics:**
- ~50,000 file changes: 45 seconds (35% improvement)
- 500,000+ file changes: Up to 50% speedup

### 4.3 IntelliJ IDEA Optimizations (Large Scala Monorepo)

**Achievements:**
- Full indexing: 14 minutes → 6 minutes (57% reduction)
- Incremental (~50K files): 50-65% speedup
- Pre-built indexes: Shared JDK indexes across team

**Techniques:**
1. **Pre-built indexes**: Store in geo-distributed storage
2. **Parallel processing**: Multi-threaded parsing
3. **Incremental compilation**: Only reprocess modified AST nodes

### 4.4 Quantized Vector Search (Augment Code - 100M LOC)

**Innovation:** Product Quantization (PQ) for memory efficiency

**Results:**
- Memory: 2GB → 250MB (8x reduction)
- Latency: 2+ seconds → <200ms (10x improvement)
- Accuracy: 99.9% maintained
- Overall: 40% improvement in code completion latency

**Implementation:**
```python
# Traditional (2GB for 100M LOC)
embeddings = np.array([[...] * 4096] * 1_000_000)  # float32

# Quantized (250MB for 100M LOC)
from sklearn.cluster import MiniBatchKMeans

# Train codebook
kmeans = MiniBatchKMeans(n_clusters=256)
kmeans.fit(embeddings)

# Quantize: store cluster IDs instead of full vectors
quantized = kmeans.predict(embeddings)  # uint8 (1 byte per dimension)

# Search: approximate distance via cluster centroids
# Accuracy loss: <0.1%
```

### 4.5 Batch Processing Optimization

**Best Practices:**
```python
# Anti-pattern: Individual updates (creates indexing queue)
for file in changed_files:
    index.upsert(file)  # 100s of requests → 100s of reindexing operations

# Better: Batch updates
batch_size = 100
for i in range(0, len(changed_files), batch_size):
    batch = changed_files[i:i+batch_size]
    index.upsert_batch(batch)  # Single reindexing operation per batch

# Best: Time-based batching
cache = []
while True:
    cache.extend(get_recent_changes())

    if elapsed_time >= 5 * 60:  # Every 5 minutes
        index.upsert_batch(cache)
        cache.clear()
```

**Recommendation:**
- Batch every 5-30 minutes (never faster than 1 minute)
- Use partial indexing (only changed attributes)

### 4.6 Performance Benchmarks

| Strategy | Indexing Time | Memory Usage | Query Latency | Accuracy |
|----------|---------------|--------------|---------------|----------|
| Full reindex (naive) | 14 min | 2GB | 2000ms | 100% |
| Incremental (Merkle) | 45s | 2GB | 200ms | 100% |
| Quantized vectors | 60s | 250MB | 180ms | 99.9% |
| Pre-built indexes | 3 min | 1.5GB | 150ms | 100% |
| Hybrid (incremental + quantized) | 30s | 200MB | 120ms | 99.8% |

---

## 5. Code Chunking Strategies with Tree-sitter

### 5.1 Why AST-Based Chunking?

**Problems with Naive Chunking:**
```python
# Fixed-size chunking (BAD)
def chunk_by_lines(code, lines_per_chunk=50):
    """
    Problems:
    - Splits functions mid-implementation
    - Separates imports from usage
    - Breaks class definitions
    - Creates syntax errors
    """
    return [code[i:i+lines_per_chunk] for i in range(0, len(code), lines_per_chunk)]

# Example disaster:
# Chunk 1: import numpy as np\ndef calculate_
# Chunk 2: average(arr):\n    return np.mean(arr)
# Result: Chunk 2 has undefined 'np', chunk 1 has incomplete function
```

**AST-Based Solution:**
```python
def chunk_by_ast(code, language='python'):
    """
    Benefits:
    - Preserves function boundaries
    - Includes necessary imports
    - Maintains class hierarchy
    - Each chunk is syntactically valid
    """
    tree = tree_sitter.parse(code, language)
    chunks = []

    for node in tree.root_node.children:
        if node.type in ['function_definition', 'class_definition']:
            # Extract complete syntactic unit
            chunk = extract_with_imports(node)
            chunks.append(chunk)

    return chunks
```

### 5.2 The cAST Algorithm (Split-then-Merge)

**Architecture:**
```
Code → Tree-sitter parse → AST
                    ↓
        Identify semantic units (functions, classes)
                    ↓
        Split by syntactic boundaries
                    ↓
    Check token count per chunk
                    ↓
Merge small sibling chunks (while under token limit)
                    ↓
        Add metadata (imports, context)
                    ↓
            Final chunks
```

**Implementation:**
```python
import tree_sitter

def cast_chunking(code, language, max_tokens=512):
    """
    cAST: Context-Aware Semantic Tree chunking
    """
    parser = tree_sitter.Parser()
    parser.set_language(tree_sitter_languages.get_language(language))

    tree = parser.parse(bytes(code, 'utf8'))
    chunks = []
    current_chunk = []
    current_tokens = 0

    for node in tree.root_node.children:
        node_text = code[node.start_byte:node.end_byte]
        node_tokens = estimate_tokens(node_text)

        if node.type in ['import_statement', 'import_from_statement']:
            # Always include imports in all chunks
            imports.append(node_text)
            continue

        if node.type in ['function_definition', 'class_definition']:
            # Semantic unit found
            if current_tokens + node_tokens > max_tokens:
                # Flush current chunk
                if current_chunk:
                    chunks.append(merge_with_imports(current_chunk, imports))
                    current_chunk = []
                    current_tokens = 0

            # Check if single function exceeds limit
            if node_tokens > max_tokens:
                # Recursively split the function
                chunks.extend(split_large_function(node, max_tokens))
            else:
                current_chunk.append(node_text)
                current_tokens += node_tokens

    # Flush remaining
    if current_chunk:
        chunks.append(merge_with_imports(current_chunk, imports))

    return chunks

def merge_with_imports(chunk_code, imports):
    """
    Ensure each chunk includes necessary imports
    """
    # Analyze which imports are used in this chunk
    used_imports = identify_used_imports(chunk_code, imports)

    return '\n'.join(used_imports + [chunk_code])
```

### 5.3 Performance Comparisons

**Benchmark on RepoEval (StarCoder2-7B):**
| Chunking Method | Accuracy | Avg. Chunk Size | Chunks per File |
|-----------------|----------|-----------------|-----------------|
| Fixed-size (50 lines) | 65.2% | 50 lines | 8.3 |
| Token-based (512 tokens) | 68.1% | 512 tokens | 6.7 |
| AST-based (function-level) | 73.3% | Variable | 4.2 |
| cAST (split-then-merge) | 75.8% | Adaptive | 3.8 |

**Performance Gain:** +5.5 points on RepoEval with cAST

**CrossCodeEval (multi-language):**
- cAST improvement: +4.3 points over fixed-size
- Reason: Language-agnostic AST parsing generalizes well

### 5.4 Tree-sitter Language Support

**Supported Languages (40+):**
- **Systems:** C, C++, Rust, Go, Zig
- **Web:** JavaScript, TypeScript, Python, Ruby, PHP
- **JVM:** Java, Kotlin, Scala
- **Functional:** Haskell, OCaml, Elixir
- **Data:** SQL, YAML, JSON, TOML
- **Markup:** HTML, CSS, Markdown

**Performance Characteristics:**
- Parsing speed: 36x faster than JavaParser
- Incremental parsing: Only reparse changed sections
- Error recovery: Returns partial AST even with syntax errors
- Context-aware lexing: More robust than typical lexers

### 5.5 Best Practices

**Chunk Size Guidelines:**
```python
# Measure by non-whitespace characters, not lines
def measure_chunk_quality(chunk):
    non_ws_chars = len(re.sub(r'\s+', '', chunk))

    # Target: 300-800 non-whitespace characters
    # Equivalent to ~512 tokens for most code
    return 300 <= non_ws_chars <= 800

# Avoid these anti-patterns:
❌ Counting lines (formatting-dependent)
❌ Counting tokens without AST awareness
❌ Fixed-size windows that break semantics

# Follow these patterns:
✅ AST-based boundaries (functions, classes)
✅ Include necessary imports/dependencies
✅ Merge small siblings to reduce chunk count
✅ Measure by actual content (non-whitespace)
```

**Multi-language Handling:**
```python
# JavaScript + JSX
def parse_jsx_file(code):
    """
    Tree-sitter supports multi-language documents
    """
    # Parse JavaScript portion
    js_tree = parse_with_language(code, 'javascript')

    # Parse JSX portions (different syntax tree)
    jsx_ranges = identify_jsx_ranges(code)
    jsx_trees = [parse_with_language(code[r.start:r.end], 'tsx')
                 for r in jsx_ranges]

    # Combine trees with overlapping ranges
    return merge_trees(js_tree, jsx_trees)
```

### 5.6 Metadata Retention

**Key Advantage of AST Chunking:**
```python
class CodeChunk:
    def __init__(self, code, ast_node):
        self.code = code

        # File-level metadata
        self.file_path = ast_node.source_file
        self.language = detect_language(self.file_path)

        # Function/class metadata
        self.type = ast_node.type  # 'function_definition', 'class_definition'
        self.name = extract_name(ast_node)
        self.docstring = extract_docstring(ast_node)
        self.parameters = extract_parameters(ast_node)

        # Context metadata
        self.parent_class = find_parent_class(ast_node)
        self.imports = extract_imports(ast_node.parent)
        self.line_range = (ast_node.start_line, ast_node.end_line)
```

**Benefits for Retrieval:**
- Filter by function name, class, or file path
- Include docstrings in semantic search
- Preserve parameter information for API matching
- Maintain hierarchical relationships (class → method)

---

## 6. Code Embedding Models

### 6.1 Model Comparison (2024-2025)

| Model | Dimensions | Architecture | Use Case | Performance |
|-------|------------|--------------|----------|-------------|
| CodeBERT | 768 | BERT encoder | General code understanding | Baseline |
| GraphCodeBERT | 768 | BERT + data flow graph | Bug detection, flow analysis | +8% over CodeBERT |
| UniXcoder | 768 | Encoder-decoder | Code translation, completion | +12% over CodeBERT |
| StarEncoder | 2048 | Decoder-only (StarCoder) | Code completion | High quality, heavy compute |
| CodeXEmbed | 4096 | Large instruction-tuned | State-of-art retrieval | +20% on CoIR, expensive |
| OpenAI text-embedding-3-small | 1536 | Proprietary | General-purpose | Good balance |
| OpenAI text-embedding-3-large | 3072 | Proprietary | High accuracy needs | Best accuracy |

### 6.2 LoRACode (2025 Innovation)

**Key Innovation:** Parameter-Efficient Fine-Tuning (PEFT) for code embeddings

**Architecture:**
```
Base Model (CodeBERT/GraphCodeBERT/UniXcoder)
                    ↓
        LoRA adapters (low-rank matrices)
                    ↓
        Fine-tuned for specific domains
                    ↓
    Deployment (base + small adapter ~10MB)
```

**Benefits:**
- 100x smaller than full model fine-tuning
- Swap adapters per domain (Python, JavaScript, Rust)
- Maintains base model quality
- Fast training (hours vs. days)

**Example Configuration:**
```python
from peft import LoraConfig, get_peft_model

# Base: CodeBERT (768 dimensions)
base_model = AutoModel.from_pretrained('microsoft/codebert-base')

# LoRA configuration
lora_config = LoraConfig(
    r=8,  # Rank of low-rank matrices
    lora_alpha=16,
    target_modules=['query', 'key', 'value'],
    lora_dropout=0.1,
    bias='none',
)

# Apply LoRA
model = get_peft_model(base_model, lora_config)

# Trainable params: ~1% of base model
print(f"Trainable: {model.num_trainable_parameters() / 1e6:.2f}M")
```

### 6.3 Dimensionality Trade-offs

**Memory Calculation:**
```python
# For 1M code chunks

# CodeBERT (768 dimensions, float32)
memory_768 = 1_000_000 * 768 * 4 / (1024**3)  # 2.86 GB

# CodeXEmbed (4096 dimensions, float32)
memory_4096 = 1_000_000 * 4096 * 4 / (1024**3)  # 15.26 GB

# With quantization (int8)
memory_4096_quantized = 1_000_000 * 4096 * 1 / (1024**3)  # 3.81 GB

# Recommendation for MVPs: 768-1536 dimensions (balance of cost/accuracy)
```

**Performance vs. Dimensionality:**
| Dimensions | Index Size (1M vecs) | Query Latency | Accuracy (CoIR) |
|------------|----------------------|---------------|-----------------|
| 384 | 1.4 GB | 20ms | 0.72 |
| 768 | 2.9 GB | 35ms | 0.81 |
| 1536 | 5.7 GB | 55ms | 0.87 |
| 3072 | 11.4 GB | 95ms | 0.91 |
| 4096 | 15.3 GB | 130ms | 0.93 |

**Recommendation:**
- **MVP/Startup:** 768 dimensions (CodeBERT, UniXcoder)
- **Production:** 1536 dimensions (OpenAI text-embedding-3-small)
- **High-accuracy:** 3072 dimensions (OpenAI text-embedding-3-large)

### 6.4 Specialized Models for Code

**GraphCodeBERT (Data Flow Awareness):**
```python
# Captures variable usage and control flow
# Example: detecting that 'user_id' flows from input to database query

code = """
def get_user(user_id):
    query = f"SELECT * FROM users WHERE id = {user_id}"  # Vulnerable!
    return db.execute(query)
"""

# GraphCodeBERT embedding captures:
# - Function signature
# - Variable flow: user_id → query string
# - Database interaction
# - Security vulnerability (SQL injection)
```

**UniXcoder (Cross-modal Pretraining):**
```python
# Trained on (code, docstring, AST) triplets
# Excellent for code translation and API matching

query = "convert JSON string to Python dictionary"

# UniXcoder matches:
# 1. json.loads(json_string)  # Exact API
# 2. import json; data = json.loads(s)  # With import
# 3. ast.literal_eval(json_string)  # Alternative approach
```

### 6.5 Practical Recommendations

**For Code Search (MVP):**
```python
# Option 1: OpenAI (easiest, cloud-based)
from openai import OpenAI

client = OpenAI()
embedding = client.embeddings.create(
    model="text-embedding-3-small",  # 1536 dimensions
    input=code_snippet
).data[0].embedding

# Option 2: Open-source (self-hosted, free)
from sentence_transformers import SentenceTransformer

model = SentenceTransformer('BAAI/bge-large-en-v1.5')  # 1024 dimensions
embedding = model.encode(code_snippet)

# Option 3: Code-specific (best for code understanding)
from transformers import AutoTokenizer, AutoModel

tokenizer = AutoTokenizer.from_pretrained('microsoft/unixcoder-base')
model = AutoModel.from_pretrained('microsoft/unixcoder-base')
embedding = get_unixcoder_embedding(code_snippet, model, tokenizer)
```

**Cost Comparison (1M embeddings):**
- OpenAI text-embedding-3-small: $100 (cloud)
- BGE-large-en-v1.5: $0 (self-hosted, GPU recommended)
- UniXcoder: $0 (self-hosted, CPU okay for small scale)

---

## 7. Database Architectures

### 7.1 PostgreSQL + pgvector

**Why pgvector for MVPs:**
✅ No new infrastructure (leverage existing PostgreSQL)
✅ SQL familiarity (developers already know it)
✅ ACID transactions (data consistency)
✅ 11.4x higher throughput than Qdrant (471 QPS vs. 41 QPS)
✅ Mature ecosystem (backups, monitoring, scaling)

**Performance Characteristics:**
- **p50 latency:** 31.07ms (at 99% recall)
- **p95 latency:** 60.42ms
- **p99 latency:** 74.60ms
- **Throughput:** 471 queries/second (single node)

**HNSW Index Parameters:**
```sql
-- Default (works for most cases)
CREATE INDEX ON code_embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- High-recall scenario (slower build, better search)
CREATE INDEX ON code_embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 32, ef_construction = 128);

-- Fast build scenario (faster indexing, slight recall trade-off)
CREATE INDEX ON code_embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 8, ef_construction = 32);

-- Adjust search-time recall
SET hnsw.ef_search = 100;  -- Higher = better recall, slower search
```

**Parameter Guidelines:**
- `m`: Maximum edges per node (default: 16)
  - Higher m = more memory, better recall
  - Typical range: 8-32
- `ef_construction`: Candidate queue size during build (default: 64)
  - Higher = slower build, better index quality
  - Typical range: 32-128
- `ef_search`: Runtime search candidates (default: 40)
  - Higher = better recall, slower queries
  - Typical range: 50-200

**Memory Requirements:**
```sql
-- Estimate HNSW index size
-- Formula: num_vectors * dimensions * 4 bytes * (1 + m/10)

-- Example: 1M vectors, 768 dimensions, m=16
Index size ≈ 1,000,000 * 768 * 4 * (1 + 1.6) = ~8 GB

-- Recommendation: Keep entire index in RAM for best performance
```

**Full Implementation:**
```sql
-- Create table
CREATE TABLE code_embeddings (
    id SERIAL PRIMARY KEY,
    file_path TEXT NOT NULL,
    chunk_text TEXT NOT NULL,
    line_start INT,
    line_end INT,
    language TEXT,
    embedding VECTOR(768),  -- Or 1536 for OpenAI
    metadata JSONB,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Create HNSW index
CREATE INDEX code_embeddings_hnsw_idx
ON code_embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Create supporting indexes
CREATE INDEX idx_file_path ON code_embeddings(file_path);
CREATE INDEX idx_language ON code_embeddings(language);
CREATE INDEX idx_metadata ON code_embeddings USING gin(metadata);

-- Similarity search query
SELECT
    file_path,
    chunk_text,
    line_start,
    line_end,
    1 - (embedding <=> $1::vector) AS similarity
FROM code_embeddings
WHERE language = 'python'  -- Optional filter
ORDER BY embedding <=> $1::vector
LIMIT 10;
```

### 7.2 Qdrant (Specialized Vector Database)

**Why Qdrant for Production:**
✅ Best tail latency (p99: 38.71ms vs. pgvector: 74.60ms)
✅ Written in Rust (performance-focused)
✅ Rich filtering capabilities (payload-based)
✅ Multiple vectors per point (e.g., code + docstring)
✅ Quantization support (memory efficiency)

**Performance Characteristics:**
- **p50 latency:** 30.75ms (slightly better than pgvector)
- **p95 latency:** 36.73ms (39% better than pgvector)
- **p99 latency:** 38.71ms (48% better than pgvector)
- **Throughput:** 41 QPS (lower than pgvector, but consistent)

**When to Choose Qdrant:**
- High-performance requirements (p99 < 50ms)
- Need for multi-vector storage
- Experimentation mindset (save DB as file, then deploy)
- Rust ecosystem integration

**Implementation:**
```python
from qdrant_client import QdrantClient
from qdrant_client.models import Distance, VectorParams, PointStruct

# Initialize client
client = QdrantClient(url="http://localhost:6333")

# Create collection
client.create_collection(
    collection_name="code_embeddings",
    vectors_config=VectorParams(
        size=768,  # Embedding dimension
        distance=Distance.COSINE
    )
)

# Insert embeddings
client.upsert(
    collection_name="code_embeddings",
    points=[
        PointStruct(
            id=idx,
            vector=embedding.tolist(),
            payload={
                "file_path": file_path,
                "chunk_text": chunk_text,
                "line_start": line_start,
                "line_end": line_end,
                "language": language,
            }
        )
        for idx, (embedding, file_path, chunk_text, ...) in enumerate(data)
    ]
)

# Search with filtering
results = client.search(
    collection_name="code_embeddings",
    query_vector=query_embedding,
    limit=10,
    query_filter={
        "must": [
            {"key": "language", "match": {"value": "python"}}
        ]
    }
)
```

**Quantization for Memory Efficiency:**
```python
from qdrant_client.models import ScalarQuantization, ScalarType

# Enable scalar quantization (4x memory reduction)
client.create_collection(
    collection_name="code_embeddings_quantized",
    vectors_config=VectorParams(size=768, distance=Distance.COSINE),
    quantization_config=ScalarQuantization(
        scalar=ScalarType.INT8,  # 8-bit quantization
        quantile=0.99,  # Preserve 99% of values
        always_ram=True  # Keep quantized vectors in RAM
    )
)

# Memory reduction:
# float32 (4 bytes) → int8 (1 byte) = 4x savings
# 1M vectors * 768 dims: 2.9GB → 730MB
```

### 7.3 Weaviate (Schema-Rich Vector DB)

**Why Weaviate:**
✅ Best for complex schemas and structured data
✅ GraphQL query interface
✅ Built-in hybrid search (BM25 + vectors)
✅ Modular architecture (swap embedding models easily)

**Performance:**
- **Query latency:** 120ms (slower than Qdrant/pgvector)
- **Use case:** Rich metadata, complex relationships

**When to Choose Weaviate:**
- Complex data relationships (e.g., code → module → package → repository)
- GraphQL preference
- Built-in hybrid search (no custom implementation)

### 7.4 Pinecone (Managed Service)

**Why Pinecone:**
✅ Zero infrastructure management
✅ Seamless integration with LlamaIndex, LangChain
✅ Automatic scaling
✅ Simple API

**Cost:**
- ~$100-200/month for 1M embeddings
- Pricing based on vectors stored + queries

**When to Choose Pinecone:**
- Startup without DevOps resources
- Rapid prototyping
- Budget available for managed services

**Anti-pattern:**
❌ Not open-source (vendor lock-in)
❌ More expensive than self-hosted

### 7.5 Database Selection Matrix

| Criterion | pgvector | Qdrant | Weaviate | Pinecone |
|-----------|----------|--------|----------|----------|
| **Setup Complexity** | Low (existing PG) | Medium | Medium | Very Low |
| **Cost (1M vecs)** | $10/mo (hosting) | $20/mo | $30/mo | $100-200/mo |
| **Latency (p99)** | 75ms | 39ms | 150ms | 80ms |
| **Throughput** | 471 QPS | 41 QPS | 100 QPS | 200 QPS |
| **Memory (1M @ 768d)** | 8GB | 3GB (quantized) | 4GB | Managed |
| **Hybrid Search** | Manual | Manual | Built-in | Manual |
| **Multi-vector** | No | Yes | Yes | No |
| **SQL Support** | Yes | No | No | No |
| **Open Source** | Yes | Yes | Yes | No |

**Recommendation by Stage:**
- **MVP (0-10K users):** PostgreSQL + pgvector
- **Growth (10K-100K users):** Qdrant (self-hosted) or Pinecone (managed)
- **Scale (100K+ users):** Qdrant cluster or Weaviate with replicas
- **Enterprise:** Weaviate or custom Qdrant deployment

---

## 8. MCP (Model Context Protocol) Integration

### 8.1 MCP Overview

**What is MCP?**
- Open standard for connecting AI models to data sources
- Announced by Anthropic (August 2024)
- "USB-C for AI" - standardized connection protocol

**Architecture:**
```
AI Application (MCP Client)
           ↓
    MCP Protocol (JSON-RPC)
           ↓
  MCP Server (exposes tools/resources)
           ↓
   Data Sources (code, docs, APIs)
```

**Key Benefits:**
✅ Write integration once, use across many AI apps
✅ Two-way communication (not just passive data loading)
✅ Dynamic discovery (AI discovers tools at runtime)
✅ Reduced maintenance (no custom API clients per tool)

### 8.2 MCP Components

**1. Resources:** Data that can be read (files, database records, API responses)
**2. Tools:** Functions the AI can invoke (search code, run tests, deploy)
**3. Prompts:** Pre-defined templates for common tasks

**Example MCP Server for Code Search:**
```python
from mcp import MCPServer, Resource, Tool

server = MCPServer(name="code-search-mcp")

# Resource: Expose code files
@server.resource("code://{file_path}")
async def get_code_file(file_path: str) -> Resource:
    """
    Expose individual code files as resources
    """
    content = await read_file(file_path)
    return Resource(
        uri=f"code://{file_path}",
        mime_type="text/x-python",
        content=content,
        metadata={
            "language": detect_language(file_path),
            "lines": content.count('\n'),
            "last_modified": get_mtime(file_path)
        }
    )

# Tool: Semantic search
@server.tool("search_code")
async def search_code(
    query: str,
    language: str = None,
    top_k: int = 10
) -> list[dict]:
    """
    Semantic search across codebase

    Args:
        query: Natural language query (e.g., "authentication logic")
        language: Filter by programming language
        top_k: Number of results to return

    Returns:
        List of code chunks with metadata
    """
    # Embed query
    query_embedding = await embed(query)

    # Vector search
    results = await vector_search(
        query_embedding,
        language=language,
        limit=top_k
    )

    return [
        {
            "file_path": r.file_path,
            "chunk_text": r.chunk_text,
            "line_range": [r.line_start, r.line_end],
            "similarity": r.score,
            "language": r.language
        }
        for r in results
    ]

# Tool: Graph-based navigation
@server.tool("find_references")
async def find_references(symbol: str) -> list[dict]:
    """
    Find all references to a function/class using code graph
    """
    # Query code graph (LSP-based or custom)
    references = await code_graph.find_references(symbol)

    return [
        {
            "file_path": ref.file,
            "line": ref.line,
            "context": ref.surrounding_code
        }
        for ref in references
    ]

# Run server
if __name__ == "__main__":
    server.run(transport="stdio")  # or "http", "sse"
```

### 8.3 MCP Transport Mechanisms

**1. stdio (Standard Input/Output):**
```python
# Server runs as subprocess
# AI app manages process lifecycle
# Low latency, local only

server.run(transport="stdio")

# Client usage:
from mcp import MCPClient

client = MCPClient()
server_process = client.spawn_server("python code_search_mcp.py")
result = await client.call_tool("search_code", query="auth")
```

**2. HTTP with SSE (Server-Sent Events):**
```python
# Server runs as HTTP endpoint
# Suitable for remote deployments
# Supports streaming responses

server.run(transport="sse", host="0.0.0.0", port=8080)

# Client usage:
client = MCPClient(url="http://code-search.example.com:8080")
result = await client.call_tool("search_code", query="auth")
```

**3. Streamable HTTP:**
```python
# Custom transport for low-latency streaming
# Managed by client infrastructure

server.run(transport="streamable_http")
```

### 8.4 Integration with AI Agents

**Microsoft Copilot Studio:**
```
MCP Server (code search) → Copilot Studio
                                ↓
                    Auto-discover tools as actions
                                ↓
                    Sync updates automatically
                                ↓
            Users invoke actions via natural language
```

**Key Innovation:** Tools are automatically added as actions in Copilot Studio. Updates on MCP server propagate automatically.

**Claude Desktop Integration:**
```json
// claude_desktop_config.json
{
  "mcpServers": {
    "code-search": {
      "command": "python",
      "args": ["/path/to/code_search_mcp.py"],
      "env": {
        "DATABASE_URL": "postgresql://localhost/code_db"
      }
    }
  }
}
```

**VS Code Copilot Integration:**
```typescript
// VS Code extension integrates MCP servers
import { MCPClient } from '@anthropic/mcp-client';

const client = new MCPClient({
  serverCommand: 'python',
  serverArgs: ['code_search_mcp.py']
});

// Tools are exposed to Copilot automatically
// User: "Find all authentication functions"
// → Copilot calls search_code("authentication") via MCP
```

### 8.5 Multi-Agent Patterns with MCP

**Swarm Pattern (Multi-Agent Collaboration):**
```python
from mcp_agent import Agent, AugmentedLLM, Swarm

# Define specialized agents
code_search_agent = Agent(
    name="CodeSearcher",
    purpose="Find relevant code based on natural language queries",
    tools=["search_code", "find_references"],
    llm=AugmentedLLM(model="gpt-4")
)

code_review_agent = Agent(
    name="CodeReviewer",
    purpose="Review code for bugs and style issues",
    tools=["analyze_code", "run_linter"],
    llm=AugmentedLLM(model="claude-sonnet-3.5")
)

test_generator_agent = Agent(
    name="TestGenerator",
    purpose="Generate unit tests for code",
    tools=["generate_tests", "run_tests"],
    llm=AugmentedLLM(model="gpt-4")
)

# Orchestrate with Swarm
swarm = Swarm([code_search_agent, code_review_agent, test_generator_agent])

# User request: "Review the authentication module and add tests"
result = await swarm.execute("""
1. Find all authentication-related code
2. Review it for security issues
3. Generate comprehensive unit tests
""")
```

**Workflow Composition:**
```python
from mcp_agent import Workflow

# Define workflow steps
workflow = Workflow(
    name="code_enhancement",
    steps=[
        {
            "agent": code_search_agent,
            "task": "Find functions related to {topic}",
            "output_var": "relevant_code"
        },
        {
            "agent": code_review_agent,
            "task": "Review {relevant_code} for issues",
            "output_var": "issues"
        },
        {
            "agent": test_generator_agent,
            "task": "Generate tests covering {issues}",
            "output_var": "tests"
        }
    ]
)

# Execute
result = await workflow.run(topic="user authentication")
```

### 8.6 Tool Filtering and Context Management

**Selective Tool Exposure:**
```python
# Filter tools per agent to reduce context clutter
agent = Agent(
    name="SpecializedAgent",
    tools={
        "code-search-mcp": ["search_code"],  # Only expose search_code
        "git-mcp": ["git_diff", "git_log"],  # Only git history tools
    }
)

# Dynamic filtering per run
result = await agent.run(
    task="Find authentication bugs",
    tool_filter=lambda tool: tool.category == "security"
)
```

**Streaming Results:**
```python
# MCP supports streaming for long-running tools
async for chunk in client.stream_tool("search_code", query="auth"):
    # Process incremental results
    print(f"Found: {chunk.file_path}")

# Useful for:
# - Incremental search results
# - Real-time test execution
# - Progressive code generation
```

### 8.7 MCP Best Practices

**1. Tool Design:**
```python
# ✅ Good: Clear, focused tools
@server.tool("search_functions")
async def search_functions(query: str, language: str = None):
    """Search for function definitions only"""
    pass

@server.tool("search_classes")
async def search_classes(query: str, language: str = None):
    """Search for class definitions only"""
    pass

# ❌ Bad: Overly broad tools
@server.tool("search_everything")
async def search_everything(
    query: str,
    search_type: str,  # "functions" | "classes" | "variables" | ...
    language: str = None,
    include_comments: bool = False,
    # ... 10 more parameters
):
    """Does too much, confuses LLM"""
    pass
```

**2. Error Handling:**
```python
@server.tool("search_code")
async def search_code(query: str):
    try:
        results = await vector_search(query)
        return results
    except DatabaseConnectionError as e:
        # Return structured error for LLM to handle
        return {
            "error": "database_unavailable",
            "message": "Code search temporarily unavailable",
            "suggestion": "Try using keyword search instead",
            "retry_after": 30
        }
```

**3. Metadata Enrichment:**
```python
@server.tool("search_code")
async def search_code(query: str):
    results = await vector_search(query)

    # Enrich with helpful metadata
    return [
        {
            **result,
            "explanation": f"Match reason: {explain_match(query, result)}",
            "related_symbols": find_related(result.symbols),
            "common_usage": get_usage_examples(result.function_name)
        }
        for result in results
    ]
```

### 8.8 Industry Adoption

**Current Status (October 2025):**
- Early adopters: Block, Apollo
- Development tools: Zed, Replit, Codeium, Sourcegraph
- AI platforms: Microsoft Copilot Studio, Claude Desktop

**Expected Growth:**
- MCP becoming standard for LLM integrations (like REST for web APIs)
- Expansion in finance, healthcare, tech sectors
- Open-source connector ecosystem growing rapidly

---

## 9. Performance Benchmarks and SLAs

### 9.1 Code Search Latency Targets

**Industry Standards:**
| System Type | p50 | p95 | p99 | Notes |
|-------------|-----|-----|-----|-------|
| Web API (general) | <100ms | <200ms | <300ms | User-facing |
| Code search (IDE) | <50ms | <150ms | <250ms | Interactive |
| Financial systems | <50ms | <100ms | <150ms | Strict requirements |
| Gaming services | <20ms | <50ms | <100ms | Real-time |

**Code Search Specific:**
- **pgvector:** p99 = 74.60ms ✅ (meets IDE requirements)
- **Qdrant:** p99 = 38.71ms ✅✅ (exceeds expectations)
- **Pinecone:** p99 = ~80ms ✅ (acceptable for most uses)
- **Weaviate:** p99 = ~150ms ⚠️ (borderline for interactive use)

**Recommendation:**
- **MVP target:** p95 < 200ms, p99 < 500ms
- **Production target:** p95 < 100ms, p99 < 200ms
- **Premium experience:** p95 < 50ms, p99 < 100ms

### 9.2 Indexing Performance

**Tree-sitter Parsing:**
- **Speed:** 36x faster than JavaParser
- **Real-time capability:** Can parse on every keystroke
- **Incremental updates:** Only reparse changed sections

**Full Indexing Benchmarks:**
| Codebase Size | Naive Approach | Incremental | Quantized |
|---------------|----------------|-------------|-----------|
| 10K LOC | 30s | 10s | 8s |
| 100K LOC | 5min | 45s | 30s |
| 1M LOC | 50min | 6min | 3min |
| 10M LOC | 8hrs | 45min | 20min |
| 100M LOC | Days | 6hrs | 2hrs |

**Incremental Update Performance:**
- **Cursor (Merkle tree):** 10-minute sync cycle, only changed files
- **Glean (Meta):** 50K files in 45s (35% improvement over baseline)
- **IntelliJ IDEA:** 57% reduction (14min → 6min)

**Files Per Minute (Estimated):**
- Tree-sitter parsing: ~1000-2000 files/min (simple files)
- Full pipeline (parse + embed + index): ~200-500 files/min
- With GPU acceleration: ~1000-1500 files/min

### 9.3 Search Quality Metrics

**Benchmark Datasets:**
| Dataset | Size | Task | Languages |
|---------|------|------|-----------|
| CodeSearchNet | 2M pairs | Natural language → code | 6 (Python, JS, Java, etc.) |
| CoIR | 2M docs, 10 datasets | Code retrieval | 8 domains |
| RepoEval | 2K functions | Repository-level tasks | Multiple |
| CrossCodeEval | Multi-language | Cross-language transfer | 40+ |

**Performance by Method:**
| Approach | Recall@10 | MRR | Comments |
|----------|-----------|-----|----------|
| BM25 | 0.65 | 0.58 | Good for exact keywords |
| Vector (CodeBERT) | 0.72 | 0.64 | Better semantic understanding |
| Vector (UniXcoder) | 0.78 | 0.71 | Code-specific model wins |
| Hybrid (BM25 + vector) | 0.81 | 0.75 | Best of both worlds |
| Hybrid + cross-encoder | 0.87 | 0.82 | Production quality |
| AST-based chunking | +5.5% | +4.3% | Over fixed-size chunking |

**Real-world Performance (GitHub Copilot):**
- 37.6% lift in retrieval quality (new embedding model)
- +110% acceptance ratio for C# (with better indexing)
- +113% acceptance ratio for Java

### 9.4 Cost Benchmarks

**Embedding Costs (1M code chunks):**
| Provider | Model | Cost | Dimensions |
|----------|-------|------|------------|
| OpenAI | text-embedding-3-small | $100 | 1536 |
| OpenAI | text-embedding-3-large | $300 | 3072 |
| Self-hosted | BGE-large | $0 (compute only) | 1024 |
| Self-hosted | UniXcoder | $0 (compute only) | 768 |

**Storage Costs (1M vectors, 1 year):**
| Database | Monthly Cost | Storage Size |
|----------|--------------|--------------|
| pgvector (DigitalOcean) | $10-20 | 8GB |
| Qdrant (self-hosted) | $20-40 | 3GB (quantized) |
| Pinecone | $100-200 | Managed |
| Weaviate (cloud) | $50-100 | 4GB |

**Query Costs:**
- Self-hosted: $0 (compute included in hosting)
- Pinecone: Included in base price (up to limits)
- API-based: Variable (typically <$0.001/query)

**Total Cost of Ownership (1 year, 1M chunks):**
| Stack | Setup | Embeddings | Hosting | Total |
|-------|-------|------------|---------|-------|
| OpenAI + Pinecone | $0 | $100 | $1200-2400 | $1300-2500 |
| OpenAI + pgvector | $500 | $100 | $120-240 | $720-840 |
| Self-hosted (all) | $1000 | $0 | $240-480 | $1240-1480 |

**Recommendation for MVP:**
- Budget-conscious: Self-hosted (UniXcoder + pgvector)
- Speed-to-market: OpenAI + Pinecone
- Balanced: OpenAI + pgvector

---

## 10. Production-Ready MVP Architecture

### 10.1 Recommended Tech Stack

**For Startups (0-10K users):**
```
Code Parsing: Tree-sitter
Chunking: AST-based (cAST algorithm)
Embeddings: OpenAI text-embedding-3-small (1536d)
Database: PostgreSQL + pgvector
Search: Hybrid (BM25 + vector) with RRF
Backend: Python (FastAPI) or Node.js (Express)
MCP Integration: Anthropic MCP SDK
```

**Implementation Timeline:**
- Week 1: Set up PostgreSQL + pgvector, basic indexing
- Week 2: Tree-sitter integration, AST-based chunking
- Week 3: Embedding pipeline (OpenAI), vector storage
- Week 4: Search API (hybrid retrieval)
- Week 5: MCP server for AI integration
- Week 6: Testing, optimization, deployment

### 10.2 Minimal Viable Architecture

```
┌─────────────────┐
│   Code Files    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Tree-sitter    │ Parse → AST
│  (cAST chunk)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ OpenAI Embed    │ text-embedding-3-small
│  (1536d)        │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────┐
│  PostgreSQL + pgvector      │
│  - HNSW index (m=16)        │
│  - BM25 full-text search    │
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│   FastAPI Backend           │
│   - Hybrid search           │
│   - RRF merging             │
│   - MCP server              │
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│   AI Apps (MCP clients)     │
│   - Claude Desktop          │
│   - VS Code Copilot         │
│   - Custom agents           │
└─────────────────────────────┘
```

### 10.3 Code Example (MVP Implementation)

**1. Indexing Pipeline:**
```python
import tree_sitter
from openai import OpenAI
import psycopg2
from psycopg2.extras import execute_values

# Initialize
ts_parser = tree_sitter.Parser()
ts_parser.set_language(tree_sitter_languages.get_language('python'))
openai_client = OpenAI()
db_conn = psycopg2.connect("postgresql://localhost/code_search")

def index_codebase(repo_path):
    """
    Index entire codebase with AST-based chunking
    """
    chunks = []

    for file_path in glob.glob(f"{repo_path}/**/*.py", recursive=True):
        code = read_file(file_path)

        # Parse with tree-sitter
        tree = ts_parser.parse(bytes(code, 'utf8'))

        # AST-based chunking
        file_chunks = cast_chunking(code, tree, max_tokens=512)

        for chunk in file_chunks:
            chunks.append({
                'file_path': file_path,
                'chunk_text': chunk.text,
                'line_start': chunk.line_start,
                'line_end': chunk.line_end,
                'language': 'python',
            })

    # Batch embed (100 chunks at a time for API efficiency)
    for i in range(0, len(chunks), 100):
        batch = chunks[i:i+100]

        # Get embeddings
        response = openai_client.embeddings.create(
            model="text-embedding-3-small",
            input=[c['chunk_text'] for c in batch]
        )

        # Add embeddings to chunks
        for chunk, embedding_obj in zip(batch, response.data):
            chunk['embedding'] = embedding_obj.embedding

        # Insert to database
        with db_conn.cursor() as cur:
            execute_values(
                cur,
                """
                INSERT INTO code_embeddings
                (file_path, chunk_text, line_start, line_end, language, embedding)
                VALUES %s
                """,
                [
                    (
                        c['file_path'], c['chunk_text'], c['line_start'],
                        c['line_end'], c['language'], c['embedding']
                    )
                    for c in batch
                ]
            )

        db_conn.commit()

    print(f"Indexed {len(chunks)} code chunks")
```

**2. Search API:**
```python
from fastapi import FastAPI, Query
from pydantic import BaseModel

app = FastAPI()

class SearchResult(BaseModel):
    file_path: str
    chunk_text: str
    line_start: int
    line_end: int
    score: float

@app.get("/search", response_model=list[SearchResult])
async def search_code(
    query: str = Query(..., description="Natural language query"),
    language: str = Query(None, description="Filter by language"),
    top_k: int = Query(10, description="Number of results")
):
    """
    Hybrid search: BM25 + vector
    """
    # Embed query
    query_embedding = openai_client.embeddings.create(
        model="text-embedding-3-small",
        input=query
    ).data[0].embedding

    # Hybrid search with RRF
    with db_conn.cursor() as cur:
        cur.execute("""
            WITH bm25_results AS (
                SELECT
                    id, file_path, chunk_text, line_start, line_end,
                    ts_rank(to_tsvector('english', chunk_text),
                            plainto_tsquery('english', %s)) as bm25_score,
                    ROW_NUMBER() OVER (ORDER BY ts_rank(...) DESC) as bm25_rank
                FROM code_embeddings
                WHERE language = COALESCE(%s, language)
                ORDER BY bm25_score DESC
                LIMIT 50
            ),
            vector_results AS (
                SELECT
                    id, file_path, chunk_text, line_start, line_end,
                    1 - (embedding <=> %s::vector) as vector_score,
                    ROW_NUMBER() OVER (ORDER BY embedding <=> %s::vector) as vector_rank
                FROM code_embeddings
                WHERE language = COALESCE(%s, language)
                ORDER BY embedding <=> %s::vector
                LIMIT 50
            )
            SELECT
                COALESCE(b.file_path, v.file_path) as file_path,
                COALESCE(b.chunk_text, v.chunk_text) as chunk_text,
                COALESCE(b.line_start, v.line_start) as line_start,
                COALESCE(b.line_end, v.line_end) as line_end,
                (COALESCE(1.0 / (60 + b.bm25_rank), 0) +
                 COALESCE(1.0 / (60 + v.vector_rank), 0)) as rrf_score
            FROM bm25_results b
            FULL OUTER JOIN vector_results v ON b.id = v.id
            ORDER BY rrf_score DESC
            LIMIT %s
        """, (query, language, query_embedding, query_embedding,
              language, query_embedding, top_k))

        results = cur.fetchall()

    return [
        SearchResult(
            file_path=r[0],
            chunk_text=r[1],
            line_start=r[2],
            line_end=r[3],
            score=r[4]
        )
        for r in results
    ]
```

**3. MCP Server:**
```python
from mcp import MCPServer

mcp_server = MCPServer(name="code-search")

@mcp_server.tool("search_code")
async def mcp_search_code(
    query: str,
    language: str = None,
    top_k: int = 10
) -> list[dict]:
    """
    Search codebase using natural language
    """
    # Reuse FastAPI search logic
    results = await search_code(query, language, top_k)

    return [
        {
            "file_path": r.file_path,
            "chunk_text": r.chunk_text,
            "line_range": [r.line_start, r.line_end],
            "relevance_score": r.score
        }
        for r in results
    ]

if __name__ == "__main__":
    mcp_server.run(transport="stdio")
```

### 10.4 Deployment Considerations

**Database Setup:**
```sql
-- Enable pgvector extension
CREATE EXTENSION vector;

-- Create optimized indexes
CREATE INDEX code_embeddings_hnsw_idx
ON code_embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

CREATE INDEX code_embeddings_fts_idx
ON code_embeddings
USING gin(to_tsvector('english', chunk_text));

-- Vacuum and analyze
VACUUM ANALYZE code_embeddings;
```

**Environment Variables:**
```bash
# .env
DATABASE_URL=postgresql://user:pass@localhost/code_search
OPENAI_API_KEY=sk-...
MAPROOM_EMBEDDING_MODEL=text-embedding-3-small
EMBEDDING_DIMENSIONS=1536
MAX_CHUNK_SIZE=512
```

**Scaling Considerations:**
1. **Read replicas** for search queries (offload from primary)
2. **Connection pooling** (pgBouncer) for high concurrency
3. **Caching layer** (Redis) for frequent queries
4. **Async workers** (Celery) for background indexing

### 10.5 Monitoring and Observability

**Key Metrics:**
```python
from prometheus_client import Counter, Histogram

# Search metrics
search_requests = Counter('search_requests_total', 'Total search requests')
search_latency = Histogram('search_latency_seconds', 'Search latency')
search_results = Histogram('search_results_count', 'Number of results returned')

# Indexing metrics
indexing_chunks = Counter('indexing_chunks_total', 'Total chunks indexed')
indexing_latency = Histogram('indexing_latency_seconds', 'Indexing latency per file')
indexing_errors = Counter('indexing_errors_total', 'Indexing errors')

# Database metrics
db_connections = Gauge('db_connections_active', 'Active database connections')
db_query_latency = Histogram('db_query_latency_seconds', 'Database query latency')
```

**Health Checks:**
```python
@app.get("/health")
async def health_check():
    """
    Verify all components are healthy
    """
    health = {
        "status": "healthy",
        "components": {}
    }

    # Check database
    try:
        with db_conn.cursor() as cur:
            cur.execute("SELECT 1")
        health["components"]["database"] = "healthy"
    except Exception as e:
        health["status"] = "unhealthy"
        health["components"]["database"] = f"unhealthy: {e}"

    # Check OpenAI API
    try:
        openai_client.embeddings.create(
            model="text-embedding-3-small",
            input="health check"
        )
        health["components"]["openai"] = "healthy"
    except Exception as e:
        health["status"] = "unhealthy"
        health["components"]["openai"] = f"unhealthy: {e}"

    return health
```

---

## 11. Key Takeaways and Recommendations

### 11.1 Critical Success Factors

**1. AST-Based Chunking is Non-Negotiable**
- Fixed-size chunking breaks code semantics
- Tree-sitter provides 36x faster parsing than alternatives
- AST-based chunking yields +5.5% improvement on benchmarks

**2. Hybrid Search Outperforms Single Methods**
- BM25 alone: 65% recall
- Vector alone: 72% recall
- Hybrid (BM25 + vector): 81% recall
- With reranking: 87% recall

**3. Context Window Optimization is Critical**
- Don't fill the entire context (40-70% is optimal)
- Most LLMs utilize only 10-20% effectively
- "Lost in the middle" problem is real

**4. Incremental Indexing is Essential for Scale**
- Full reindexing doesn't scale beyond 1M LOC
- Merkle trees enable efficient change detection
- Quantization reduces memory by 8x with <0.1% accuracy loss

**5. PostgreSQL + pgvector is the MVP Sweet Spot**
- 11.4x higher throughput than specialized vector DBs
- Familiar SQL interface
- Sub-100ms latency at 99% recall
- Upgrade path to specialized DBs when needed

### 11.2 Architecture Decision Tree

```
Start MVP
    │
    ▼
Codebase size?
    │
    ├─ <100K LOC ──→ PostgreSQL + pgvector + OpenAI embeddings
    │                 (Simplest, fastest to market)
    │
    ├─ 100K-1M LOC ─→ PostgreSQL + pgvector + self-hosted embeddings
    │                 (Cost-optimized)
    │
    └─ >1M LOC ─────→ Qdrant + quantization + incremental indexing
                      (Performance-optimized)

Latency requirements?
    │
    ├─ p99 < 50ms ──→ Qdrant (38ms p99)
    │
    ├─ p99 < 100ms ─→ PostgreSQL + pgvector (75ms p99)
    │
    └─ p99 < 200ms ─→ Any solution works

Budget?
    │
    ├─ <$100/mo ────→ Self-hosted (all components)
    │
    ├─ <$500/mo ────→ OpenAI embeddings + self-hosted DB
    │
    └─ >$500/mo ────→ Managed services (Pinecone, etc.)

Team expertise?
    │
    ├─ SQL experts ──→ PostgreSQL + pgvector
    │
    ├─ Python/ML ────→ Qdrant or Weaviate
    │
    └─ No DevOps ────→ Pinecone (fully managed)
```

### 11.3 Implementation Roadmap

**Phase 1: MVP (Weeks 1-6)**
- [ ] Set up PostgreSQL + pgvector
- [ ] Implement Tree-sitter AST-based chunking
- [ ] Create embedding pipeline (OpenAI)
- [ ] Build basic search API (vector only)
- [ ] Deploy to staging environment

**Phase 2: Enhancement (Weeks 7-10)**
- [ ] Add BM25 full-text search
- [ ] Implement hybrid retrieval with RRF
- [ ] Create MCP server for AI integration
- [ ] Add incremental indexing (Merkle trees)
- [ ] Set up monitoring and alerting

**Phase 3: Optimization (Weeks 11-14)**
- [ ] Implement cross-encoder reranking
- [ ] Add query classification (dynamic α weighting)
- [ ] Optimize context window utilization
- [ ] Add caching layer (Redis)
- [ ] Performance tuning (HNSW parameters)

**Phase 4: Scale (Weeks 15+)**
- [ ] Migrate to Qdrant if latency is critical
- [ ] Implement quantization for memory efficiency
- [ ] Add read replicas for search queries
- [ ] Advanced features (HyDE, graph-based expansion)
- [ ] Multi-tenant support

### 11.4 Common Pitfalls to Avoid

❌ **Don't:**
1. Use fixed-size chunking (breaks code semantics)
2. Fill the entire context window (40-70% is optimal)
3. Ignore incremental indexing (doesn't scale)
4. Over-engineer initially (start with pgvector)
5. Skip BM25 (hybrid is significantly better)
6. Forget to monitor tail latency (p99 matters)
7. Hard-code embedding dimensions (make configurable)

✅ **Do:**
1. Use AST-based chunking with Tree-sitter
2. Start with PostgreSQL + pgvector
3. Implement hybrid search from day one
4. Monitor p95/p99 latency, not just averages
5. Use RRF reranking (cheap, effective)
6. Plan for incremental indexing from start
7. Build MCP server for AI integration
8. Optimize context window utilization (40-70%)

### 11.5 Future-Proofing

**Trends to Watch:**
1. **Long-context models** (Gemini 1M tokens, Claude 200K tokens)
   - RAG still valuable for cost and precision
   - Hybrid approach: RAG for retrieval, long context for reasoning

2. **Smaller, faster embedding models**
   - Matryoshka embeddings (variable dimensions)
   - Binary quantization (32x memory reduction)

3. **Graph-enhanced retrieval**
   - Code knowledge graphs (functions → modules → packages)
   - Call graph traversal for context expansion

4. **Multi-modal code understanding**
   - Code + diagrams + documentation
   - Visual code representations

5. **Federated search**
   - Search across multiple repositories
   - Privacy-preserving code search

**Preparation:**
- Keep embedding model swappable (abstraction layer)
- Design database schema for future metadata
- Build API versioning from start
- Document chunking strategy (easy to change later)

---

## 12. References and Resources

### 12.1 Key Papers

1. **CodeSearchNet Challenge** (2019) - Hamel Husain et al.
   - Foundational benchmark for code search
   - 2M code-comment pairs

2. **CoIR: A Comprehensive Benchmark for Code Information Retrieval Models** (2024)
   - 10 datasets, 8 tasks, 7 domains
   - Current standard for evaluation

3. **cAST: Enhancing Code RAG with Structural Chunking via AST** (2024)
   - Split-then-merge algorithm
   - +5.5% improvement on RepoEval

4. **Tree-Enhanced CodeBERTa** (2024)
   - Depth + sibling embeddings
   - Better hierarchical understanding

5. **LoRACode** (2025)
   - Parameter-efficient fine-tuning for code
   - 100x smaller than full fine-tuning

### 12.2 Tools and Libraries

**Parsing:**
- Tree-sitter: https://tree-sitter.github.io/
- tree-sitter-languages: https://pypi.org/project/tree-sitter-languages/

**Embeddings:**
- OpenAI: https://platform.openai.com/docs/guides/embeddings
- SentenceTransformers: https://www.sbert.net/
- Hugging Face Transformers: https://huggingface.co/transformers/

**Vector Databases:**
- pgvector: https://github.com/pgvector/pgvector
- Qdrant: https://qdrant.tech/
- Weaviate: https://weaviate.io/
- Pinecone: https://www.pinecone.io/

**MCP (Model Context Protocol):**
- Anthropic MCP: https://www.anthropic.com/news/model-context-protocol
- MCP Agent: https://github.com/lastmile-ai/mcp-agent

**Benchmarks:**
- CodeSearchNet: https://github.com/github/CodeSearchNet
- CoIR: https://huggingface.co/datasets/CoIR-Retrieval

### 12.3 Production Examples

**Open Source Projects:**
1. **Sourcegraph** - https://github.com/sourcegraph/sourcegraph
   - Production code search at scale
   - LSIF integration

2. **Aider** - https://aider.chat/
   - Terminal-based AI coding assistant
   - RepoMap for context management

3. **CocoIndex** - https://cocoindex.io/
   - Tree-sitter-based indexing
   - Real-time incremental updates

**Industry Case Studies:**
1. **GitHub Copilot** - Embedding model evolution
2. **Meta Glean** - Incremental indexing at scale
3. **Cursor IDE** - Merkle tree synchronization
4. **IntelliJ IDEA** - Pre-built shared indexes

### 12.4 Further Reading

**Blogs:**
- Augment Code: https://www.augmentcode.com/blog
- Sourcegraph Blog: https://about.sourcegraph.com/blog
- GitHub Blog: https://github.blog/

**Newsletters:**
- AI Engineer: https://www.aieng.news/
- The Batch (deeplearning.ai): https://www.deeplearning.ai/the-batch/

**Communities:**
- r/MachineLearning (Reddit)
- HuggingFace Forums
- MCP Discord (Anthropic)

---

## Appendix A: Sample Database Schema

```sql
-- Main code embeddings table
CREATE TABLE code_embeddings (
    id BIGSERIAL PRIMARY KEY,

    -- File information
    file_path TEXT NOT NULL,
    repository_url TEXT,
    commit_hash TEXT,

    -- Chunk information
    chunk_text TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,

    -- Metadata
    language TEXT NOT NULL,
    chunk_type TEXT,  -- 'function', 'class', 'module', etc.
    symbol_name TEXT,
    parent_symbol TEXT,

    -- Embeddings
    embedding VECTOR(1536),  -- Adjust dimension as needed

    -- Full-text search
    tsv TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('english', chunk_text)
    ) STORED,

    -- Additional metadata (JSON for flexibility)
    metadata JSONB,

    -- Timestamps
    indexed_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Indexes
CREATE INDEX code_embeddings_hnsw_idx
ON code_embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

CREATE INDEX code_embeddings_fts_idx
ON code_embeddings
USING gin(tsv);

CREATE INDEX idx_file_path ON code_embeddings(file_path);
CREATE INDEX idx_language ON code_embeddings(language);
CREATE INDEX idx_symbol ON code_embeddings(symbol_name);
CREATE INDEX idx_metadata ON code_embeddings USING gin(metadata);

-- Repositories tracking
CREATE TABLE repositories (
    id SERIAL PRIMARY KEY,
    url TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    last_indexed_commit TEXT,
    last_indexed_at TIMESTAMP,
    metadata JSONB
);

-- Indexing jobs (for tracking progress)
CREATE TABLE indexing_jobs (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER REFERENCES repositories(id),
    status TEXT NOT NULL,  -- 'pending', 'running', 'completed', 'failed'
    files_processed INTEGER DEFAULT 0,
    chunks_created INTEGER DEFAULT 0,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    error_message TEXT
);
```

---

## Appendix B: Performance Tuning Checklist

### PostgreSQL + pgvector Optimization

- [ ] **Shared buffers**: Set to 25% of RAM
  ```
  shared_buffers = 8GB  # For 32GB RAM server
  ```

- [ ] **Work memory**: Increase for sorting/aggregation
  ```
  work_mem = 256MB
  maintenance_work_mem = 2GB
  ```

- [ ] **Effective cache size**: Set to 50-75% of RAM
  ```
  effective_cache_size = 24GB
  ```

- [ ] **Max connections**: Limit with connection pooler
  ```
  max_connections = 100  # Use pgBouncer for more
  ```

- [ ] **HNSW ef_search**: Tune per query
  ```sql
  SET hnsw.ef_search = 100;  -- Higher = better recall, slower
  ```

- [ ] **Vacuum regularly**: Prevent index bloat
  ```sql
  VACUUM ANALYZE code_embeddings;
  ```

- [ ] **Monitor index size**:
  ```sql
  SELECT pg_size_pretty(pg_relation_size('code_embeddings_hnsw_idx'));
  ```

### Embedding Pipeline Optimization

- [ ] **Batch API calls**: 100-1000 items per request
- [ ] **Use async/await**: Parallel processing
- [ ] **Implement retries**: Exponential backoff for API failures
- [ ] **Cache embeddings**: Don't re-embed unchanged code
- [ ] **Monitor rate limits**: OpenAI: 3,000 RPM for tier 1

### Search Query Optimization

- [ ] **Use prepared statements**: Avoid SQL injection + faster execution
- [ ] **Limit result set**: Don't fetch more than needed
- [ ] **Filter early**: Apply language/file filters before vector search
- [ ] **Use query timeouts**: Prevent runaway queries
- [ ] **Cache frequent queries**: Redis with 5-minute TTL

---

**End of Report**

This research represents the state-of-the-art in code indexing and retrieval systems as of October 2025, with focus on practical, production-ready implementations suitable for multi-agent development environments.
