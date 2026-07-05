# Fixture: rust_test_of (F-B — test_of derivation)

`src/lib.rs` defines `alpha`, a non-test `beta` that calls `alpha`, and a
`#[cfg(test)]` `test_alpha` that also calls `alpha`.

## Expected edges after `scan`

| src | dst | type |
|---|---|---|
| `test_alpha` | `alpha` | `calls` |
| `test_alpha` | `alpha` | `test_of` |
| `beta` | `alpha` | `calls` |

- `test_of` is derived only where the caller is a test (spec B7/B8):
  `test_alpha` (via the `test_` prefix), never `beta`.
- `find_test_files(alpha)` returns `test_alpha`.
- `find_callers(alpha)` returns `test_alpha` and `beta` via `calls` edges only —
  `test_of` never leaks into callers (purity).
