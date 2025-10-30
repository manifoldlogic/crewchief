# CONTEXT_ASM Analysis: Context Assembly Engine

## Problem Space

### Current Limitations
Maproom currently lacks a sophisticated context assembly system. When agents need code context, they either get too much (entire files) or too little (single functions), leading to:
- **Token waste**: Loading unnecessary code exhausts context windows
- **Missing context**: Related code not included, leading to incomplete understanding
- **Poor relevance**: No prioritization of what's most important
- **Static assembly**: One-size-fits-all approach ignores query intent

### Industry Context

Research shows optimal context window usage is 40-70%, not 100%. Key findings:
- **Cursor IDE**: Dynamic context injection based on relevance
- **Aider**: RepoMap with graph-based importance ranking
- **Continue**: Intelligent chunking at 512 tokens
- **"Lost in the middle"**: LLMs struggle with information in the middle of long contexts

### Current State
The specification defines a context tool that should assemble:
1. Primary chunk (the target code)
2. Tests for that code
3. Callers and callees
4. Related configuration
5. Documentation

Currently, none of this is implemented. The MCP server has a placeholder for the context tool.

## Key Insights

### 1. Budget Awareness is Critical
Not about filling the context window, but about providing the right context. Research shows:
- 40-70% usage is optimal
- Quality > Quantity
- Strategic placement matters (important info at start/end)

### 2. Graph Relationships Drive Relevance
Code structure provides natural importance signals:
- Functions with many callers are important context
- Tests validate behavior
- Config affects execution
- Related components share imports

### 3. Language-Specific Heuristics Matter
- **React**: Include routes, hooks, components
- **Python**: Include class definitions, decorators
- **Rust**: Include trait implementations, modules
Different languages need different assembly strategies.

### 4. Incremental Assembly
Start with the most relevant, expand as budget allows:
1. Primary symbol
2. Direct relationships
3. Transitive relationships
4. Supporting context

## Success Criteria

### Functional Requirements
- [ ] Assemble context within token budget
- [ ] Include relevant test files
- [ ] Track caller/callee relationships
- [ ] Handle configuration context
- [ ] Support expand options

### Performance Requirements
- [ ] Assembly time <120ms p95
- [ ] Memory efficient streaming
- [ ] Handle large codebases
- [ ] Concurrent assembly support

### Quality Metrics
- [ ] Context relevance score >0.8
- [ ] Token utilization 40-70%
- [ ] Include rate for tests >90%
- [ ] User satisfaction with context

## Risk Assessment

### Technical Risks
1. **Graph traversal complexity**
   - Mitigation: Depth limits, caching
2. **Token counting accuracy**
   - Mitigation: Conservative estimates
3. **Memory overhead**
   - Mitigation: Streaming assembly

### Implementation Risks
1. **Over-engineering**
   - Mitigation: Start simple, iterate
2. **Poor relevance**
   - Mitigation: User feedback, metrics

## Recommendations

### MVP Scope
- Basic assembly with primary + tests
- Simple token counting
- Static prioritization
- Fixed expansion rules

### Production Scope
- Graph-based relevance
- Dynamic prioritization
- Language-specific heuristics
- Configurable policies