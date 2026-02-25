# Type Synchronization Workflow

Rust is the source of truth for all RPC types. TypeScript must mirror Rust structs exactly.

## Sync Points

| Rust File | Rust Type | TypeScript File | TypeScript Type |
|-----------|-----------|-----------------|-----------------|
| `src/daemon/types.rs` | `SearchParams` | `daemon-client/src/client.ts` | `SearchParams` |
| `src/daemon/types.rs` | `ContextParams` | `daemon-client/src/client.ts` | `ContextParams` |
| `src/context/types.rs` | `ContextBundle` | `daemon-client/src/client.ts` | `RustContextBundle` |
| `src/search/errors.rs` | `ErrorType` | `daemon-client/src/types.ts` | `ErrorType` |
| `src/search/results.rs` | `PipelineStage` | `daemon-client/src/types.ts` | `PipelineStage` |

Both files contain `// Sync with:` comments linking to the corresponding source.

## Adding a New RPC Method

1. Add Rust handler in `crates/maproom/src/daemon/` (request/response structs in `types.rs`)
2. Add TypeScript types in `packages/daemon-client/src/client.ts` with sync comment
3. Add client method in `DaemonClient`
4. Test: `cd packages/daemon-client && pnpm test types.test.ts`

## Adding a New Error Type

1. **Rust** (`crates/maproom/src/search/errors.rs`): Add variant to `ErrorType` enum, add conversion in `from_pipeline_error()`, add 1-2 actionable suggestions
2. **TypeScript** (`packages/daemon-client/src/types.ts`): Add variant to `ErrorType` union type, update sync comment
3. **Validation** (`packages/daemon-client/src/types.test.ts`): Add variant to validation array
4. **Integration** (`crates/maproom/tests/daemon_error_serialization.rs`): Add serialization test

See also: `docs/runbooks/adding-error-types.md`

## Verification

```bash
cd packages/daemon-client && pnpm test types.test.ts  # Enum values match
cd crates/maproom && cargo test daemon_error           # Serialization roundtrip
```
