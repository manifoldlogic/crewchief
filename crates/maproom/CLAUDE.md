# Maproom Indexer (Rust)

## TypeScript Synchronization

| Rust (this crate) | TypeScript (daemon-client) |
|-------------------|---------------------------|
| `src/daemon/types.rs::SearchParams` | `src/client.ts::SearchParams` |
| `src/daemon/types.rs::ContextParams` | `src/client.ts::ContextParams` |
| `src/context/types.rs::ContextBundle` | `src/client.ts::RustContextBundle` |

**Rust is the source of truth.** Full workflow: `.claude/docs/type-sync-workflow.md`

## Exit Codes

All commands follow a consistent contract:

- **0**: Success (with or without results)
- **1**: Runtime error (transient failures, database errors, network issues)
- **2**: Configuration error (missing env vars, invalid provider, missing sqlite-vec). Note: clap also uses 2 for CLI parse errors.

Agents use this: exit 0 → process results, exit 1 → report/retry, exit 2 → fall back (e.g., FTS instead of vector).

## Binary Output

Built to `../../packages/cli/bin/<platform>/maproom`:
- Platforms: darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64

## Pitfalls

- **sqlite-vec silent degradation**: If sqlite-vec extension fails to load, vector search silently returns no results. FTS still works. Check exit code 2.
- **`.maproomignore` no hot-reload**: Changes require restarting the watcher or running a new scan. Patterns loaded once at startup.
- **Git polling, not filesystem events**: File watching uses `git status --porcelain` polling (default 3s). Trades instant detection for 2-5s latency to avoid EMFILE errors on large repos.
- **No negation in `.maproomignore`**: Unlike `.gitignore`, there is no `!pattern` syntax. All patterns are exclusions only.
- **Fail-fast patterns**: Invalid glob patterns in `.maproomignore` cause scan/watch startup to fail immediately.
- **Cross-file edge inbound staleness (v1)**: `calls`/`test_of` edges are resolved cross-file in a per-worktree post-pass. Re-indexing a callee file B alone (via `upsert_files`/watch) deletes B's edges — including inbound `A → B` edges — and single-file recomputation cannot restore them (A is not re-read). This is deliberate v1 policy: inbound edges regenerate on the next scan of A or a full rescan (pinned by `test_inbound_edge_staleness_is_deliberate`). Full `scan` is always internally consistent.
- **Cross-file resolution never guesses (ambiguity policy)**: a call resolves cross-file only when exactly one same-language callable chunk in another file matches the name (or exactly one of several shares the caller's directory). Otherwise the reference is dropped and counted in a `debug!` summary — no id-order or insertion-order tiebreak. This keeps the accuracy suite's colliding-symbol precision gate (≥ 0.85) intact.

## Conventions

- **FTS-first default**: `maproom scan` defaults to FTS-only (no embeddings). Use `--generate-embeddings` to opt in to vector search. Vector/hybrid require a configured embedding provider.
- **Embedding dimension auto-inference**: Known Ollama models (`mxbai-embed-large` → 1024, `nomic-embed-text` → 768) are inferred automatically. Override with `MAPROOM_EMBEDDING_DIMENSION`. Bedrock infers too (Titan v2 → 1024, Titan v1 → 1536, Cohere v3 → 1024), but an **unrecognized** Bedrock model id is a hard config error rather than a warning-plus-default: those are usually provisioned-throughput ARNs of unknown width, and a wrong width builds an index that succeeds and then silently returns nothing.
- **Bedrock uses hand-rolled SigV4, not the AWS SDK** (`src/embedding/aws/`): `aws-sdk-bedrockruntime` 1.95 declares MSRV 1.85, but its transitive `aws-sdk-{sso,ssooidc,sts}` require 1.88 — which breaks the CI-enforced MSRV in `.github/workflows/msrv.yml`. Holding MSRV would mean pinning ~6 AWS crates at mid-2025 versions (no security updates); the SDK also adds ~204 crates and a second TLS stack (rustls + `aws-lc-rs`, needing cmake) to a binary that already links vendored OpenSSL and cross-compiles to four targets. Same trade as the hand-rolled Postgres migrator. `hmac` is the only added dependency. Signing is pinned against AWS's published test vectors **and** an independent Python reference implementation in `tests/fixtures/sigv4_reference.py` — regenerate the fixture if the canonical-request construction ever changes.
- **Query vs document embeddings**: `EmbeddingProvider::embed_query` defaults to `embed`, and `distinguishes_queries()` defaults to `false` — so Ollama, OpenAI, Google, and Bedrock-with-Titan are entirely unaffected. Cohere on Bedrock is asymmetric (`search_query` vs `search_document`) and overrides both. `EmbeddingService::embed_query` only namespaces the cache key (`"query\u{1f}" + text`) when the provider says it distinguishes; otherwise it is byte-for-byte the old `embed_text` path, cache entry included. All four query call sites (`search/query_processor.rs`, two in `main.rs`, two in `daemon/mod.rs`) use it.
- **Bedrock batch size is per-family, not configurable upward**: Titan's `InvokeModel` payload has a single `inputText` field, so `max_batch_size()` is 1 and a scan fans out to one request per chunk under the concurrency semaphore. Cohere takes 96. `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE` is clamped down to the family limit — raising it cannot make Titan batch.
- **Multiple vector tables**: sqlite-vec requires fixed dimensions at table creation. Separate tables per dimension (`vec_code`, `vec_code_1024`, `vec_code_768`).
- **Postgres vector storage is HNSW-cosine**: the content-addressed `code_embeddings` pool stores each embedding in a per-dimension typed column (`embedding_768`/`embedding_1024`/`embedding_1536`), each backed by a **partial HNSW `vector_cosine_ops` index** (migration `0004`). Postgres vector search is cosine (`<=>`), not SQLite's L2 (`<->`); similarity is `1 - cosine_distance` (`cosine_distance_to_similarity`, PG-local — SQLite keeps its L2 `1/(1+d)`). Parity is on membership + ordering, not raw scores. HNSW recall is tunable via `MAPROOM_SEARCH_INDEX_HNSW_EF_SEARCH` (default 40, clamped up to the query's `k`) with no index rebuild. The KNN runs under the normal `statement_timeout` (the ANN index bounds the scan), NOT the old `SET LOCAL statement_timeout = 0` workaround.
- **First-connect index build is locking**: the migration runner applies `0004` in one transaction, so the HNSW indexes are built with a **locking** (non-`CONCURRENT`) build — Postgres forbids `CREATE INDEX CONCURRENTLY` inside a transaction. Fast on a fresh/small pool; a large pre-existing pool will block writers during the first-connect migration that adds the indexes.
- **PG vector search is approximate, not exactly reproducible**: HNSW is an approximate index, and the KNN uses `ORDER BY distance ASC` with NO secondary tiebreak on purpose — adding `, c.id` makes Postgres fall back to a full seq-scan + sort (~150× slower, EXPLAIN-verified), defeating the index. Because the pool is content-addressed, one embedding fans out over the JOIN to every same-content chunk (shared `blob_sha`), which share an exact `distance`; their relative order and membership at the `k` boundary are not pinned run-to-run. The tied rows are identical content, so this is a benign accepted tradeoff — recall `ef_search` also trades recall for speed. SQLite's exact-KNN path is unaffected. Vector scores are clamped to `[0, 1]` (cosine of opposed vectors would be negative) to honor the `--threshold` contract.
- **Backend migration + data minimization** (F47/F48): `maproom db export --out <file>` writes a portable, versioned NDJSON artifact from a SQLite index (runs on the shipped SQLite-only binary); `maproom db import --in <file> --to <postgres-url>` loads it into Postgres, remapping ids by natural key and moving the content-addressed embedding pool **without re-embedding** (`--features postgres` only, since using the PG backend already requires that build). `maproom db minimize --confirm` enables a sticky, one-way **don't-store-content** mode: future scans/imports persist only blob hashes, embeddings, line ranges, and symbol names — never raw code content (preview/signature/docstring/FTS source) — and it purges content already at rest. On SQLite this disables FTS (vector-only); on Postgres keyword search survives via the derived `tsvector` (not raw-content-backed). The marker is cached at connect and read by scan, incremental, and import at the central `insert_chunk` chokepoint.

## Known Limitations

These apply to the **SQLite backend** (the default). The optional **PostgreSQL** backend (build with `--features postgres`, select via a `postgres://` URL) supports concurrent multi-process writes and transport-level TLS. (Encryption at rest is a deployment concern, not something this crate provides — but `db minimize` / F48 is a content-minimization mode that keeps raw code out of the database entirely, distinct from encryption.)

- Single-user only, no multi-process concurrent writes — SQLite backend
- No database encryption — SQLite backend
- sqlite-vec extension must be compiled in (statically linked) — SQLite backend

## Versioning

The crate version lives in **one place**: `version` in `crates/maproom/Cargo.toml`.

- `maproom --version` derives it automatically via clap `#[command(version)]` → `CARGO_PKG_VERSION` — never hardcode a version string in source or docs.
- `daemon/protocol.rs::PROTOCOL_VERSION` is a **separate** wire-protocol version; bump it only on a protocol change, independently of the crate version.
- The npm packages (`@crewchief/cli`, etc.) version independently via `release-config.json`; they bundle the compiled binary but don't pin the crate's semver.
- Bump policy: this is a `0.x` crate, so a breaking public-API change uses a **minor** bump (`0.1.0 → 0.2.0`); additive/fixes use a patch bump.

## Benchmarking (F75)

The SQLite-vs-Postgres search benchmark runs ONE engineered corpus through both backends
and reports quality (P@k / recall@k / nDCG@k / MRR via `evaluation::metrics`) + latency:

```
# SQLite only (the benchmark is #[ignore]d — run it on demand):
cargo test -p maproom --test backend_benchmark backend_search_benchmark -- --ignored --nocapture
# Both backends:
MAPROOM_TEST_PG_URL=postgres://user@host/db \
  cargo test -p maproom --features postgres --test backend_benchmark backend_search_benchmark -- --ignored --nocapture
```

It is test-only (no production path) and asserts **tolerant** quality thresholds, never
exact-order equality (Postgres vector search is approximate — see the HNSW note above).
Ground truth is by natural key (`symbol_name`), never a raw chunk_id. The old
`tests/golden_test.rs` `execute_search_query` is a superseded `vec![]` mock — use this.
The instrument is ready for a larger/real corpus to locate the latency crossover.

## Docs

- Agent integration: `docs/agent-usage.md`
- Database architecture: `docs/architecture/DATABASE_ARCHITECTURE.md`
- Context assembly API: `docs/context_assembly_api.md` (relative to this crate)
- Vector search config: `docs/VECTOR_SEARCH_CONFIGURATION.md` (relative to this crate)
- Provider comparison: `docs/providers/comparison.md` (repo root)
- Migrations: `.claude/docs/migration-workflow.md`
