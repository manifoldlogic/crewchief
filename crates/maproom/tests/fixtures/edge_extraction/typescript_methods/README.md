# TypeScript Function Calls - Test Fixture

TypeScript functions with internal calls to validate function call detection.

**Note:** Originally designed for class methods, but converted to standalone functions because Phase 1 doesn't extract individual class methods as chunks. This still tests the same edge extraction logic for function calls with multiple levels of reuse.

## Structure

```
src/
└── calculator.ts    # Calculator functions with internal calls
```

## Ground Truth (Expected Edges)

### Phase 1: Same-File Edges Only

**calculator.ts:**
- `multiply → add` (line 13) - multiply calls add(a, a)
- `compute → add` (line 18) - compute calls add(5, 3)
- `compute → multiply` (line 19) - compute calls multiply(2, 4)
- `compute → subtract` (line 20) - compute calls subtract(x, y)
- `<top-level> → compute` (line 23) - top-level calls compute() [NOT DETECTED - top-level not extracted as chunk]

## Expected Edge Count

- **Same-file edges (Phase 1)**: 4 total
  - multiply → add (1 edge)
  - compute → add, multiply, subtract (3 edges)
  - Top-level calls not detected (0 edges)

## Test Coverage

This fixture tests:
- Function call detection
- Multiple calls within a single function
- Function reuse patterns (multiply uses add, compute uses all)
- Top-level function invocation
