# Fixture: python_imports (F-C — Python import scoping)

Exercises the spec §6 Gherkin: two files import the same symbol from a shared
module; import edges must be per-file-scoped and decoy/external-proof.

## Layout

- `pkg/utils.py` — the sole legitimate `helper` definition.
- `pkg/__init__.py` — package marker, defines no symbols.
- `a.py` — `from pkg.utils import helper`.
- `b.py` — `from pkg.utils import helper`, PLUS a local `def helper()` decoy
  (higher chunk id), PLUS `import os` and `import some_external_lib` (external).

## Expected edges after `scan`

| src (`__imports__` chunk) | dst | type |
|---|---|---|
| `a.py` `__imports__` | `pkg/utils.py` `helper` | `imports` |
| `b.py` `__imports__` | `pkg/utils.py` `helper` | `imports` |

Exactly **two** `imports` edges, with **distinct** src chunks, both pointing at
`pkg/utils.py`'s `helper` — never `b.py`'s local decoy.

## Must NOT produce

- Any edge whose dst is `b.py`'s local `helper` (the max-id decoy).
- Any edge referencing `os` or `some_external_lib` (external — not indexed).
- Any change to the above on a second scan (idempotence).
