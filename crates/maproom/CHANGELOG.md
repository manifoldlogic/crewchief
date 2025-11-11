# Changelog

All notable changes to the Maproom indexer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Incremental Scanning Optimization (INCRSCAN)**: Scan commands now automatically skip re-indexing unchanged worktrees by comparing git tree SHAs, achieving 10,000x speedup for unchanged code (2-3 hours → 5-10ms). This makes the genetic optimizer usable, reducing 12-worktree setup from 24+ hours to under 2 minutes.
  - Tree SHA checking before scan with automatic skip when no changes detected
  - State persistence after scan to track last indexed tree SHA per worktree
  - Fail-safe design: any error defaults to full scan (never skips incorrectly)
  - Force flag (`--force`) to override skip behavior and perform full scan
  - Database table `worktree_index_state` to track indexing history

### Changed
- Scan command now performs incremental scans by default, logging "⚡ Incremental scan mode" on startup
- Progress tracking exposed through getter methods for state persistence integration

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
