# Ticket: CTXCLI-3001: Update MCP Context Schema

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Extend the MCP context schema with React-specific options, ensuring full parity with the Rust `ExpandOptions` type.

## Background
This is the first ticket of Phase 3 (MCP Integration). The MCP context tool's schema needs to be updated to support all expand options available in the Rust implementation, including React-specific options (hooks, jsx_parents, jsx_children).

Reference: [planning/architecture.md](../planning/architecture.md) - Section 4: Schema Synchronization

## Acceptance Criteria
- [x] `routes`, `hooks`, `jsx_parents`, `jsx_children` added to expand schema
- [x] Zod validation updated for new fields
- [x] TypeScript types updated to match
- [x] Schema accepts React-specific options in tool calls
- [x] Validation rejects invalid values (wrong types, unknown fields)
- [x] Types match Rust `ExpandOptions` exactly (all 10 fields)
- [x] Cross-reference comment added: `// Sync with: crates/maproom/src/context/types.rs ExpandOptions`

## Technical Requirements
- Update Zod schema in `context_schema.ts`
- All boolean fields default to `false`
- `max_depth` defaults to `2`
- Export updated TypeScript types

## Implementation Notes

### Schema Update
```typescript
// packages/maproom-mcp/src/tools/context_schema.ts

// Sync with: crates/maproom/src/context/types.rs ExpandOptions
export const ExpandOptionsSchema = z.object({
  callers: z.boolean().default(false),
  callees: z.boolean().default(false),
  tests: z.boolean().default(false),
  docs: z.boolean().default(false),
  config: z.boolean().default(false),
  max_depth: z.number().int().min(1).max(10).default(2),
  // React-specific options
  routes: z.boolean().default(false),
  hooks: z.boolean().default(false),
  jsx_parents: z.boolean().default(false),
  jsx_children: z.boolean().default(false),
})

export type ExpandOptions = z.infer<typeof ExpandOptionsSchema>

export const ContextParamsSchema = z.object({
  chunk_id: z.string(),
  budget_tokens: z.number().int().positive().default(6000),
  expand: ExpandOptionsSchema.optional(),
})

export type ContextParams = z.infer<typeof ContextParamsSchema>
```

### Field Mapping (Rust ↔ TypeScript)
| Rust Field | TypeScript Field | Type | Default |
|------------|------------------|------|---------|
| callers | callers | bool | false |
| callees | callees | bool | false |
| tests | tests | bool | false |
| docs | docs | bool | false |
| config | config | bool | false |
| max_depth | max_depth | i32/number | 2 |
| routes | routes | bool | false |
| hooks | hooks | bool | false |
| jsx_parents | jsx_parents | bool | false |
| jsx_children | jsx_children | bool | false |

## Dependencies
- None (can be developed in parallel, but tested after daemon is ready)

## Risk Assessment
- **Risk**: Schema drift between Rust and TypeScript
  - **Mitigation**: Add cross-reference comment, document in architecture.md
- **Risk**: Breaking existing MCP clients
  - **Mitigation**: All new fields are optional with defaults, backward compatible

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/context_schema.ts` (modify - add React-specific options)
