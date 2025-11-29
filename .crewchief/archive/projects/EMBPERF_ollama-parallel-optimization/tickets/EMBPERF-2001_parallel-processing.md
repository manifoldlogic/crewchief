# Ticket: EMBPERF-2001: Parallel Sub-Batch Processing

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: Tests MUST be executed - unit tests for sub-batch splitting and parallel execution.

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add concurrent processing of sub-batches using existing ParallelConfig infrastructure. Split large batches into sub-batches, process them in parallel with semaphore-controlled concurrency, merge results in order.

## Background
Phase 1 (EMBPERF-1001) implements batch API support. This ticket adds parallelism on top:
- Split large batches into configurable sub-batches
- Process sub-batches concurrently using tokio semaphore
- Merge results preserving original order

Combined with batch API, this should achieve 10-20x throughput improvement.

This implements Phase 2 from `plan.md`.

## Acceptance Criteria
- [x] `OllamaProvider` accepts `ParallelConfig` in constructor
- [x] `embed_batch_parallel()` method implemented
- [x] Sub-batches split according to `config.sub_batch_size`
- [x] Concurrent requests limited by `config.max_concurrency` via Semaphore
- [x] Results merged in correct order after parallel execution
- [x] `embed_batch()` uses parallel when `config.enabled && texts.len() > sub_batch_size`
- [x] Factory passes `ParallelConfig` to `OllamaProvider`
- [x] Config defaults updated: `sub_batch_size: 50`, `max_concurrency: 8`
- [x] Unit tests pass for sub-batch splitting logic
- [x] Unit tests pass for result ordering
- [x] Integration test confirms parallel execution works

## Technical Requirements

### OllamaProvider Changes
```rust
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
    parallel_config: ParallelConfig,  // NEW
    semaphore: Arc<Semaphore>,        // NEW - for concurrency control
}

impl OllamaProvider {
    pub fn new_with_config(
        endpoint: String,
        model: String,
        config: ParallelConfig,
    ) -> Result<Self, EmbeddingError> {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
        // ...
    }

    async fn embed_batch_parallel(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        // Split into sub-batches
        let sub_batches: Vec<Vec<String>> = texts
            .chunks(self.parallel_config.sub_batch_size)
            .map(|c| c.to_vec())
            .collect();

        // Process in parallel with semaphore
        let handles: Vec<_> = sub_batches
            .into_iter()
            .enumerate()
            .map(|(idx, batch)| {
                let semaphore = self.semaphore.clone();
                let this = self.clone(); // or Arc<Self>
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    (idx, this.embed_batch_raw(batch).await)
                })
            })
            .collect();

        // Collect and sort by index
        let mut results: Vec<(usize, Vec<Vector>)> = Vec::new();
        for handle in handles {
            let (idx, result) = handle.await?;
            results.push((idx, result?));
        }
        results.sort_by_key(|(idx, _)| *idx);

        // Flatten in order
        Ok(results.into_iter().flat_map(|(_, vecs)| vecs).collect())
    }

    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        if self.parallel_config.enabled && texts.len() > self.parallel_config.sub_batch_size {
            self.embed_batch_parallel(texts).await
        } else {
            self.embed_batch_raw(texts).await
        }
    }
}
```

### Factory Changes (`factory.rs`)
```rust
pub fn create_provider_from_env() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    match provider_type {
        Provider::Ollama => {
            let parallel_config = ParallelConfig::from_env()?;
            Box::new(OllamaProvider::new_with_config(endpoint, model, parallel_config)?)
        }
        // ...
    }
}
```

### Config Defaults (`config.rs`)
```rust
impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            enabled: true,           // Enable by default
            sub_batch_size: 50,      // Was: 25
            max_concurrency: 8,      // Was: 4
        }
    }
}
```

## Implementation Notes

### Concurrency Control
Use `tokio::sync::Semaphore` to limit concurrent requests. This prevents overwhelming Ollama with too many simultaneous requests.

### Order Preservation
Critical: Track sub-batch index through parallel execution, sort before merging. Test thoroughly.

### Progress Logging
```rust
info!("Batch embedding: {} texts in {} sub-batches with concurrency {}",
      total_texts, num_batches, self.parallel_config.max_concurrency);
debug!("Sub-batch {} completed in {}ms ({} texts)",
       batch_idx, elapsed_ms, batch_size);
info!("Batch complete: {} texts in {:.2}s ({:.0} texts/sec)",
      total_texts, elapsed_secs, throughput);
```

## Dependencies
- EMBPERF-1001 (batch API support must be complete)
- `tokio::sync::Semaphore` (already in tokio crate)

## Risk Assessment
- **Risk**: Order corruption in parallel execution
  - **Mitigation**: Track indices, sort before merge, comprehensive tests
- **Risk**: Semaphore starvation or deadlock
  - **Mitigation**: Use standard tokio Semaphore pattern, bounded permits
- **Risk**: Memory pressure with many concurrent batches
  - **Mitigation**: Semaphore limits concurrency, batch size limits memory per request

## Files/Packages Affected
- `crates/maproom/src/embedding/ollama.rs` - Main parallel implementation
- `crates/maproom/src/embedding/factory.rs` - Pass ParallelConfig to Ollama
- `crates/maproom/src/embedding/config.rs` - Update defaults

## Tests to Add

### Unit Tests (in `ollama.rs`)
```rust
#[test]
fn test_sub_batch_splitting() {
    // 105 texts with batch_size 50 → 3 batches: [50, 50, 5]
    let texts: Vec<String> = (0..105).map(|i| i.to_string()).collect();
    let batches: Vec<Vec<String>> = texts.chunks(50).map(|c| c.to_vec()).collect();
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0].len(), 50);
    assert_eq!(batches[2].len(), 5);
}

#[test]
fn test_result_merge_ordering() {
    // Simulate out-of-order completion
    let mut results = vec![
        (2, vec!["c1".to_string(), "c2".to_string()]),
        (0, vec!["a1".to_string(), "a2".to_string()]),
        (1, vec!["b1".to_string(), "b2".to_string()]),
    ];
    results.sort_by_key(|(idx, _)| *idx);
    let merged: Vec<String> = results.into_iter().flat_map(|(_, v)| v).collect();
    assert_eq!(merged, vec!["a1", "a2", "b1", "b2", "c1", "c2"]);
}
```

### Integration Test (requires Ollama)
```rust
#[tokio::test]
#[ignore]
async fn test_parallel_preserves_order() {
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 10,
        max_concurrency: 4,
    };
    let provider = OllamaProvider::new_with_config(endpoint, model, config)?;

    // Create texts with identifiable content
    let texts: Vec<String> = (0..50).map(|i| format!("text_{}", i)).collect();
    let embeddings = provider.embed_batch(texts.clone()).await?;

    // Verify order by re-embedding individually and comparing
    for (i, text) in texts.iter().enumerate() {
        let single = provider.embed(text.clone()).await?;
        assert_eq!(embeddings[i], single, "Mismatch at index {}", i);
    }
}
```
