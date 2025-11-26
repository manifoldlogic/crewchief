# Ticket: EMBPERF-0001: Baseline & API Validation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: N/A - This is a research/validation ticket, no code tests required.

## Agents
- technical-researcher
- verify-ticket
- commit-ticket

## Summary
Establish current performance baseline and verify Ollama's batch API works as expected before implementing changes. This ensures we have measurable data to compare against and validates our technical assumptions.

## Background
The project aims to improve Ollama embedding throughput 10-20x. Before implementing changes, we need to:
1. Measure current performance as a baseline
2. Verify Ollama's batch API accepts array input as documented
3. Determine optimal batch sizes and concurrency levels empirically

This implements Phase 0 from `plan.md`.

## Acceptance Criteria
- [x] Current throughput measured (texts/sec) with existing implementation
- [x] HTTP request count verified for known batch sizes (e.g., 100 texts = 100 requests currently)
- [x] Ollama `/api/embed` batch input tested: `{"input": ["text1", "text2"]}`
- [x] Batch response format confirmed: `{"embeddings": [[...], [...]]}`
- [x] Optimal batch sizes identified (test: 10, 25, 50, 100, 128)
- [x] Optimal concurrency levels identified (test: 4, 8, 16, 24)
- [x] Brief report created with findings

## Technical Requirements
- Use `nomic-embed-text` model for testing (768 dimensions)
- Test on available hardware (document hardware specs)
- Use `curl` or small Rust test program for API validation
- Measure wall-clock time for throughput calculations

## Implementation Notes

### Baseline Measurement
```bash
# Run existing embedding pipeline on known corpus
# Measure: time, HTTP requests, texts processed
```

### API Validation
```bash
# Test batch input format
curl http://localhost:11434/api/embed -d '{
  "model": "nomic-embed-text",
  "input": ["text one", "text two", "text three"]
}'
# Verify response has 3 embeddings in array
```

### Batch Size Testing
Test latency for batch sizes: 10, 25, 50, 100, 128
Record: request time, GPU utilization if observable

### Concurrency Testing
With batch size fixed at optimal, test parallel requests: 4, 8, 16, 24
Record: total throughput, any errors or throttling

## Dependencies
- Ollama running locally with `nomic-embed-text` model
- No code dependencies - this is pre-implementation research

## Risk Assessment
- **Risk**: Ollama batch API doesn't work as expected
  - **Mitigation**: If batch API fails, document the issue and adjust Phase 1 approach
- **Risk**: Hardware limitations affect baseline accuracy
  - **Mitigation**: Document hardware specs, note that results may vary

## Files/Packages Affected
- New file: `.agents/projects/EMBPERF_ollama-parallel-optimization/research/baseline-report.md`

## Deliverables

### Report Structure
```markdown
# EMBPERF Baseline Report

## Hardware
- CPU/GPU: [specs]
- Memory: [amount]
- Ollama version: [version]

## Baseline (Current Implementation)
- Throughput: X texts/sec
- HTTP requests for 100 texts: 100

## Batch API Validation
- Format confirmed: YES/NO
- Response format: [structure]

## Optimal Batch Size
| Size | Latency | Throughput |
|------|---------|------------|
| 10   | Xms     | Y texts/s  |
| ...  | ...     | ...        |

## Optimal Concurrency
| Level | Throughput | Notes |
|-------|------------|-------|
| 4     | X texts/s  | ...   |
| ...   | ...        | ...   |

## Recommendations
- Recommended batch size: N
- Recommended concurrency: M
- Expected improvement: X-Yx
```
