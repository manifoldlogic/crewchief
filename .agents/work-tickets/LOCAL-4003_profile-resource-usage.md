# Ticket: LOCAL-4003: Profile resource usage (CPU, RAM, disk)

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Profile resource consumption (CPU, RAM, disk) across all LOCAL services (PostgreSQL, Ollama, Maproom MCP) to validate we stay within advertised limits (<6GB RAM, <5GB disk) and identify optimization opportunities.

## Background
The LOCAL MVP advertises specific resource requirements:
- **RAM**: 4-5GB total (8GB recommended, 4GB minimum)
- **Disk**: <5GB total
- **CPU**: 2-3 cores during indexing, <0.5 cores idle

Before releasing the package, we need to validate these claims with real profiling data across different usage scenarios. This ticket ensures users with minimum specs can actually run the system and provides data for optimization recommendations.

This is part of Phase 4 (Testing & Optimization) and builds on the performance benchmarks from LOCAL-4001.

## Acceptance Criteria
- [x] Resource usage profiled for all four scenarios (idle, indexing, search, large repo)
- [x] Memory usage stays < 6GB during normal operation (indexing 100 files, concurrent searches)
- [x] Peak memory < 8GB during heavy indexing workload (1000+ files)
- [x] Disk usage < 5GB after indexing 500 files (including Docker images, models, data)
- [x] CPU usage reasonable: <0.5 cores idle, 2-3 cores during indexing (not pegged at 100%)
- [x] Report documents bottlenecks and optimization opportunities for each service
- [x] Recommendations provided for resource-constrained systems (4GB RAM minimum)
- [x] Validation confirms system requirements documentation is accurate

## Technical Requirements

### Profiling Scenarios

1. **Idle State** (all services running, no activity):
   - Memory usage per service (PostgreSQL, Ollama, Maproom MCP)
   - CPU usage baseline
   - Disk I/O patterns

2. **Indexing Workload** (100 files):
   - Peak memory usage during tree-sitter parsing
   - CPU utilization per service
   - Disk I/O patterns (database writes, model loading)
   - Network traffic (should be minimal for LOCAL)

3. **Search Workload** (10 concurrent queries):
   - Memory consumption during search
   - CPU spikes from embedding generation
   - Database query performance
   - Ollama model activation overhead

4. **Large Repository** (1000+ files):
   - Database size growth (chunks table, embeddings, vectors)
   - Vector index memory requirements
   - Query performance degradation
   - Incremental indexing impact

### Resource Targets (from LOCAL_ANALYSIS.md)

**RAM Breakdown**:
- PostgreSQL: 512MB
- Ollama idle: 1GB
- Ollama active (query processing): 2-3GB
- Maproom MCP: 512MB
- Total: 4-5GB typical, <8GB peak

**Disk Breakdown**:
- Docker images: ~2GB
- Models (nomic-embed-text): ~200MB
- PostgreSQL data: ~3GB for typical usage (500-1000 files)
- Total: <5GB

**CPU**:
- Idle: <0.5 cores
- Indexing: 2-3 cores
- Search: 1-2 cores

### Tools and Methods

1. **Docker monitoring**:
   - `docker stats` - Real-time resource monitoring (CPU%, MEM USAGE, NET I/O, BLOCK I/O)
   - `docker system df` - Disk usage analysis (images, containers, volumes)
   - Container-specific stats with `docker stats <container_name>`

2. **Memory profiling**:
   - Rust memory profiling tools (if needed)
   - PostgreSQL memory settings validation (`shared_buffers`, `work_mem`)
   - Ollama model memory tracking

3. **Database analysis**:
   - PostgreSQL query statistics (`pg_stat_statements`)
   - Table sizes (`pg_total_relation_size`)
   - Index sizes and usage

4. **Disk usage tracking**:
   - Volume sizes over time
   - Database growth rate
   - Model cache size

## Implementation Notes

### Profiling Workflow

1. **Baseline Setup**:
   - Start clean LOCAL environment: `npx @crewchief/local start`
   - Wait for all services to stabilize (health checks pass)
   - Record idle state metrics

2. **Indexing Profile**:
   - Index test repository with 100 files
   - Monitor resource usage during indexing
   - Record peak values and time-series data
   - Repeat with 1000-file repository

3. **Search Profile**:
   - Run 10 concurrent search queries
   - Monitor Ollama model activation
   - Track embedding generation overhead
   - Measure query response times

4. **Long-running Profile**:
   - Keep services running for extended period (e.g., 1 hour)
   - Monitor for memory leaks or resource creep
   - Validate idle state remains stable

### Metrics to Collect

For each scenario, collect:
- **CPU**: % utilization per service, average and peak
- **RAM**: Current usage, peak usage, RSS vs VSZ
- **Disk I/O**: Read/write bytes, operations per second
- **Network**: Traffic between containers (should be minimal)
- **Database**: Connection count, query times, cache hit rates
- **Response times**: Index time per file, search latency

### Output Format

Create a profiling report in `/docs/profiling/LOCAL-4003_resource-profile.md`:

```markdown
# LOCAL Resource Usage Profile

## Executive Summary
- Memory: X GB (target: <6GB) ✓/✗
- Disk: Y GB (target: <5GB) ✓/✗
- CPU: Z% (target: reasonable) ✓/✗

## Scenario: Idle State
[Results table]

## Scenario: Indexing (100 files)
[Results table]

## Scenario: Search (10 queries)
[Results table]

## Scenario: Large Repo (1000 files)
[Results table]

## Bottlenecks Identified
1. [Service/component]: [Issue] - [Impact]

## Optimization Opportunities
1. [Component]: [Recommendation] - [Expected improvement]

## Recommendations for Resource-Constrained Systems
- 4GB RAM: [Configuration tweaks]
- Slow disk: [Suggestions]
- Limited CPU: [Tuning options]
```

### Validation Against Documentation

Cross-reference findings with:
- `docs/LOCAL_ANALYSIS.md` (lines 246-255) - Resource targets
- `docs/LOCAL_PLAN.md` - System requirements
- README documentation (once created)

Update documentation if actual usage differs significantly from targets.

## Dependencies
- **LOCAL-4001**: Performance benchmarks (provides baseline test scenarios)
- **LOCAL-1003**: Docker Compose orchestration (services must be running)
- **LOCAL-2005**: Ollama integration tests (confirms embedding generation works)

All Phase 1-3 tickets should be complete for accurate profiling.

## Risk Assessment

- **Risk**: Resource usage exceeds advertised limits
  - **Mitigation**: Identify optimization opportunities, adjust documentation, or optimize services. If 6GB target cannot be met, update system requirements to reflect reality.

- **Risk**: Profiling tools add overhead that skews results
  - **Mitigation**: Use lightweight monitoring (`docker stats`), profile over multiple runs, compare with production-like scenarios.

- **Risk**: Resource usage varies significantly by repository size
  - **Mitigation**: Profile multiple repository sizes (small, medium, large), document scaling characteristics.

- **Risk**: Ollama model memory usage depends on model choice
  - **Mitigation**: Profile with default model (nomic-embed-text), document memory requirements for alternative models.

## Files/Packages Affected

**New Files**:
- `/docs/profiling/LOCAL-4003_resource-profile.md` - Profiling report with results and recommendations

**Existing Files** (may need updates based on findings):
- `/docs/LOCAL_ANALYSIS.md` - Update resource targets if actuals differ
- `/docs/LOCAL_PLAN.md` - Update system requirements
- `README.md` (future) - System requirements section

**Services Profiled**:
- PostgreSQL container
- Ollama container
- Maproom MCP binary (running in container)
- Docker daemon overhead

**Profiling Scripts** (optional):
- `/scripts/profile-resources.sh` - Automated profiling script
- `/scripts/monitor-docker-stats.sh` - Continuous monitoring helper
