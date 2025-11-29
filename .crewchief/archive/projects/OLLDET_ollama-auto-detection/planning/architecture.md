# Architecture: Ollama Auto-Detection Fallback Chain

## Solution Overview

Replace the hardcoded `is_ollama_available()` function with a new `detect_ollama_endpoint()` function that tries multiple endpoints and returns the first reachable one.

## Design Decisions

### 1. Function Signature Change

**Current:**
```rust
async fn is_ollama_available() -> bool
```

**Proposed:**
```rust
async fn detect_ollama_endpoint() -> Option<String>
```

**Rationale:** Returning the endpoint allows consistent use between detection and provider creation. `None` indicates no endpoint is available.

### 2. Fallback Order

```
1. MAPROOM_EMBEDDING_API_ENDPOINT (explicit config)
   → Extract base URL from embed endpoint

2. localhost:11434 (native development)

3. host.docker.internal:11434 (Docker/devcontainer)
```

**Rationale:**
- Explicit config always wins (user intent)
- localhost is the most common case
- Docker host is the fallback for containerized environments

### 3. Endpoint URL Extraction

When `MAPROOM_EMBEDDING_API_ENDPOINT=http://custom:11434/api/embed` is set:
- Extract base: `http://custom:11434`
- Check: `http://custom:11434/api/tags`

```rust
fn extract_base_url(endpoint: &str) -> Option<String> {
    // "http://host:port/api/embed" → "http://host:port"
    // Also handles trailing slashes: "http://host:port/api/embed/" → "http://host:port"
    let endpoint = endpoint.trim_end_matches('/');
    endpoint.strip_suffix("/api/embed")
        .or_else(|| endpoint.strip_suffix("/api/embeddings"))
        .map(|s| s.to_string())
}
```

### 4. Health Check Endpoint

Use `/api/tags` for health check (current behavior):
- Returns quickly
- Returns 200 when Ollama is healthy
- Doesn't require a model to be loaded

### 5. Timeout Strategy

- 2 seconds per endpoint (current timeout)
- Sequential checks (not parallel)
- Total max: 6 seconds in worst case

**Why not parallel?**
- Simpler implementation
- Avoids race conditions
- 6 seconds worst case is acceptable for startup

## Code Changes

### Modified Function: `detect_ollama_endpoint()`

```rust
/// Detect Ollama endpoint using fallback chain.
///
/// Checks endpoints in order:
/// 1. MAPROOM_EMBEDDING_API_ENDPOINT (if set, extract base URL)
/// 2. localhost:11434
/// 3. host.docker.internal:11434
///
/// Returns the first reachable endpoint's base URL, or None.
async fn detect_ollama_endpoint() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;

    // Build fallback list
    let mut endpoints = Vec::new();

    // 1. Check explicit endpoint config
    if let Ok(embed_endpoint) = env::var("MAPROOM_EMBEDDING_API_ENDPOINT") {
        if let Some(base) = extract_base_url(&embed_endpoint) {
            endpoints.push(base);
        }
    }

    // 2. localhost (native development)
    endpoints.push("http://localhost:11434".to_string());

    // 3. Docker host (containerized development)
    endpoints.push("http://host.docker.internal:11434".to_string());

    // Log all endpoints we'll try (helpful for debugging)
    tracing::debug!("Ollama detection fallback chain: {:?}", endpoints);

    // Try each endpoint
    for base in endpoints {
        let check_url = format!("{}/api/tags", base);
        tracing::debug!("Checking Ollama at: {}", check_url);

        match client.get(&check_url).send().await {
            Ok(response) if response.status().is_success() => {
                tracing::info!("Ollama detected at: {}", base);
                return Some(base);
            }
            Ok(response) => {
                tracing::debug!("Ollama check failed at {}: status {}", base, response.status());
            }
            Err(e) => {
                tracing::debug!("Ollama not available at {}: {}", base, e);
            }
        }
    }

    tracing::debug!("No Ollama endpoint detected");
    None
}

fn extract_base_url(endpoint: &str) -> Option<String> {
    // Handle trailing slashes: "http://host:port/api/embed/" → "http://host:port"
    let endpoint = endpoint.trim_end_matches('/');
    endpoint
        .strip_suffix("/api/embed")
        .or_else(|| endpoint.strip_suffix("/api/embeddings"))
        .map(|s| s.to_string())
}
```

### Updated `create_provider_from_env()`

```rust
pub async fn create_provider_from_env() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    let explicit_provider = env::var("MAPROOM_EMBEDDING_PROVIDER").ok();

    let (provider_name, detected_endpoint) = match explicit_provider.as_deref() {
        Some(p) => {
            tracing::debug!("Using explicit provider: {}", p);
            (p.to_lowercase(), None)
        }
        None => {
            tracing::debug!("No MAPROOM_EMBEDDING_PROVIDER set, auto-detecting Ollama");
            match detect_ollama_endpoint().await {
                Some(endpoint) => {
                    tracing::info!("Ollama auto-detected at: {}", endpoint);
                    ("ollama".to_string(), Some(endpoint))
                }
                None => {
                    tracing::warn!("No embedding provider detected");
                    return Err(EmbeddingError::Config(ConfigError::MissingConfig(
                        "No embedding provider configured...".to_string()
                    )));
                }
            }
        }
    };

    match provider_name.as_str() {
        "ollama" => {
            // Use detected endpoint if available, else check env, else default
            let endpoint = detected_endpoint
                .map(|base| format!("{}/api/embed", base))
                .or_else(|| env::var("MAPROOM_EMBEDDING_API_ENDPOINT").ok())
                .unwrap_or_else(|| "http://localhost:11434/api/embed".to_string());
            // ... create provider
        }
        // ... other providers
    }
}
```

## Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    create_provider_from_env()                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────┐                                     │
│  │ MAPROOM_EMBEDDING_  │──Yes──► Use explicit provider       │
│  │ PROVIDER set?       │                                     │
│  └──────────┬──────────┘                                     │
│             │ No                                             │
│             ▼                                                │
│  ┌─────────────────────┐                                     │
│  │ detect_ollama_      │──None──► Error: no provider         │
│  │ endpoint()          │                                     │
│  └──────────┬──────────┘                                     │
│             │ Some(endpoint)                                 │
│             ▼                                                │
│  ┌─────────────────────┐                                     │
│  │ Create Ollama       │                                     │
│  │ provider with       │                                     │
│  │ detected endpoint   │                                     │
│  └─────────────────────┘                                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    detect_ollama_endpoint()                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌───────────────────────────┐                               │
│  │ 1. MAPROOM_EMBEDDING_     │───reachable───► Some(base)    │
│  │    API_ENDPOINT base URL  │                               │
│  └────────────┬──────────────┘                               │
│               │ not reachable                                │
│               ▼                                              │
│  ┌───────────────────────────┐                               │
│  │ 2. localhost:11434        │───reachable───► Some(base)    │
│  └────────────┬──────────────┘                               │
│               │ not reachable                                │
│               ▼                                              │
│  ┌───────────────────────────┐                               │
│  │ 3. host.docker.internal   │───reachable───► Some(base)    │
│  │    :11434                 │                               │
│  └────────────┬──────────────┘                               │
│               │ not reachable                                │
│               ▼                                              │
│           None                                               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Files Changed

| File | Change |
|------|--------|
| `crates/maproom/src/embedding/factory.rs` | Add `detect_ollama_endpoint()`, `extract_base_url()`, update `create_provider_from_env()` |

## Dependencies

No new dependencies. Uses existing:
- `reqwest` for HTTP
- `tracing` for logging
- `std::env` for environment variables

## Performance Impact

| Scenario | Before | After |
|----------|--------|-------|
| Explicit provider set | ~0ms | ~0ms (no detection) |
| Ollama on localhost | ~10ms | ~10ms (first try succeeds) |
| Ollama on Docker host | ~2s timeout + fail | ~2s (localhost timeout) + ~10ms |
| No Ollama | ~2s | ~6s (all endpoints timeout) |

**Trade-off:** Slightly slower failure case, but automatic success in Docker environments.

## Future Considerations

1. **Parallel detection** - Could speed up worst case
2. **Caching** - Cache detected endpoint for session
3. **OLLAMA_URL support** - Some tools use this env var
4. **Service discovery** - Kubernetes DNS, Docker service names

These are intentionally out of scope for MVP.
