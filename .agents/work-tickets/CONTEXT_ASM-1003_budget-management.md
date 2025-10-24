# Ticket: CONTEXT_ASM-1003: Token Budget Management

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement token budget management system for Context Assembly, including budget allocation, truncation logic, priority queue, and graceful overflow handling. This ensures that assembled context never exceeds the specified token budget while maximizing the value of included content.

## Background
The Context Assembly system must operate within strict token budgets (typically 8K-32K tokens for Claude models). Without proper budget management, the assembler could produce context that exceeds model limits or inefficiently allocates available tokens. This ticket implements the core budget management infrastructure that:

1. Allocates tokens across different context categories (primary chunk 40%, tests 20%, callers 15%, callees 15%, config 10%)
2. Reserves and tracks token usage during assembly
3. Implements intelligent truncation when content exceeds allocated budget
4. Uses a priority queue to select the most valuable chunks when space is limited
5. Handles budget overflow gracefully by skipping lowest priority items

This is a critical component of Phase 1 (Week 2, Task 1) from the CONTEXT_ASM planning document.

## Acceptance Criteria
- [ ] Budget never exceeded - total assembled context always stays within specified token limit
- [ ] Intelligent truncation working - content is truncated preserving signatures and docstrings
- [ ] Priority queue implemented - chunks are selected and ordered by relevance/importance
- [ ] Overflow handled gracefully - when budget is tight, lowest priority items are skipped without errors
- [ ] Budget allocation matches architecture spec - 40% primary, 20% tests, 15% callers, 15% callees, 10% config
- [ ] Token reservation system functional - components can reserve tokens before adding content
- [ ] Unit tests pass with 100% coverage for budget manager, truncation, and priority queue

## Technical Requirements
- Implement `TokenBudgetManager` with reserve() and allocate() methods as specified in architecture
- Create budget allocation structure: primary 40%, tests 20%, callers 15%, callees 15%, config 10%
- Implement token reservation system using Map<string, number> to track category usage
- Build priority queue for chunk selection based on relevance scores and category importance
- Create intelligent truncation logic that:
  - Preserves function/class signatures
  - Preserves docstrings and leading comments
  - Truncates function bodies when necessary
  - Adds truncation markers to indicate omitted content
- Handle overflow scenarios by skipping chunks rather than exceeding budget
- Track actual token usage vs allocated budget for monitoring and optimization
- Integrate with TokenCounter from CONTEXT_ASM-1001 for accurate token measurement

## Implementation Notes

### Budget Manager (`crates/maproom/src/context/budget.rs`)
```rust
pub struct TokenBudgetManager {
    budget: usize,
    used: usize,
    reserved: HashMap<String, usize>,
}

impl TokenBudgetManager {
    pub fn new(budget: usize) -> Self;
    pub fn reserve(&mut self, category: &str, tokens: usize) -> bool;
    pub fn release(&mut self, category: &str);
    pub fn allocate(&self) -> BudgetAllocation;
    pub fn remaining(&self) -> usize;
    pub fn usage_stats(&self) -> UsageStats;
}

pub struct BudgetAllocation {
    pub primary: usize,    // 40%
    pub tests: usize,      // 20%
    pub callers: usize,    // 15%
    pub callees: usize,    // 15%
    pub config: usize,     // 10%
}
```

### Truncation Logic (`crates/maproom/src/context/truncate.rs`)
- Parse code chunks to identify structure (signature, docstring, body)
- Preserve essential parts (signatures, docstrings) up to reasonable limits
- Truncate body content when budget is tight
- Add markers like `// ... [truncated N lines] ...` to indicate omissions
- Handle edge cases: very small budgets, already-truncated content

### Priority Queue (`crates/maproom/src/context/priority_queue.rs`)
```rust
pub struct PriorityQueue<T> {
    items: BinaryHeap<PriorityItem<T>>,
}

pub struct PriorityItem<T> {
    priority: f64,  // Higher = more important
    category: Category,
    item: T,
}
```
- Order chunks by priority score (from search relevance + category weight)
- Support peek, pop, and drain operations
- Provide iterator for processing chunks in priority order

### Overflow Handling
- When adding a chunk would exceed budget:
  1. Check if it's higher priority than any existing chunk
  2. If yes, remove lowest priority chunk(s) to make room
  3. If no, skip this chunk
  4. Log decisions for debugging
- Never throw errors on overflow - degrade gracefully

### Architecture Reference
See CONTEXT_ASM_ARCHITECTURE.md:
- Lines 89-111: Token Budget Manager component specification
- Lines 188-192: Content Formatter truncation approach

## Dependencies
- **CONTEXT_ASM-1001** (token counting) - Required for accurate token measurement
  - Must be able to count tokens for arbitrary text
  - Should support multiple encoding schemes (cl100k_base for Claude)

## Risk Assessment
- **Risk**: Truncation logic may break code syntax or readability
  - **Mitigation**: Use tree-sitter to parse code structure before truncating; preserve complete syntax elements; add comprehensive tests with edge cases

- **Risk**: Priority queue may not select the most useful chunks
  - **Mitigation**: Start with simple relevance-based priority; iterate based on real-world usage; make priority calculation configurable

- **Risk**: Budget allocation percentages may not suit all use cases
  - **Mitigation**: Make percentages configurable in future iteration; use architecture-specified defaults as sensible starting point

- **Risk**: Token counting accuracy may drift from actual model tokenization
  - **Mitigation**: Use same tokenizer (tiktoken) as Claude models; validate counts against actual API usage; add tolerance buffer

## Files/Packages Affected
- `crates/maproom/src/context/budget.rs` - NEW: Budget manager implementation
- `crates/maproom/src/context/truncate.rs` - NEW: Truncation logic
- `crates/maproom/src/context/priority_queue.rs` - NEW: Priority queue data structure
- `crates/maproom/src/context/mod.rs` - MODIFIED: Export new modules
- `crates/maproom/tests/context/budget_test.rs` - NEW: Budget manager unit tests
- `crates/maproom/tests/context/truncate_test.rs` - NEW: Truncation logic tests
- `crates/maproom/tests/context/priority_queue_test.rs` - NEW: Priority queue tests
- `crates/maproom/Cargo.toml` - MODIFIED: May need additional dependencies (e.g., priority queue crate)
