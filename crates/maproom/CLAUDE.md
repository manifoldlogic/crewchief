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

## Conventions

- **FTS-first default**: `maproom scan` defaults to FTS-only (no embeddings). Use `--generate-embeddings` to opt in to vector search. Vector/hybrid require a configured embedding provider.
- **Embedding dimension auto-inference**: Known Ollama models (`mxbai-embed-large` → 1024, `nomic-embed-text` → 768) are inferred automatically. Override with `MAPROOM_EMBEDDING_DIMENSION`.
- **Multiple vector tables**: sqlite-vec requires fixed dimensions at table creation. Separate tables per dimension (`vec_code`, `vec_code_1024`, `vec_code_768`).

## Known Limitations

These apply to the **SQLite backend** (the default). The optional **PostgreSQL** backend (build with `--features postgres`, select via a `postgres://` URL) supports concurrent multi-process writes and transport-level TLS. (Encryption at rest is a deployment concern, not something this crate provides.)

- Single-user only, no multi-process concurrent writes — SQLite backend
- No database encryption — SQLite backend
- sqlite-vec extension must be compiled in (statically linked) — SQLite backend

## Versioning

The crate version lives in **one place**: `version` in `crates/maproom/Cargo.toml`.

- `maproom --version` derives it automatically via clap `#[command(version)]` → `CARGO_PKG_VERSION` — never hardcode a version string in source or docs.
- `daemon/protocol.rs::PROTOCOL_VERSION` is a **separate** wire-protocol version; bump it only on a protocol change, independently of the crate version.
- The npm packages (`@crewchief/cli`, etc.) version independently via `release-config.json`; they bundle the compiled binary but don't pin the crate's semver.
- Bump policy: this is a `0.x` crate, so a breaking public-API change uses a **minor** bump (`0.1.0 → 0.2.0`); additive/fixes use a patch bump.

## Docs

- Agent integration: `docs/agent-usage.md`
- Database architecture: `docs/architecture/DATABASE_ARCHITECTURE.md`
- Context assembly API: `docs/context_assembly_api.md` (relative to this crate)
- Vector search config: `docs/VECTOR_SEARCH_CONFIGURATION.md` (relative to this crate)
- Provider comparison: `docs/providers/comparison.md` (repo root)
- Migrations: `.claude/docs/migration-workflow.md`
