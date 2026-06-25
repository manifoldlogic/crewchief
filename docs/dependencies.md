# Maproom Dependencies

This document tracks the rationale for key dependencies in the Maproom project.

## Core Dependencies

### Async Runtime
- **tokio** `1.x`: Industry-standard async runtime for Rust
  - Features: rt-multi-thread, macros, fs, process, time, sync, signal
  - Reason: Provides async I/O for database, HTTP, and file operations

### Database

Maproom stores data behind a `Store` trait with two backends.

- **rusqlite** (`bundled`, `chrono`): SQLite backend — **default, always compiled in**
  - Reason: zero-config local storage at `~/.maproom/maproom.db`; statically linked sqlite-vec for vector search
- **r2d2** / **r2d2_sqlite**: SQLite connection pooling
- **sqlx** `0.8` (`runtime-tokio` + `tls-rustls` + `postgres`): PostgreSQL + pgvector backend — **optional, behind the `postgres` feature**
  - Reason: native-async Postgres driver, selected at runtime for `postgres://`/`postgresql://` URLs. The default build pulls **no** sqlx/pgvector dependencies. The pgvector *extension* is used via text casts, not the `pgvector` crate.

### Parsing & Code Analysis
- **tree-sitter** `0.22.x`: Incremental parsing framework
  - Reason: Parse source code into syntax trees for indexing
- **tree-sitter-typescript** `0.21.x`: TypeScript grammar
- **tree-sitter-javascript** `0.21.x`: JavaScript grammar
- **tree-sitter-python** `0.21.x`: Python grammar
- **tree-sitter-rust** `0.21.x`: Rust grammar
- **tree-sitter-go** `0.21.x`: Go grammar
- **tree-sitter-md** `0.2.x`: Markdown grammar
  - Reason: Multi-language support for code indexing

### Serialization
- **serde** `1.x`: Serialization framework
  - Features: derive
  - Reason: JSON/YAML serialization for configs and API responses
- **serde_json** `1.x`: JSON support
- **serde_yaml** `0.9.x`: YAML support for configuration files

### Error Handling
- **anyhow** `1.0.100+`: Error handling with context
  - Reason: Ergonomic error handling for CLI and library code
- **thiserror** `2.x`: Custom error types
  - Reason: Define domain-specific error types with derive macros

### CLI
- **clap** `4.5.x`: Command-line argument parser
  - Features: derive
  - Reason: Parse CLI commands and arguments with derive macros

## Multi-Provider Embeddings (MPEMBED Project)

### Provider Abstraction (Phase 2)
- **async-trait** `0.1.x`: Async trait definitions
  - Version: 0.1.89+
  - Reason: Enable trait-based abstraction for embedding providers (OpenAI, Ollama, Google)
  - Usage: `#[async_trait] pub trait EmbeddingProvider { ... }`

### Google Vertex AI Support (Phase 3)
- **google-cloud-auth** `0.13.x`: Google service account authentication
  - Version: 0.13.x (pinned to minor)
  - Reason: Authenticate with Google Vertex AI for embeddings
  - Note: Has heavy transitive dependencies (gRPC, protobuf, tonic ~15MB)
  - Pinned: Breaking changes common in Google crates, review quarterly

### HTTP Client
- **reqwest** `0.12.x`: HTTP client
  - Features: json
  - Reason: Make HTTP requests to embedding APIs (OpenAI, Ollama, Google)

### Caching
- **lru** `0.12.x`: LRU cache implementation
  - Reason: Cache embeddings to reduce API calls and improve performance

## Utilities

### File System
- **ignore** `0.4.24+`: .gitignore support
  - Reason: Respect .gitignore patterns when scanning repositories
- **notify** `6.x`: File system events
  - Features: macos_kqueue
  - Reason: Watch for file changes and trigger re-indexing

### Cryptography
- **blake3** `1.8.x`: Fast hashing
  - Reason: Content-addressable storage, change detection
- **sha2** `0.10.x`: SHA-256 hashing
  - Reason: Cache key generation for embeddings

### Monitoring
- **prometheus** `0.13.x`: Metrics collection
  - Features: process
  - Reason: Export metrics for monitoring and alerting
  - Note: Uses protobuf 2.28 with known vulnerability (RUSTSEC-2024-0437)
    - Risk accepted: prometheus is not processing untrusted user input
    - Mitigation: Monitor for updates to prometheus crate

### Logging
- **tracing** `0.1.x`: Structured logging
  - Reason: Instrumented logging with spans and events
- **tracing-subscriber** `0.3.20+`: Tracing output
  - Features: env-filter, fmt
  - Reason: Format and filter tracing output
  - Updated: Fix ANSI escape sequence injection (RUSTSEC-2025-0055)

### Token Counting
- **tiktoken-rs** `0.5.x`: OpenAI tokenizer
  - Reason: Count tokens for context assembly and budget management

### Parallel Processing
- **rayon** `1.10.x`: Data parallelism
  - Reason: Parallel batch processing during indexing
- **crossbeam** `0.8.x`: Concurrent data structures
  - Reason: Lock-free channels and queues for parallel operations

## Security Audit (Last Updated: 2025-10-28)

### Known Vulnerabilities (Accepted)
1. **protobuf** `2.28.0` (via prometheus `0.13.4`)
   - Advisory: RUSTSEC-2024-0437
   - Issue: Crash due to uncontrolled recursion in parsing
   - Status: **Accepted risk** - prometheus not processing untrusted user input
   - Mitigation: Prometheus metrics are internal only, not exposed to user data
   - Fix: Waiting for prometheus crate to update to protobuf 3.7.2+

2. **ring** `0.17.9` (via rustls, jsonwebtoken)
   - Advisory: RUSTSEC-2025-0009
   - Issue: Some AES functions may panic when overflow checking is enabled
   - Status: **Low impact** - affects specific AES configurations not used in our code
   - Mitigation: We use ring transitively through rustls/google-cloud-auth for TLS, not direct AES
   - Fix: Waiting for transitive dependencies to update to ring 0.17.12+

### Recently Fixed (2025-10-28)
- **tracing-subscriber**: Updated from `0.3.x` to `0.3.20`
  - Fixed: ANSI escape sequence injection (RUSTSEC-2025-0055)
  - Impact: Prevented log poisoning from user input

## Dependency Tree Size
- Total crates: 410 (as of 2025-10-28, after adding google-cloud-auth)
- Direct dependencies: 31
- Build time: ~36 seconds (release build on typical developer machine)
- Build time impact from MPEMBED additions:
  - google-cloud-auth adds gRPC/protobuf dependencies (+22 crates)
  - Increases build time by ~10-15% (acceptable for multi-provider support)
  - Consider feature flags for optional Google provider in future if build time becomes issue

## Update Policy
- **Patch updates**: Apply immediately for security fixes
- **Minor updates**: Apply conservatively, test thoroughly
- **Major updates**: Defer unless critical (breaking changes)
- **Review schedule**: Quarterly dependency audit with cargo-audit and cargo-outdated
