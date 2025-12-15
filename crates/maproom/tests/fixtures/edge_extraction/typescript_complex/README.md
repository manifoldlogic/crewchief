# TypeScript Complex - Test Fixture

Complex TypeScript patterns including nested calls, higher-order functions, and arrow functions.

## Structure

```
src/
└── patterns.ts    # Various call patterns (nested, HOF, arrows)
```

## Ground Truth (Expected Edges)

### Phase 1: Same-File Edges Only

**patterns.ts:**
- `outer → inner` (line 3) - outer calls inner
- `inner → helper` (line 7) - inner calls helper
- `process → double` (line 25) - arrow function body calls double
- `orchestrate → outer` (line 30) - orchestrate calls outer
- `orchestrate → inner` (line 31) - orchestrate calls inner
- `orchestrate → helper` (line 32) - orchestrate calls helper
- `orchestrate → map` (line 33) - orchestrate calls map
- `<top-level> → orchestrate` (line 37) - top-level calls orchestrate [NOT DETECTED - top-level not extracted as chunk]

## Expected Edge Count

- **Same-file edges (Phase 1)**: 7 total
  - Nested calls: outer → inner, inner → helper (2 edges)
  - Arrow function: process → double (1 edge)
  - Multiple calls: orchestrate → outer, inner, helper, map (4 edges)
  - Top-level calls not detected (0 edges)

## Edge Cases

**Higher-order function arguments:**
- `map(double, [1, 2, 3])` - double is passed as argument
- May or may not create `map → double` edge depending on implementation
- This is acceptable for MVP (Phase 1 focuses on direct calls)

**Arrow function detection:**
- `const process = (x: number) => { return double(x); }`
- Arrow functions may be detected as anonymous chunks or function chunks
- Edge detection should work for arrow function bodies

## Test Coverage

This fixture tests:
- Nested function call chains
- Multiple calls within a single function
- Arrow function call detection
- Higher-order function patterns
- Top-level function invocation
- Edge cases for function-as-argument patterns
