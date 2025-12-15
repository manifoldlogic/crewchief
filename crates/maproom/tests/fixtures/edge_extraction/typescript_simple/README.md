# TypeScript Simple - Test Fixture

Simple TypeScript call chain for validating basic edge extraction.

## Structure

```
src/
├── utils.ts    # Utility functions with internal calls
└── main.ts     # Main entry point with cross-file import
```

## Ground Truth (Expected Edges)

### Phase 1: Same-File Edges Only

**utils.ts:**
- `calculate → add` (line 8) - calculate calls add
- `calculate → multiply` (line 9) - calculate calls multiply

**main.ts:**
- `<top-level> → main` (line 8) - top-level execution calls main [NOT DETECTED - top-level not extracted as chunk]

### Phase 2: Cross-File Edges (Not Implemented Yet)

**main.ts:**
- `main → calculate` (line 4) - main calls calculate from utils

## Expected Edge Count

- **Same-file edges (Phase 1)**: 2 total
  - 2 in utils.ts (calculate → add, calculate → multiply)
  - 0 in main.ts (top-level calls not detected)
- **Cross-file edges (Phase 2)**: 1 (main → calculate)

## Test Coverage

This fixture tests:
- Simple function call detection
- Multiple calls within a single function
- Top-level function invocation
- Export syntax handling
