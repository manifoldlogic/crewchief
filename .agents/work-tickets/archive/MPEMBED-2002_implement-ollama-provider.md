# Ticket: MPEMBED-2002: Implement OllamaProvider for local embeddings

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- embeddings-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create OllamaProvider struct that implements EmbeddingProvider trait, calling Ollama HTTP API to generate 768-dimensional embeddings.

## Background
Ollama runs locally on `http://localhost:11434` with nomic-embed-text model. The Ollama API endpoint is `POST /api/embed` with JSON `{"model": "nomic-embed-text", "input": "text"}`.

Ollama doesn't support native batching, so we'll use concurrent requests with tokio::spawn. Ollama is the zero-config default provider (auto-detected if running).

This ticket implements the Ollama provider as part of Phase 2: Provider Abstraction from the MPEMBED multi-provider embedding support plan.

## Acceptance Criteria
- [x] `OllamaProvider` struct defined with `client`, `endpoint`, `model` fields
- [x] `OllamaProvider::new()` constructor validates endpoint (returns error if unreachable)
- [x] `embed()` method calls Ollama API and returns 768-dim vector
- [x] `embed_batch()` method uses concurrent requests (not sequential)
- [x] `dimension()` returns 768 (nomic-embed-text fixed dimension)
- [x] `provider_name()` returns "ollama"
- [x] HTTP errors mapped to `EmbeddingError` enum
- [x] Retry logic for transient failures (e.g., Ollama temporary overload)

## Technical Requirements
- File location: `crates/maproom/src/embedding/ollama.rs` (NEW FILE)
- Use `reqwest::Client` for HTTP calls
- Endpoint configurable via constructor (default: `http://localhost:11434/api/embed`)
- Model configurable via constructor (default: `nomic-embed-text`)
- Timeout: 30 seconds per request (embedding can take time)
- Concurrent batch limit: 10 simultaneous requests (avoid overwhelming Ollama)
- Use semaphore to limit concurrency

## Implementation Notes

```rust
// crates/maproom/src/embedding/ollama.rs (NEW FILE)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use std::sync::Arc;
use crate::embedding::provider::{EmbeddingProvider, Vector};
use crate::embedding::error::EmbeddingError;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct OllamaResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Clone)]
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
    semaphore: Arc<Semaphore>,
}

impl OllamaProvider {
    pub fn new(endpoint: String, model: String) -> Result<Self, EmbeddingError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            endpoint,
            model,
            semaphore: Arc::new(Semaphore::new(10)), // Limit to 10 concurrent requests
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        let response = self.client
            .post(&self.endpoint)
            .json(&OllamaRequest {
                model: self.model.clone(),
                input: text,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(EmbeddingError::ProviderError(
                format!("Ollama API error: {}", response.status())
            ));
        }

        let body: OllamaResponse = response.json().await?;
        Ok(body.embeddings[0].clone())
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        // Ollama doesn't support native batching, use concurrent requests with semaphore
        let mut tasks = Vec::new();
        for text in texts {
            let provider = self.clone();
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            tasks.push(tokio::spawn(async move {
                let result = provider.embed(text).await;
                drop(permit); // Release semaphore
                result
            }));
        }

        let results = futures::future::join_all(tasks).await;
        results.into_iter()
            .map(|r| r.map_err(|e| EmbeddingError::Other(e.to_string()))?)
            .collect()
    }

    fn dimension(&self) -> usize {
        768 // nomic-embed-text fixed dimension
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }
}
```

## Dependencies
- MPEMBED-2001 (trait definition)

## Risk Assessment
- **Risk**: Concurrent requests overwhelm Ollama (OOM or slowdown)
  - **Mitigation**: Use semaphore to limit concurrency to 10 simultaneous requests
- **Risk**: Network timeouts on slow machines
  - **Mitigation**: 30-second timeout per request, configurable via constructor

## Files/Packages Affected
- crates/maproom/src/embedding/ollama.rs (create)
- crates/maproom/src/embedding/mod.rs (modify - add `pub mod ollama;`)
