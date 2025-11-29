# Analysis: Ollama Auto-Detection Fallback Chain

## Problem Statement

The `is_ollama_available()` function in `/crates/maproom/src/embedding/factory.rs` is hardcoded to check only `localhost:11434`, which fails in containerized environments where Ollama runs on the host machine.

**Current behavior:**
```rust
match client.get("http://localhost:11434/api/tags").send().await {
```

**Expected behavior:**
1. Check `MAPROOM_EMBEDDING_API_ENDPOINT` first (explicit override)
2. Try `localhost:11434` (native development)
3. Try `host.docker.internal:11434` (Docker/devcontainer environments)

## Context

### When This Fails

1. **DevContainer development** - Ollama runs on host, container uses `host.docker.internal`
2. **Docker Compose** - Separate Ollama container accessible via service name
3. **Remote Ollama** - Ollama on different machine/port

### When This Works

1. **Native macOS/Linux development** - Ollama on localhost
2. **Explicit configuration** - User sets `MAPROOM_EMBEDDING_PROVIDER=ollama`

## Current Code Analysis

### `is_ollama_available()` (factory.rs:413-442)

```rust
async fn is_ollama_available() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    { ... };

    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}
```

**Issues:**
- Hardcoded to `localhost:11434`
- No fallback chain
- Ignores `MAPROOM_EMBEDDING_API_ENDPOINT`
- No container-aware detection

### `create_provider_from_env()` (factory.rs:152-299)

When creating the Ollama provider, it correctly uses the endpoint env var:

```rust
let endpoint = env::var("MAPROOM_EMBEDDING_API_ENDPOINT")
    .unwrap_or_else(|_| "http://localhost:11434/api/embed".to_string());
```

**Gap:** Auto-detection uses different logic than provider creation.

## User Impact

| Environment | Before Fix | After Fix |
|-------------|-----------|-----------|
| Native localhost | Works | Works |
| DevContainer | Fails | Works |
| Docker Compose | Fails | Works |
| Custom endpoint | Requires MAPROOM_EMBEDDING_PROVIDER | Works with just endpoint |

## Existing Industry Solutions

### Docker's Host Detection

Docker provides `host.docker.internal` as a DNS alias for the host machine. This is:
- Built into Docker Desktop (macOS, Windows)
- Requires `--add-host` on Linux Docker

### Common Patterns

1. **Environment variable override** - Most tools check an env var first
2. **Well-known fallbacks** - Try localhost, then container-aware addresses
3. **Fast timeout** - 1-2 seconds per attempt to avoid slow startup

## Related Code

The Ollama provider creation already handles endpoints correctly:

```rust
// factory.rs:186-189
let endpoint = env::var("MAPROOM_EMBEDDING_API_ENDPOINT")
    .unwrap_or_else(|_| "http://localhost:11434/api/embed".to_string());
```

The detection function should align with this logic.

## Requirements

### Functional

1. Check endpoints in this order:
   - `MAPROOM_EMBEDDING_API_ENDPOINT` (if set, derive base URL)
   - `localhost:11434`
   - `host.docker.internal:11434`

2. Use first reachable endpoint

3. Return which endpoint was detected (for consistent provider creation)

### Non-Functional

1. **Timeout**: 2 seconds per endpoint (current behavior)
2. **Total startup impact**: Max 6 seconds (3 endpoints x 2s)
3. **Logging**: Debug-level logging of detection attempts

## Scope

### In Scope

- Modify `is_ollama_available()` to try multiple endpoints
- Return detected endpoint for use in provider creation
- Update logging to show which endpoint was detected

### Out of Scope

- Custom DNS resolution
- Service discovery
- Kubernetes-specific detection
- Retry logic (single attempt per endpoint)
