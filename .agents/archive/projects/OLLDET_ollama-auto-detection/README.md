# OLLDET: Ollama Auto-Detection Fallback Chain

## Problem

The `is_ollama_available()` function is hardcoded to check `localhost:11434`. This fails in containerized environments (DevContainer, Docker) where Ollama runs on the host machine and is accessible via `host.docker.internal`.

## Solution

Replace `is_ollama_available()` with `detect_ollama_endpoint()` that tries multiple endpoints in order:

1. **`MAPROOM_EMBEDDING_API_ENDPOINT`** - Explicit user configuration
2. **`localhost:11434`** - Native development (current behavior)
3. **`host.docker.internal:11434`** - Docker/DevContainer environments

## Impact

| Environment | Before | After |
|-------------|--------|-------|
| Native (localhost) | Works | Works |
| DevContainer | Fails | Works |
| Docker Compose | Fails | Works |
| Custom endpoint | Requires MAPROOM_EMBEDDING_PROVIDER | Works with just endpoint |

## Agents

- **rust-indexer-engineer** - Implementation
- **unit-test-runner** - Test execution
- **verify-ticket** - Acceptance verification
- **commit-ticket** - Commit creation

## Planning Documents

- [Analysis](planning/analysis.md) - Problem space and requirements
- [Architecture](planning/architecture.md) - Solution design
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Execution phases

## Tickets

| ID | Title | Status |
|----|-------|--------|
| OLLDET-1001 | Implement Ollama Endpoint Detection Fallback | Pending |

*Note: Manual verification is included in OLLDET-1001's acceptance criteria rather than as a separate ticket.*

## Files Changed

- `crates/maproom/src/embedding/factory.rs`

## Quick Reference

After implementation, auto-detection works without configuration:

```bash
# DevContainer - no config needed
cargo run -p crewchief-maproom -- scan --path . --repo test
# Logs: "Ollama detected at: http://host.docker.internal:11434"

# Native - no config needed (existing behavior)
cargo run -p crewchief-maproom -- scan --path . --repo test
# Logs: "Ollama detected at: http://localhost:11434"

# Custom endpoint - explicit override
export MAPROOM_EMBEDDING_API_ENDPOINT=http://ollama.custom:11434/api/embed
cargo run -p crewchief-maproom -- scan --path . --repo test
# Logs: "Ollama detected at: http://ollama.custom:11434"
```
