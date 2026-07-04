# Fixture: rust_crossfile (F-B — cross-file call resolution)

Two Rust files where one calls a function defined in the other.

## Layout

- `src/caller.rs` — `caller_a()` calls `helper_b()`.
- `src/helper.rs` — defines `helper_b()`.

## Expected edges after `scan`

| src | dst | type | note |
|---|---|---|---|
| `caller_a` (caller.rs) | `helper_b` (helper.rs) | `calls` | cross-file (`src.file != dst.file`) |

`find_callers(helper_b)` must return `caller_a`.

## Inbound staleness (v1 policy, spec B5)

After `upsert_files([src/helper.rs])`, the inbound `caller_a -> helper_b` edge is
deleted and NOT restored (helper.rs alone cannot recompute an edge whose source is
caller.rs). A full rescan restores it.
