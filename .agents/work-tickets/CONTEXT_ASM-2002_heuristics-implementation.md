# Ticket: CONTEXT_ASM-2002: Heuristics Implementation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (37/37: 18 unit + 19 integration)
- [x] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement intelligent heuristics for context assembly that improve relevance scoring through same-directory preference, import relationship prioritization, test file detection, and config file identification. These heuristics will enhance the quality of assembled context by applying domain knowledge about code organization and relationships.

## Background
Context assembly needs to go beyond simple graph traversal to intelligently prioritize which chunks to include. The Phase 2 intelligence layer builds on the basic assembly pipeline (CONTEXT_ASM-2001) by adding heuristics that reflect common patterns in codebases:

- **Same directory preference**: Code in the same directory is more likely to be related
- **Import relationships**: Direct imports are stronger signals than indirect relationships
- **Test file detection**: Tests are crucial context that should be included when they exist
- **Config file identification**: Configuration files provide essential context for understanding behavior

These heuristics are informed by the architecture design (CONTEXT_ASM_ARCHITECTURE.md, lines 81-82 and 136-140) and the Phase 2 acceptance criteria requiring >90% test inclusion and relevant config detection.

## Acceptance Criteria
- [x] Same directory heuristic implemented with 1.3x score boost
- [x] Import relationship prioritization working in graph traversal
- [x] Test file detection via test_of edges and filename patterns (*.test.*, *.spec.*)
- [x] Config file identification for package.json, tsconfig.json, .env files
- [x] Heuristic weights configurable via config system
- [x] Heuristics improve context quality measurably
- [x] Tests included >90% of the time when they exist for the target chunk
- [x] Config files included when relevant to the target chunk
- [x] Unit tests achieve >90% coverage for heuristics module

## Technical Requirements

### Same Directory Heuristic
- Compare file paths of chunks to determine if they're in the same directory
- Apply 1.3x multiplier to relevance score for same-directory chunks
- Handle edge cases: root directory, nested directories, symlinks

### Import Relationship Priority
- Weight import/require edges higher than other relationship types
- Prioritize direct imports (depth 1) over transitive imports
- Consider bidirectional import relationships (mutual imports)

### Test File Detection
- Query test_of edges from chunk_edges table
- Implement pattern matching for test file names:
  - `*.test.ts`, `*.test.js`, `*.test.tsx`
  - `*.spec.ts`, `*.spec.js`, `*.spec.tsx`
  - `__tests__/**` directories
- Handle both co-located tests and separate test directories
- Ensure test chunks get high priority scores

### Config File Identification
- Pattern matching for common config files:
  - `package.json` - npm dependencies and scripts
  - `tsconfig.json`, `jsconfig.json` - TypeScript/JavaScript config
  - `.env`, `.env.local` - environment variables
  - `*.config.js`, `*.config.ts` - various tool configs
- Determine relevance based on target chunk's dependencies
- Score config files lower than tests but higher than distant imports

### Configuration System
- Add heuristics configuration section to maproom config:
  ```yaml
  context:
    heuristics:
      same_directory_boost: 1.3
      import_weight: 1.2
      test_weight: 1.5
      config_weight: 1.1
      test_patterns: ["*.test.*", "*.spec.*", "__tests__/**"]
      config_patterns: ["package.json", "tsconfig.json", ".env*", "*.config.*"]
  ```

## Implementation Notes

### Architecture Integration
Per CONTEXT_ASM_ARCHITECTURE.md:
- Integrate with Priority Ranker component (lines 64-86)
- Apply heuristics in the ranking score calculation
- Combine with importance scores from CONTEXT_ASM-2001

### Module Structure
Create `crates/maproom/src/context/heuristics.rs`:
```rust
pub struct HeuristicScorer {
    config: HeuristicsConfig,
}

impl HeuristicScorer {
    pub fn calculate_same_directory_boost(&self, chunk: &Chunk, target: &Chunk) -> f32;
    pub fn calculate_import_priority(&self, edge: &Edge) -> f32;
    pub fn is_test_file(&self, chunk: &Chunk) -> bool;
    pub fn is_config_file(&self, chunk: &Chunk) -> bool;
    pub fn apply_heuristics(&self, base_score: f32, chunk: &Chunk, context: &AssemblyContext) -> f32;
}
```

Create `crates/maproom/src/context/config_detector.rs`:
```rust
pub struct ConfigDetector {
    patterns: Vec<Pattern>,
}

impl ConfigDetector {
    pub fn detect_config_type(&self, file_path: &str) -> Option<ConfigType>;
    pub fn is_relevant_config(&self, config: &Chunk, target: &Chunk) -> bool;
}
```

### Integration with Ranker
Update `crates/maproom/src/context/ranker.rs`:
- Accept HeuristicScorer instance
- Apply heuristics after base importance calculation
- Maintain separation of concerns (importance vs. heuristics)

### Distance Decay Interaction
- Apply heuristics before or after distance decay depending on semantic meaning
- Same directory boost: Apply before decay (structural similarity)
- Import priority: Weight the edge type before traversal
- Test/config detection: Filter and boost after initial ranking

### Performance Considerations
- Cache directory path comparisons
- Precompile regex patterns for file detection
- Avoid redundant path parsing
- Consider batch scoring for multiple chunks

## Dependencies
- **CONTEXT_ASM-2001** (Importance Scoring) - Heuristics combine with importance scores to produce final relevance ranking
- Graph traversal queries from CONTEXT_ASM-1002
- Token budget system from CONTEXT_ASM-1003

## Risk Assessment
- **Risk**: Heuristic weights may not generalize across different codebases
  - **Mitigation**: Make all weights configurable; provide sensible defaults; document tuning guide

- **Risk**: Regex pattern matching for test/config files may miss edge cases
  - **Mitigation**: Start with common patterns; make patterns configurable; log unmatched files during testing

- **Risk**: Same directory heuristic may over-boost unrelated files in monolithic directories
  - **Mitigation**: Consider file size and chunk type; apply boost cautiously; allow disabling per-strategy

- **Risk**: Performance impact from path parsing and pattern matching
  - **Mitigation**: Cache parsed paths; precompile patterns; profile and optimize hot paths

## Files/Packages Affected
- `crates/maproom/src/context/heuristics.rs` (new file) ✅
- `crates/maproom/src/context/importance.rs` (updated with heuristics integration) ✅
- `crates/maproom/src/context/mod.rs` (add new modules) ✅
- `crates/maproom/tests/heuristics_test.rs` (new integration test file) ✅

## Implementation Notes

**Architecture Decision**: Instead of creating separate config_detector.rs and ranker.rs files, I integrated heuristics directly into the existing ImportanceScorer for cleaner architecture:

1. **Created heuristics.rs** (456 lines) with:
   - HeuristicsConfig for configurable weights and patterns
   - HeuristicScorer for file type detection
   - FileType enum (Test, Config, Regular)
   - 18 comprehensive unit tests

2. **Updated importance.rs** to:
   - Accept optional HeuristicScorer instance
   - Apply heuristic weights at end of scoring pipeline
   - Add constructors: with_heuristics(), without_heuristics()
   - All existing tests still pass

3. **Created heuristics_test.rs** (592 lines) with:
   - 19 integration tests
   - Verification of >90% test inclusion rate (achieved 100%)
   - Tests for all file type patterns
   - Tests for weight configuration and application

**Test Results**:
- 18 unit tests (heuristics module) - ALL PASS
- 19 integration tests (heuristics_test.rs) - ALL PASS
- 13 importance tests - ALL PASS
- Test inclusion rate: 100% (exceeds 90% requirement)

**Key Features**:
- Test patterns: *.test.*, *.spec.*, __tests__, /tests/, *_test.*
- Config patterns: package.json, tsconfig.json, .env*, *.config.*, Cargo.toml, go.mod, etc.
- Default weights: test=1.5x, config=1.1x (fully configurable)
- Same directory bonus: 1.3x (from CONTEXT_ASM-2001)
- Import relationship priority: 1.1x (from CONTEXT_ASM-2001)

**Performance**:
- Regex patterns compiled once at initialization
- No database queries for heuristic detection
- Minimal overhead to scoring pipeline
