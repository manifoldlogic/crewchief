# Fixture: python_calls (F-D — Python call extraction)

`src/app.py` exercises direct function calls, `self.method()` calls, and decoys
that must NOT produce edges (builtins, str methods, class instantiation).

## Ground truth (same-file `calls` edges)

| src | dst |
|---|---|
| `process` | `validate` |
| `process` | `transform` |
| `run` | `load` |
| `run` | `process` |

## Must NOT produce edges

- `bool(...)`, `print(...)`, `data.upper()` — builtins / no chunk.
- `Pipeline(src)` — `Pipeline` is a class, not a callable kind.

Precision gate: ≥ 0.85 (the F-D enablement gate). If not met, `py` ships behind
the `supports_call_extraction` predicate as NOT enabled.
