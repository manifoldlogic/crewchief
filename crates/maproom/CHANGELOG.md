# Changelog

All notable changes to the Maproom indexer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-07-03

### Added
- **PostgreSQL + pgvector storage backend** (build with `--features postgres`, select with a `postgres://` `MAPROOM_DATABASE_URL`): full `Store`-trait implementation with junction-based multi-worktree chunk sharing, a persistent content-addressed embedding pool, hand-rolled migration runner (`migrations_pg/`), and schema-health verification. Parity-tested against SQLite via the `store_parity` suite. (#56, #57)
- **Postgres trust triad** (#57): a pgvector CI job runs the live PG suites against a real `pgvector/pgvector:pg16` container (F73); the search stack and socket daemon are backend-neutral over `dyn Store` (F45); backend selection fails loudly — a `postgres://` URL on a non-postgres build or an unreachable server exits 2 instead of silently creating and using a SQLite database (F70).
- `maproom serve --pid-path <path>` to override the global per-uid PID file (test/process isolation).
- **Incremental Scanning Optimization (INCRSCAN)**: Scan commands now automatically skip re-indexing unchanged worktrees by comparing git tree SHAs, achieving 10,000x speedup for unchanged code (2-3 hours → 5-10ms). This makes the genetic optimizer usable, reducing 12-worktree setup from 24+ hours to under 2 minutes.
  - Tree SHA checking before scan with automatic skip when no changes detected
  - State persistence after scan to track last indexed tree SHA per worktree
  - Fail-safe design: any error defaults to full scan (never skips incorrectly)
  - Force flag (`--force`) to override skip behavior and perform full scan
  - Database table `worktree_index_state` to track indexing history

### Changed
- **BREAKING (library API)**: `db::connect()`/`connect_url()` now return `Arc<dyn Store + Send + Sync>` instead of `SqliteStore`; use `db::connect_sqlite()` for the concrete SQLite store. CLI behavior is unchanged (SQLite remains the default backend).
- Daemon JSON-RPC conformance: notifications (absent `id`) receive no reply; a missing/wrong `jsonrpc` version returns `-32600`; store failures during worktree pre-validation return retryable `-32000` instead of `-32602 unknown worktree`; an unknown worktree name yields an empty result set on every search path instead of silently widening scope.
- Ollama endpoint resolution honors `MAPROOM_EMBEDDING_API_ENDPOINT` > `MAPROOM_OLLAMA_URL` > `OLLAMA_URL` > `OLLAMA_HOST` > auto-detect > default; scheme-less hosts default to port 11434; empty values are treated as unset; when the environment configures an endpoint, auto-detection probes only the configured candidates (no localhost fallback masking a dead configured host).
- Scan command now performs incremental scans by default, logging "⚡ Incremental scan mode" on startup
- Progress tracking exposed through getter methods for state persistence integration

### Fixed
- 21 CLI end-to-end regressions across scan/watch/search/daemon/db (R01–R21), including: the watch pipeline never indexing live working-tree edits (tree-SHA gate removed from the live-event path); pseudo-Deleted events from commit/stash/restore destroying just-indexed data; stale-worktree cleanup re-reporting forever; FTS index bloat from missed external-content deletes; daemon DOA in provider-less environments (hybrid search now degrades to FTS).
- 40 in-depth review findings, including: junction-aware `delete_worktree_data` (chunks shared with other worktrees survive; FTS5 postings and embeddings cleaned with last-reference discipline); supersession GC scoped to candidate chunks (removes O(files × total-chunks) rescan overhead); PID-file takeover hardening (O_NOFOLLOW, ownership, post-lock re-stat); honest embedding metrics (`failed_requests` accounting); `--max-cost 0` no longer aborts free-provider runs; backup-table names validated before SQL interpolation in markdown migrate rollback/delete.

## [0.1.0] - Initial Release

### Added
- Initial implementation of semantic code search indexer
- Tree-sitter based parsing for TypeScript, Rust, Python, Go, JavaScript, Markdown
- PostgreSQL database with pgvector extension for vector storage
- Multi-provider embedding support (Ollama, OpenAI, Google Vertex AI)
- Hybrid search combining FTS (full-text) and vector similarity
- Context assembly with code relationships (imports, callers, callees)
- Parallel batch processing pipeline for large codebases
- CLI commands: scan, search, context, upsert, generate-embeddings
- Database migrations with automatic schema management
