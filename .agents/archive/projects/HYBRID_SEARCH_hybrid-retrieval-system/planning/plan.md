# HYBRID_SEARCH Plan: Hybrid Retrieval System

## Project Overview
Implement a state-of-the-art hybrid search system combining full-text search, vector similarity, and graph signals to achieve >80% recall and <50ms p95 latency.

## Phase 1: Embedding Infrastructure (Week 1)

### Tasks
**Agent: embeddings-engineer**

1. **Embedding Service Setup**
   - Configure OpenAI client with text-embedding-3-small
   - Implement embedding generation with retry logic
   - Set up LRU cache for embeddings
   - Create batch processing pipeline

2. **Database Preparation**
   - Add vector columns if not present
   - Create ivfflat indices
   - Configure index parameters (lists=200, probes=10)
   - Run ANALYZE on tables

3. **Embedding Generation**
   - Generate embeddings for existing chunks
   - Implement incremental embedding updates
   - Create monitoring for embedding costs
   - Test with 1000 sample chunks

**Acceptance Criteria:**
- [ ] Successfully generate embeddings for all chunks
- [ ] Embedding cache with >80% hit rate
- [ ] Cost tracking implemented
- [ ] Vector indices created and optimized

## Phase 2: Search Pipeline (Week 2)

### Tasks
**Agent: database-engineer + search-quality-engineer**

1. **Query Processing**
   - Implement query tokenizer
   - Add query expansion logic
   - Create embedding generation for queries
   - Implement search mode detection

2. **Parallel Search Execution**
   - FTS query implementation
   - Vector similarity search
   - Graph signal queries
   - Recency/churn score integration

3. **Initial Integration**
   - Basic score combination
   - Result deduplication
   - Simple ranking pipeline
   - API endpoint creation

**Acceptance Criteria:**
- [ ] All search types return results
- [ ] Parallel execution under 100ms
- [ ] Basic fusion working
- [ ] API endpoint functional

## Phase 3: Score Fusion (Week 3)

### Tasks
**Agent: database-engineer**

1. **Reciprocal Rank Fusion**
   - Implement RRF algorithm
   - Configure k parameter (default 60)
   - Create fusion benchmarks
   - Compare with baseline

2. **Weighted Combination**
   - Implement configurable weights
   - Create weight tuning interface
   - Add debug mode for score breakdown
   - Document weight impacts

3. **Signal Integration**
   - Integrate graph importance scores
   - Add recency decay calculation
   - Include churn score normalization
   - Create signal debugging

**Acceptance Criteria:**
- [ ] RRF implementation complete
- [ ] Weighted fusion configurable
- [ ] All signals integrated
- [ ] Score explanations available

## Phase 4: Performance Optimization (Week 4)

### Tasks
**Agent: performance-engineer + database-engineer**

1. **Query Optimization**
   - Create materialized views for common patterns
   - Optimize SQL queries with EXPLAIN ANALYZE
   - Implement query result caching
   - Add connection pooling

2. **Index Tuning**
   - Benchmark different ivfflat settings
   - Create partial indices for filters
   - Optimize GIN index configuration
   - Run pg_stat_statements analysis

3. **Caching Strategy**
   - Implement multi-layer cache
   - Add cache warming on startup
   - Create cache invalidation logic
   - Monitor cache effectiveness

**Acceptance Criteria:**
- [ ] p50 latency <30ms
- [ ] p95 latency <50ms
- [ ] p99 latency <100ms
- [ ] 10+ QPS supported

## Phase 5: Quality Validation (Week 5)

### Tasks
**Agent: search-quality-engineer + integration-tester**

1. **Golden Test Set Creation**
   - Define 100 representative queries
   - Establish ground truth results
   - Create evaluation metrics
   - Build automated testing

2. **Quality Metrics**
   - Measure precision@k
   - Calculate recall@k
   - Compute NDCG scores
   - Track MRR metrics

3. **A/B Testing Framework**
   - Implement shadow mode
   - Create comparison dashboard
   - Log user interactions
   - Analyze result quality

**Acceptance Criteria:**
- [ ] Recall >80% on test set
- [ ] Precision >70% at k=10
- [ ] NDCG >0.75
- [ ] A/B testing operational

## Phase 6: Production Rollout (Week 6)

### Tasks
**Agent: mcp-tools-engineer**

1. **MCP Integration**
   - Update search tool with new parameters
   - Add mode selection (fts/vector/hybrid)
   - Implement filter parameters
   - Create debugging options

2. **Configuration Management**
   - Create configuration file schema
   - Implement hot reload for weights
   - Add feature flags
   - Document all parameters

3. **Monitoring & Alerting**
   - Set up latency monitoring
   - Create quality dashboards
   - Implement error tracking
   - Configure alerts

**Acceptance Criteria:**
- [ ] MCP tools updated
- [ ] Configuration manageable
- [ ] Monitoring operational
- [ ] Documentation complete

## Resource Requirements

### Infrastructure
- PostgreSQL with pgvector
- OpenAI API access ($100/month budget)
- No additional servers required

### Dependencies
- OpenAI SDK or alternative
- pgvector extension
- tokio for async operations

## Risk Mitigation

### Technical Risks
1. **Embedding quality issues**
   - Mitigation: Multiple embedding strategies
   - Fallback: Pure FTS mode

2. **Latency regression**
   - Mitigation: Extensive optimization
   - Fallback: Reduced candidate set

3. **Cost overrun**
   - Mitigation: Embedding cache, monitoring
   - Fallback: Self-hosted embeddings

## Success Metrics

### Quantitative
- Recall: >80%
- Precision@10: >70%
- p95 latency: <50ms
- Embedding cost: <$100/month

### Qualitative
- Improved code discovery
- Better semantic understanding
- Intuitive search behavior
- Developer satisfaction

## Testing Strategy

### Unit Tests
- Query processor logic
- Score fusion algorithms
- Cache behavior
- Error handling

### Integration Tests
- End-to-end search flow
- Database query performance
- API endpoint behavior
- MCP tool integration

### Performance Tests
- Load testing (100+ QPS)
- Latency benchmarks
- Memory profiling
- Cache effectiveness

## Documentation Requirements

### Technical Documentation
- Architecture diagrams
- Score fusion formulas
- Configuration guide
- Tuning recommendations

### User Documentation
- Search query syntax
- Mode selection guide
- Filter usage
- Tips for better results

## Rollback Plan

If issues occur:
1. Feature flag disables hybrid search
2. Falls back to FTS-only
3. Embedding generation continues in background
4. No data loss or corruption

## Future Enhancements (Out of Scope)

### Phase 7+ Considerations
- Cross-encoder reranking
- Query suggestion/completion
- Learned ranking models
- Personalized results
- Multi-modal search (diagrams)