# Ticket: LOCAL-2006: Test batch embedding with nomic-embed-text

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- test-runner
- verify-ticket
- commit-ticket

## Summary
Validate that batch embedding processing works correctly with Ollama's nomic-embed-text model, measuring performance and verifying quality of embeddings for code and text inputs.

## Background
This is part of Phase 2 (Ollama Integration) of the LOCAL project. After implementing request formatting (LOCAL-2004), we need to validate that batch embedding processing meets MVP performance targets and correctly handles realistic code samples. This testing validates that the system can handle real-world indexing workloads with appropriate throughput, latency, and memory usage.

## Acceptance Criteria
- [ ] Test suite for batch embedding created
- [ ] Small batch test (10 chunks) passes in <2s
- [ ] Medium batch test (50 chunks) passes in <10s
- [ ] Large batch test (100 chunks) completes without errors
- [ ] Performance metrics logged for analysis
- [ ] Embeddings have correct dimensions (768)
- [ ] Batch processing handles various content types
- [ ] Memory usage tracked and within limits

## Technical Requirements
- Create integration test suite for batch embedding operations
- Test with realistic code samples (functions, classes, constants, docstrings, comments, configuration fragments)
- Implement performance measurement and logging
- Verify embedding dimensions (768 for nomic-embed-text)
- Track memory usage during batch processing
- Target performance metrics:
  - Throughput: 500-1000 chunks/minute on CPU
  - Latency: <100ms per chunk for small batches
  - Memory: Batch processing shouldn't exceed 1GB

## Implementation Notes

### Test Scenarios

**1. Small Batch (10 chunks)**
- Mix of functions, classes, constants
- Verify all embeddings generated
- Measure time (<2 seconds target)

**2. Medium Batch (50 chunks)**
- Realistic file indexing scenario
- Verify batch processing efficiency
- Measure time (<10 seconds target)
- Verify no timeout errors

**3. Large Batch (100 chunks)**
- Stress test batch size limits
- Verify memory usage stays reasonable
- Measure throughput (chunks/second)

**4. Content Types**
- Code snippets (functions, classes)
- Docstrings and comments
- Configuration fragments
- Verify quality across types

### Performance Targets (from LOCAL_ANALYSIS.md)
- Throughput: 500-1000 chunks/minute on CPU
- Latency: <100ms per chunk for small batches
- Memory: Batch processing shouldn't exceed 1GB

### Focus Areas
For MVP, focus on functional correctness and basic performance validation. Ensure the system can handle real-world indexing workloads without errors or excessive resource consumption.

## Dependencies
- LOCAL-2004: Request formatting for embeddings (prerequisite)
- Ollama service must be running with nomic-embed-text model
- Docker Compose infrastructure from Phase 1

## Risk Assessment
- **Risk**: Batch processing may be slower than expected on CPU-only systems
  - **Mitigation**: Document actual performance, adjust batch sizes if needed, consider async processing improvements
- **Risk**: Memory usage may exceed limits with large batches
  - **Mitigation**: Monitor memory usage, implement batch size limits, test incrementally
- **Risk**: Ollama service may timeout on large batches
  - **Mitigation**: Implement retry logic, adjust timeout settings, consider batch splitting

## Files/Packages Affected
- New test file for batch embedding integration tests (likely in test suite)
- Test fixtures with realistic code samples
- Performance measurement utilities
- Documentation of performance characteristics

## References
- Ollama performance documentation: https://github.com/ollama/ollama#performance
- nomic-embed-text model: https://ollama.com/library/nomic-embed-text
- LOCAL_PLAN.md: Task ID LOCAL-2006
- LOCAL_ANALYSIS.md: Performance targets and benchmarks
