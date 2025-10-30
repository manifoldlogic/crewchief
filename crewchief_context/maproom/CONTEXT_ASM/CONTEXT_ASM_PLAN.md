# CONTEXT_ASM Plan: Context Assembly Engine

## Project Overview
Build a budget-aware context assembly system that intelligently gathers relevant code context for LLMs, optimizing for relevance within token constraints.

## Phase 1: Core Assembly (Week 1-2)

### Week 1: Foundation
**Agent: mcp-context-engineer**

#### Tasks
1. **Basic Assembly Pipeline**
   - Implement chunk retrieval by ID
   - Add file content loading
   - Create token counting utility
   - Build basic assembly structure

2. **Relationship Queries**
   - Query for test files
   - Find callers/callees via edges
   - Implement graph traversal
   - Add depth limiting

**Acceptance Criteria:**
- [ ] Retrieve and format primary chunk
- [ ] Find related chunks via edges
- [ ] Count tokens accurately
- [ ] Stay within budget

### Week 2: Budget Management
**Agent: mcp-context-engineer**

#### Tasks
1. **Token Budget System**
   - Implement budget allocation
   - Add truncation logic
   - Create prioritization queue
   - Handle overflow gracefully

2. **Content Formatting**
   - Format chunks with metadata
   - Add role annotations
   - Include reason explanations
   - Generate summaries for large chunks

**Acceptance Criteria:**
- [ ] Budget never exceeded
- [ ] Intelligent truncation
- [ ] Clear role labels
- [ ] Useful summaries

## Phase 2: Intelligence Layer (Week 3-4)

### Week 3: Smart Selection
**Agent: mcp-context-engineer + database-engineer**

#### Tasks
1. **Importance Scoring**
   - Calculate chunk importance
   - Weight by relationship type
   - Apply distance decay
   - Consider recency/churn

2. **Heuristics Implementation**
   - Same directory preference
   - Import relationship priority
   - Test file detection
   - Config file identification

**Acceptance Criteria:**
- [ ] Relevance scoring implemented
- [ ] Heuristics improve quality
- [ ] Tests included >90% when exist
- [ ] Config included when relevant

### Week 4: Language Strategies
**Agent: mcp-context-engineer**

#### Tasks
1. **React-Specific Logic**
   - Detect React components
   - Find route definitions
   - Include hooks and context
   - Handle JSX relationships

2. **Strategy Framework**
   - Create strategy interface
   - Implement language detection
   - Add Python/Rust strategies
   - Make strategies configurable

**Acceptance Criteria:**
- [ ] React components get proper context
- [ ] Language-specific strategies work
- [ ] Strategies configurable
- [ ] Fallback to default strategy

## Phase 3: Performance (Week 5)

### Tasks
**Agent: performance-engineer + database-engineer**

1. **Query Optimization**
   - Create materialized views
   - Optimize recursive CTEs
   - Add strategic indices
   - Profile slow queries

2. **Caching System**
   - Implement bundle cache
   - Cache graph traversals
   - Add cache invalidation
   - Monitor cache effectiveness

3. **Parallel Processing**
   - Concurrent chunk loading
   - Parallel relationship queries
   - Async file reading
   - Pipeline optimization

**Acceptance Criteria:**
- [ ] p95 assembly <120ms
- [ ] Cache hit rate >60%
- [ ] Memory usage bounded
- [ ] Handles concurrent requests

## Phase 4: Integration (Week 6)

### Tasks
**Agent: mcp-tools-engineer + integration-tester**

1. **MCP Tool Implementation**
   - Implement context tool handler
   - Add parameter validation
   - Create response formatting
   - Handle errors gracefully

2. **Testing Suite**
   - Unit tests for assembly logic
   - Integration tests with real data
   - Performance benchmarks
   - Quality validation tests

3. **Documentation**
   - API documentation
   - Configuration guide
   - Strategy customization
   - Performance tuning

**Acceptance Criteria:**
- [ ] MCP tool fully functional
- [ ] >90% test coverage
- [ ] Documentation complete
- [ ] Performance benchmarks met

## Resource Requirements

### Infrastructure
- No additional infrastructure
- Uses existing PostgreSQL
- Leverages current file system

### Dependencies
- Token counting library
- Graph algorithms
- Caching library

## Success Metrics

### Quantitative
- Assembly time <120ms p95
- Token usage 40-70% of budget
- Test inclusion >90%
- Cache hit rate >60%

### Qualitative
- Context relevance high
- Agent satisfaction improved
- Debugging easier
- Code understanding better

## Testing Strategy

### Test Cases
1. Single function context
2. Class with methods
3. React component with hooks
4. Cross-file dependencies
5. Large file truncation
6. Budget overflow handling

### Quality Tests
- Relevance scoring accuracy
- Token counting precision
- Strategy effectiveness
- Cache behavior

## Rollback Plan

If issues occur:
1. Disable context tool
2. Fall back to simple file loading
3. Fix issues offline
4. Re-enable progressively

## Future Enhancements

- Learned relevance models
- User preference tracking
- Cross-repository context
- Semantic chunking
- Natural language summaries