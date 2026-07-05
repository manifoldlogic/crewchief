# Fixture: trilang (edge-depth DoD §2 — tri-language E2E)

One worktree mixing Rust, TypeScript, and Python at the root (Python `pkg/` must
sit at the scan root so `from pkg.mod import ...` resolves module-path-scoped).

## Cross-file `calls` edges (src.file != dst.file)

| lang | src | dst |
|---|---|---|
| rust | `r_caller` (caller.rs) | `r_helper` (helper.rs) |
| ts | `t_main` (main.ts) | `t_util` (util.ts) |
| py | `p_caller` (app.py) | `p_helper` (pkg/mod.py) |

## Scoped Python `imports` edges

- `app.py` `__imports__` → `pkg/mod.py` `p_helper`
- `test_app.py` `__imports__` → `app.py` `p_caller`

## `test_of` per language

| lang | test src | dst |
|---|---|---|
| rust | `test_r_caller` (cfg(test)) | `r_caller` |
| ts | `test_t_main` (main.test.ts) | `t_main` |
| py | `test_p_caller` (`test_` prefix) | `p_caller` |

## Context bundle (Wave-1 defaults) for `p_caller`

`DefaultAssemblyStrategy::assemble(p_caller)` yields all four role types:
- **caller** — `p_driver` (non-test caller)
- **callee** — `p_helper`
- **import** — `test_app.py` imports `p_caller`
- **test** — `test_p_caller`
