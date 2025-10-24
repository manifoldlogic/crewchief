# Weight Tuning Guide for Maproom Hybrid Search

## Overview

Maproom's hybrid search combines multiple ranking signals into a unified relevance score using weighted linear combination. This guide explains how to tune these weights for optimal search quality in your codebase.

## Understanding Fusion Weights

### The Five Signals

Maproom combines five distinct search signals:

1. **FTS (Full-Text Search)** - Keyword matching using PostgreSQL's ts_rank_cd
   - Best for: Exact term matches, identifier searches, API names
   - Normalized score: 0.0-1.0
   - Higher = better lexical match

2. **Vector (Semantic Similarity)** - Embedding-based similarity using pgvector
   - Best for: Conceptual searches, natural language queries
   - Normalized score: 0.0-1.0 (cosine similarity distance)
   - Higher = more semantically similar

3. **Graph (Code Importance)** - PageRank-style centrality from code relationships
   - Best for: Finding central/important code, entry points
   - Normalized score: 0.0-1.0
   - Higher = more central/referenced

4. **Recency** - How recently the code was modified
   - Best for: Finding actively maintained code
   - Normalized score: 0.0-1.0
   - Higher = more recently changed

5. **Churn** - How frequently the code changes (inverted during fusion)
   - Best for: Avoiding unstable/frequently changed code
   - Raw churn score: 0.0+ (unbounded, number of changes)
   - Applied as: `weight * (1.0 / (1.0 + churn_score))`
   - Note: Churn is **inverted** - high churn gets low contribution

### Weighted Linear Combination

The final score is calculated as:

```
final_score = w_fts * fts_score
            + w_vector * vector_score
            + w_graph * graph_score
            + w_recency * recency_score
            + w_churn * (1.0 / (1.0 + churn_score))
```

Where all weights should sum to 1.0 for proper normalization.

## Default Weights

Maproom ships with battle-tested defaults:

```rust
FusionWeights {
    fts: 0.4,      // 40% - Keyword matches are important
    vector: 0.35,  // 35% - Semantic similarity is strong signal
    graph: 0.1,    // 10% - Code importance provides context
    recency: 0.1,  // 10% - Recent changes matter
    churn: 0.05,   // 5%  - Slight penalty for unstable code
}
```

### Rationale

- **FTS gets the highest weight** because exact keyword matches are highly relevant
- **Vector is close second** for semantic understanding beyond keywords
- **Graph provides modest boost** to central/important code
- **Recency helps** surface actively maintained code
- **Churn has lowest weight** as a gentle quality signal

## Tuning for Different Use Cases

### Finding API Implementations

Prioritize FTS for exact identifier matches:

```rust
FusionWeights::new(
    0.6,   // fts - Heavily favor keyword matches
    0.25,  // vector - Some semantic understanding
    0.1,   // graph - API entry points are central
    0.05,  // recency - Recent APIs slightly preferred
    0.0,   // churn - Don't penalize frequently updated APIs
)
```

**Example Query:** `"authenticate"`, `"parseResponse"`

### Conceptual/Natural Language Search

Prioritize vector similarity:

```rust
FusionWeights::new(
    0.2,   // fts - Keywords still matter somewhat
    0.6,   // vector - Heavy emphasis on semantic similarity
    0.1,   // graph - Important code is relevant
    0.08,  // recency - Prefer recent patterns
    0.02,  // churn - Light stability signal
)
```

**Example Query:** `"how to handle user authentication"`, `"database connection setup"`

### Finding Core/Important Code

Prioritize graph centrality:

```rust
FusionWeights::new(
    0.25,  // fts - Match terms reasonably
    0.3,   // vector - Understand semantics
    0.35,  // graph - Heavy weight on centrality
    0.05,  // recency - Core code changes slowly
    0.05,  // churn - Stable core is better
)
```

**Example Query:** `"main entry point"`, `"core business logic"`

### Finding Recently Modified Code

Prioritize recency:

```rust
FusionWeights::new(
    0.3,   // fts - Match the terms
    0.25,  // vector - Understand context
    0.05,  // graph - Centrality less important
    0.35,  // recency - Heavy weight on recent changes
    0.05,  // churn - But not TOO churny
)
```

**Example Query:** `"new features"`, `"recent updates"`

### Finding Stable/Production Code

Penalize churn more heavily:

```rust
FusionWeights::new(
    0.35,  // fts - Find the right code
    0.3,   // vector - Understand it semantically
    0.15,  // graph - Stable code is often central
    0.0,   // recency - Don't prefer recent changes
    0.2,   // churn - Strong penalty for instability
)
```

**Example Query:** `"stable API"`, `"production configuration"`

## Weight Configuration

### Programmatic Configuration

```rust
use crewchief_maproom::search::FusionWeights;

// Create custom weights
let weights = FusionWeights::new(0.4, 0.35, 0.1, 0.1, 0.05);

// Validate weights are non-negative
weights.validate()?;

// Normalize to sum to 1.0
let normalized = weights.normalized();

// Or normalize in-place
let mut weights = FusionWeights::new(0.8, 0.7, 0.3, 0.2, 0.0);
weights.normalize();
assert!(weights.is_normalized());
```

### YAML Configuration (Future)

```yaml
# maproom-search.yml
fusion:
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
    churn: 0.05
```

## Debugging Score Contributions

### Using ScoreBreakdown

When debugging, enable score breakdown to understand how each signal contributed:

```rust
// FusedResult includes optional breakdown
if let Some(breakdown) = &result.breakdown {
    println!("Score breakdown: {}", breakdown.format_debug());
    // Output: "FTS:0.240 Vec:0.210 Graph:0.060 Recency:0.050 Churn:0.048"

    // Get percentage contributions
    for (signal, pct) in breakdown.as_percentages() {
        println!("{}: {:.1}%", signal, pct);
    }
    // Output:
    // FTS: 39.5%
    // Vector: 34.5%
    // Graph: 9.9%
    // Recency: 8.2%
    // Churn: 7.9%
}
```

### Debug Output Format

Score breakdown shows weighted contributions:

```
FTS:0.320 Vec:0.280 Graph:0.080 Recency:0.100 Churn:0.045
```

Each value shows the actual contribution to the final score (weight * signal_score).

## Tuning Methodology

### 1. Start with Defaults

Use the default weights and run test queries:

```rust
let weights = FusionWeights::default();
```

### 2. Identify Issues

Look for:
- **Poor FTS results?** Increase FTS weight, decrease vector
- **Missing conceptual matches?** Increase vector weight
- **Too much noise?** Increase graph weight (favors central code)
- **Stale results?** Increase recency weight
- **Unstable code at top?** Increase churn weight

### 3. Adjust Incrementally

Change weights by 0.05-0.1 at a time:

```rust
// If semantic search is weak, shift from FTS to vector
let weights = FusionWeights::new(
    0.35,  // fts (was 0.4)
    0.45,  // vector (was 0.35)
    0.1,   // graph (unchanged)
    0.08,  // recency (adjust to maintain sum=1.0)
    0.02,  // churn (adjust to maintain sum=1.0)
);
```

### 4. Test and Iterate

Run queries and examine:
- Top 10 results relevance
- Score breakdown percentages
- Query latency (weights don't affect performance, but verify)

### 5. Normalize and Validate

Always normalize final weights:

```rust
let mut weights = FusionWeights::new(0.5, 0.4, 0.2, 0.15, 0.1);
weights.validate()?; // Check non-negative
weights.normalize(); // Ensure sum = 1.0
```

## Common Patterns

### Keyword-Heavy Search (Traditional Search Engines)

```rust
FusionWeights::new(0.7, 0.2, 0.05, 0.05, 0.0)
```

### Semantic-Heavy Search (AI/LLM Style)

```rust
FusionWeights::new(0.15, 0.7, 0.1, 0.05, 0.0)
```

### Balanced Hybrid (Recommended Starting Point)

```rust
FusionWeights::default() // 0.4, 0.35, 0.1, 0.1, 0.05
```

### Code Quality Focus

```rust
FusionWeights::new(0.3, 0.3, 0.2, 0.05, 0.15)
```

## Performance Considerations

### Weight Tuning Does NOT Affect Query Performance

Changing weights only affects score calculation, which is O(n) where n = number of unique chunks found. This is already a small fraction of the search time.

### What DOES Affect Performance

- **Database indexes** (GIN for FTS, ivfflat/HNSW for vectors)
- **Index parameters** (ivfflat lists, HNSW m/ef_construction)
- **Query complexity** (number of terms, embedding size)
- **Result limit** (k value)

### Optimal Weight Normalization

Weights should sum to 1.0 for several reasons:

1. **Predictable score range** - Final scores stay in 0.0-1.0 range
2. **Fair comparison** - Different weight sets are comparable
3. **Explainability** - Weights represent percentage contribution

Use `weights.normalize()` to ensure this property.

## Validation and Safety

### Built-in Validation

```rust
let weights = FusionWeights::new(-0.1, 0.5, 0.3, 0.2, 0.1);
weights.validate()?; // ERROR: FTS weight must be non-negative
```

### Recommended Checks

```rust
let mut weights = custom_weights_from_config()?;

// 1. Validate non-negative
weights.validate()?;

// 2. Check if normalized
if !weights.is_normalized() {
    println!("Warning: weights sum to {}, normalizing", weights.sum());
    weights.normalize();
}

// 3. Sanity check ranges
if weights.fts < 0.1 && weights.vector < 0.1 {
    println!("Warning: Both FTS and vector weights are very low");
}
```

## Advanced Topics

### Churn Score Inversion

Churn is unique in that **high churn is bad**. The fusion applies inversion:

```rust
churn_contrib = weight * (1.0 / (1.0 + churn_score))
```

Examples:
- `churn_score = 0` → contribution = `weight * 1.0` (maximum)
- `churn_score = 1` → contribution = `weight * 0.5` (half)
- `churn_score = 9` → contribution = `weight * 0.1` (minimal)

Higher churn means lower contribution, penalizing unstable code.

### Signal Absence Handling

If a signal is not available for a chunk:
- Score for that signal is 0.0
- Weight effectively shifts to other signals
- This is why normalized weights are important

Example: Chunk found only by FTS and vector:
```
final_score = 0.4 * fts_score + 0.35 * vector_score + 0 + 0 + 0
```

### Future: Dynamic Weight Adjustment

Planned features:
- **Query-type detection** - Automatically adjust weights based on query
- **User feedback learning** - Learn optimal weights from click-through data
- **Per-language weights** - Different weights for Rust vs JavaScript, etc.
- **Time-of-day weights** - Boost recency during active development

## Troubleshooting

### Problem: Results dominated by one signal

**Symptoms:** All results have high FTS score but poor semantic matches

**Solution:** Reduce dominant signal weight, increase others:

```rust
// Before
FusionWeights::new(0.8, 0.1, 0.05, 0.05, 0.0)

// After
FusionWeights::new(0.5, 0.35, 0.1, 0.05, 0.0)
```

### Problem: Unstable/frequently changed code ranks too high

**Symptoms:** Top results are files with high churn

**Solution:** Increase churn weight:

```rust
FusionWeights::new(0.4, 0.3, 0.1, 0.08, 0.12)
```

### Problem: Old/stale code ranks too high

**Symptoms:** Top results haven't been updated recently

**Solution:** Increase recency weight:

```rust
FusionWeights::new(0.35, 0.3, 0.08, 0.22, 0.05)
```

### Problem: Results missing important core code

**Symptoms:** Central classes/functions don't appear

**Solution:** Increase graph weight:

```rust
FusionWeights::new(0.3, 0.3, 0.25, 0.1, 0.05)
```

## References

- [Hybrid Search Architecture](./ARCHITECTURE.md) - System design overview
- [Performance Tuning Guide](./PERFORMANCE.md) - Query optimization
- [Schema Documentation](./SCHEMA.md) - Database structure
- [Search Pipeline](../src/search/pipeline.rs) - Implementation details

## Examples Repository

See `tests/fusion_integration_test.rs` for working examples of:
- Custom weight configurations
- Score breakdown inspection
- Performance comparison between weight sets

## Support

For questions or issues with weight tuning:
- Open an issue on GitHub with example queries and score breakdowns
- Include your codebase characteristics (language, size, churn rate)
- Share which weights you've tried and the observed problems
