# Fixture: rust_ambiguous (F-B — ambiguity never guesses)

`run()` in `src/caller.rs` calls `multiply()`, which is defined in BOTH
`src/alpha.rs` and `src/beta.rs`. All three files share `src/`, so the same-directory
tiebreak also cannot disambiguate (two candidates share the caller's directory).

## Expected edges after `scan`

- **ZERO** `calls` edges from `run` to any `multiply` (spec B3: never guess).

The precision-first policy drops the reference and counts it in a `debug!` summary.
