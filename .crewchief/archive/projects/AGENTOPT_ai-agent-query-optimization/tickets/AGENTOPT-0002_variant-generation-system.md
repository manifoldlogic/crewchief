# Ticket: AGENTOPT-0002: Build Variant Generation System

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a system for generating, storing, and mutating tool description variants. This includes initial manual variants and a genetic algorithm mutation engine for continuous improvement. The system will support both manual baseline variants and evolutionary mutations for data-driven optimization.

## Background
This ticket implements Phase 0, Step 2 from the AGENTOPT project plan (see planning/plan.md lines 419-427). The variant generation system is core to the data-driven optimization approach that enables competing tool description variants to be evolved through genetic algorithm-style mutations.

The system must support:
1. Manual variant creation with 4-5 initial baselines representing different communication styles
2. Genetic mutations from winners (crossover, amplification, reduction, reframing, specialization)
3. Variant metadata tracking including generation, parent relationships, and mutation type
4. Variant validation ensuring token counts stay within budget and conform to MCP schema

This establishes the foundation for the iterative optimization loop where variants can be tested against the test query set (AGENTOPT-0001) and continuously improved.

## Acceptance Criteria
- [x] Variant data structure implemented matching TypeScript interface in architecture.md lines 841-851
- [x] 5 initial manual variants created (detailed, simple, conversational, code-like, control)
- [x] Mutation engine implemented with 5 mutation types (crossover, amplification, reduction, reframing, specialization)
- [x] Variant storage system created using JSON files in variants/ directory
- [x] Variant validator ensures token count <600 and validates against MCP tool schema
- [x] All variants have genealogy tracking (generation number, parent_ids, mutation_type)

## Technical Requirements
- TypeScript variant interface with properties: id, name, description, tokens, generation, parent_ids, mutation_type, created_at
- Support for 5 mutation types: crossover (combine two variants), amplification (expand detail), reduction (simplify), reframing (change perspective), specialization (narrow scope)
- Token counting implementation using tiktoken library or Claude API
- MCP schema validation matching current maproom-mcp tool structure
- Variant versioning and genealogy tracking to enable evolutionary analysis
- Initial variants should range from 200-500 tokens with control baseline at ~350 tokens (current baseline)

## Implementation Notes
Create `packages/maproom-mcp/test/tool-description-optimization/` directory structure with:
- **types.ts** - TypeScript interfaces for variants (id, name, description, tokens, generation, parent_ids, mutation_type, created_at)
- **variants/** - Directory for storing variant JSON files (one per variant)
- **mutator.ts** - Mutation engine implementing 5 mutation types with proper genealogy tracking
- **validator.ts** - Validation logic for token counts and MCP schema compliance

The 5 initial manual variants should represent different communication approaches:
1. **Detailed** (~450 tokens) - Comprehensive, verbose description with all features
2. **Simple** (~220 tokens) - Minimal, concise core functionality only
3. **Conversational** (~350 tokens) - Friendly, approachable tone with examples
4. **Code-like** (~300 tokens) - Technical, function-signature style documentation
5. **Control** (~350 tokens) - Current baseline for comparison

Each mutation type should:
- Accept parent variant(s) as input
- Generate a modified description maintaining semantic meaning
- Increment generation number
- Track parent_ids and mutation_type
- Pass validation before storage

## Dependencies
- AGENTOPT-0001 (test query set, used for validating variant effectiveness)
- Current MCP tool description at packages/maproom-mcp/src/index.ts
- tiktoken library for token counting

## Risk Assessment
- **Risk**: Mutations produce invalid tool descriptions or break MCP schema
  - **Mitigation**: Strict validation before saving, comprehensive schema checks, unit tests for each mutation type
- **Risk**: Token count exceeds 600 token budget
  - **Mitigation**: Automated token validation on all variants, rejection if exceeds limit, monitoring in validator
- **Risk**: Loss of genealogy information during mutations
  - **Mitigation**: Store complete parent_ids array and mutation_type for all variants, validation schema includes genealogy fields

## Files/Packages Affected
- packages/maproom-mcp/test/tool-description-optimization/ (create new directory)
- packages/maproom-mcp/test/tool-description-optimization/types.ts (create)
- packages/maproom-mcp/test/tool-description-optimization/variants/ (create, store variant JSON files)
- packages/maproom-mcp/test/tool-description-optimization/mutator.ts (create)
- packages/maproom-mcp/test/tool-description-optimization/validator.ts (create)

## Planning References
- Plan: `.crewchief/projects/AGENTOPT_ai-agent-query-optimization/planning/plan.md` (lines 419-427)
- Architecture: `.crewchief/projects/AGENTOPT_ai-agent-query-optimization/planning/architecture.md` (lines 819-851)
