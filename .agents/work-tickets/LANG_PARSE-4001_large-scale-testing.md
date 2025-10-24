# Ticket: LANG_PARSE-4001: Large-Scale Validation Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- performance-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Conduct comprehensive large-scale validation testing of the multi-language parser system across Python, Rust, and Go. Index 10+ real-world projects per language to validate parsing accuracy, measure performance benchmarks, profile memory usage under load, and analyze error rates to ensure production readiness.

## Background
Phase 4 of the LANG_PARSE project focuses on validation and performance optimization. Before the system can be considered production-ready, it must be tested against real-world codebases at scale. This ticket implements the first critical validation task: large-scale testing across multiple real projects to identify any remaining parsing issues, performance bottlenecks, or memory concerns that may not have surfaced in unit testing.

The testing will use well-known open-source projects (Django, Flask, numpy for Python; tokio, serde for Rust; Kubernetes, Docker for Go) to ensure the parser handles diverse coding styles, edge cases, and large codebases effectively.

## Acceptance Criteria
- [ ] Successfully index 10+ real-world projects per language (Python, Rust, Go)
- [ ] Achieve <1% error rate across all languages and projects
- [ ] Meet performance target of 150 files/min indexing speed
- [ ] Complete memory profiling under load with documented baseline and peak usage
- [ ] Generate comprehensive validation report documenting results
- [ ] Create performance benchmarks suite for all three languages
- [ ] Document any discovered issues with reproduction steps

## Technical Requirements
- Real project test corpus:
  - **Python**: Django, Flask, numpy, requests, pytest, black, FastAPI, Celery, Airflow, Pandas (minimum 10)
  - **Rust**: tokio, serde, clap, actix-web, diesel, rocket, hyper, async-std, warp, reqwest (minimum 10)
  - **Go**: Kubernetes, Docker, Prometheus, Terraform, etcd, Hugo, CockroachDB, Gitea, Minio, Caddy (minimum 10)
- Performance benchmarking infrastructure:
  - Measure files/minute indexing speed
  - Track memory usage (baseline, peak, average)
  - Monitor CPU utilization
  - Record database query performance
- Error rate analysis:
  - Classify errors by type (parse failures, extraction errors, database errors)
  - Track error rates per language
  - Log all failed files with context
- Memory profiling tooling:
  - Use Rust profiling tools (e.g., valgrind, heaptrack, or cargo-instruments)
  - Profile peak memory usage during large repository indexing
  - Identify memory leaks or excessive allocations
- Validation metrics:
  - Total files processed per language
  - Successful vs. failed parses
  - Average symbols extracted per file
  - Database insertion performance
  - End-to-end indexing time per project

## Implementation Notes

### Test Structure
Create a new validation test suite in `crates/maproom/tests/validation/large_scale_test.rs` that:
1. Downloads or clones specified real-world projects
2. Runs the maproom indexer on each project
3. Collects metrics during indexing
4. Validates results against acceptance criteria
5. Generates a markdown report with findings

### Test Fixtures
Set up `crates/maproom/tests/fixtures/` to cache or reference real projects:
- Consider using git submodules or automated cloning
- Store project metadata (repo URL, commit SHA, expected file count)
- Document how to refresh test fixtures

### Performance Benchmarking
Use Rust's criterion.rs or similar benchmarking framework to:
- Measure parsing performance per language
- Compare performance across different file sizes
- Track performance regression over time

### Memory Profiling Strategy
1. Use `cargo instruments` (macOS) or `valgrind --tool=massif` (Linux)
2. Profile during indexing of largest projects (Kubernetes, numpy)
3. Identify peak memory usage and allocation hotspots
4. Document baseline memory requirements

### Error Rate Analysis
Implement detailed error tracking:
- Parse errors: tree-sitter failures
- Extraction errors: symbol or import extraction failures
- Database errors: insertion or query failures
- Log full context for debugging (file path, line number, error message)

### Validation Report
Generate `crates/maproom/docs/validation_results.md` with:
- Executive summary of test results
- Per-language statistics (files processed, error rates, performance)
- Per-project statistics
- Memory profiling results
- Performance benchmarks
- List of any issues discovered with links to GitHub issues

## Dependencies
- LANG_PARSE-3004 (all languages integrated and tested)
- All Phase 3 tickets must be complete (Python, Rust, Go integration)

## Risk Assessment
- **Risk**: Real-world projects may contain edge cases not covered by unit tests
  - **Mitigation**: Log all failures with full context for investigation; create follow-up tickets for any systemic issues

- **Risk**: Large project downloads may be slow or unreliable in CI environments
  - **Mitigation**: Cache test fixtures; use shallow clones; provide option to skip download if fixtures exist

- **Risk**: Performance targets may not be met on first attempt
  - **Mitigation**: Identify bottlenecks through profiling; create follow-up optimization tickets (LANG_PARSE-4002)

- **Risk**: Memory usage may exceed acceptable limits for large repositories
  - **Mitigation**: Profile memory usage; implement streaming or chunking if needed; document minimum system requirements

- **Risk**: Different project structures may expose parser bugs
  - **Mitigation**: Comprehensive error logging; create regression tests for any bugs found

## Files/Packages Affected
- `crates/maproom/tests/validation/large_scale_test.rs` (new file)
- `crates/maproom/tests/fixtures/` (new directory with test project metadata)
- `crates/maproom/tests/fixtures/projects.json` (new file - list of projects to test)
- `crates/maproom/docs/validation_results.md` (new file - generated report)
- `crates/maproom/benches/` (new directory for performance benchmarks)
- `crates/maproom/benches/multi_language_bench.rs` (new file)
- `crates/maproom/Cargo.toml` (add criterion or other benchmark dependencies)
- `.github/workflows/validation.yml` (new CI workflow for large-scale testing)
