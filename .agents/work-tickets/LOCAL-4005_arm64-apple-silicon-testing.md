# Ticket: LOCAL-4005: Test on ARM64 architecture (Apple Silicon)

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
Validate that the entire Docker Compose stack (PostgreSQL + pgvector, Ollama, Maproom MCP) works correctly on ARM64/Apple Silicon Macs, ensuring compatibility and performance for developers using M1/M2/M3 hardware.

## Background
Apple Silicon Macs are increasingly common among developers, making ARM64 support critical for adoption of the LOCAL project. While most modern Docker images support multi-platform builds, there are potential architecture-specific issues that need validation:

- Different performance characteristics between ARM64 and AMD64
- Ollama may perform differently (better or worse) on ARM64
- PostgreSQL pgvector must support ARM64
- Maproom Rust binary needs to compile for ARM64
- Docker multi-stage builds must work on both architectures

This ticket ensures the entire stack works flawlessly on Apple Silicon, providing confidence for the growing developer base using ARM64 Macs.

## Acceptance Criteria
- [ ] Docker Compose stack builds successfully on ARM64 (M1/M2/M3 Mac)
- [ ] All services (postgres, ollama, maproom) start and reach healthy state on ARM64
- [ ] E2E tests from LOCAL-4004 pass identically on ARM64
- [ ] Performance metrics are within acceptable range compared to AMD64 (not more than 20% difference)
- [ ] No ARM64-specific errors, warnings, or compatibility issues
- [ ] Documentation includes notes on any platform-specific differences found
- [ ] Docker Compose configuration supports both AMD64 and ARM64 platforms
- [ ] Multi-platform image manifest created (if publishing images to registry)

## Technical Requirements

### Platform Testing Requirements
1. **Test Environment**:
   - Primary: M1/M2/M3 Mac (ARM64 native)
   - Comparison: AMD64 system (Intel Mac or Linux)
   - Docker Desktop for Mac (latest version)
   - Same Docker Compose configuration on both platforms

2. **Image Compatibility Validation**:
   - PostgreSQL with pgvector: Verify official image supports ARM64
   - Ollama official image: Verify ARM64 support
   - Maproom Rust binary: Verify successful cross-compilation or native build
   - All dependencies: Verify availability for ARM64 architecture

3. **Build Testing**:
   - Multi-stage Dockerfile builds successfully on ARM64
   - No architecture-specific hardcoded values in build scripts
   - Rust cross-compilation toolchain (if needed for multi-platform)
   - Build times compared between architectures

4. **Runtime Testing**:
   - All services start without errors on Apple Silicon
   - Docker health checks pass on ARM64
   - Service initialization times measured and compared
   - Memory usage patterns analyzed for both platforms

5. **Functional Validation**:
   - Run identical E2E test suite from LOCAL-4004
   - Verify embedding generation produces correct vectors
   - Verify search quality identical to AMD64 results
   - MCP integration works on ARM64
   - Model loading and inference with Ollama + nomic-embed-text

6. **Performance Validation**:
   - Benchmark embedding generation throughput (vectors/second)
   - Measure indexing performance (chunks/second)
   - Search query latency comparison
   - Resource usage: CPU, RAM, disk I/O
   - Ollama inference speed (may be faster on M-series)

## Implementation Notes

### Known Platform Differences
- **Performance**: ARM64 may show different (often better) performance for certain workloads
- **Ollama**: May leverage Apple's Metal GPU acceleration, potentially faster than AMD64 CPU-only
- **Memory**: Memory usage patterns may vary due to different architecture
- **Docker**: Docker Desktop on macOS uses virtualization; performance varies between Intel and Apple Silicon

### Testing Approach
1. **Setup Phase**:
   - Clean Docker environment on ARM64 test machine
   - Clone repository to ARM64 Mac
   - Ensure Docker Desktop is up to date

2. **Build Phase**:
   - Run `docker compose build` on ARM64
   - Capture build logs for any warnings
   - Measure build time
   - Verify all images created successfully

3. **Runtime Phase**:
   - Run `docker compose up -d` on ARM64
   - Monitor service startup with `docker compose ps`
   - Check logs: `docker compose logs -f`
   - Wait for all health checks to pass

4. **Functional Testing Phase**:
   - Run E2E test suite from LOCAL-4004
   - Test MCP integration with Claude Desktop (macOS)
   - Verify embedding generation
   - Verify search results match AMD64

5. **Performance Testing Phase**:
   - Run same benchmarks as LOCAL-4001 on ARM64
   - Compare metrics: throughput, latency, resource usage
   - Document any significant differences (>20%)

6. **Comparison Phase**:
   - Run identical tests on AMD64 platform
   - Generate side-by-side comparison report
   - Identify any ARM64-specific issues

### Docker Multi-Platform Considerations
If publishing images to a registry, use Docker buildx for multi-platform builds:

```bash
# Enable buildx (if not already enabled)
docker buildx create --use

# Build for multiple platforms
docker buildx build --platform linux/amd64,linux/arm64 \
  -t crewchief/maproom-mcp:latest .

# Build and push to registry
docker buildx build --platform linux/amd64,linux/arm64 \
  -t crewchief/maproom-mcp:latest \
  --push .
```

### Expected Results
- **Build**: Should succeed on both platforms with similar build times
- **Startup**: All services should reach healthy state within 2 minutes
- **Performance**: ARM64 may be faster due to M-series efficiency (especially Ollama)
- **Compatibility**: No functional differences between platforms
- **Resource Usage**: ARM64 may use less RAM due to better memory efficiency

### Risk Mitigation
- **Risk**: PostgreSQL pgvector not available for ARM64
  - **Mitigation**: Verify image availability before testing; use official postgres image with pgvector extension

- **Risk**: Rust compilation fails on ARM64
  - **Mitigation**: Ensure rust toolchain includes ARM64 target; use `rustup target add aarch64-apple-darwin`

- **Risk**: Ollama model incompatible with ARM64
  - **Mitigation**: Verify nomic-embed-text model supports ARM64; check Ollama documentation

- **Risk**: Performance significantly worse on ARM64
  - **Mitigation**: Profile and optimize; consider different resource limits; may need Apple Metal GPU support

## Dependencies
- **LOCAL-4004**: E2E tests working on AMD64 (must be completed first)
- **LOCAL-3001**: npx startup flow working (for testing npm package on ARM64)
- **LOCAL-2006**: Batch embedding with nomic-embed-text (ensures embedding functionality works)
- **Docker Desktop for Mac**: Latest version with Apple Silicon support
- **Access to ARM64 Mac**: M1/M2/M3 hardware for testing

## Risk Assessment
- **Risk**: ARM64 compatibility issues discovered late in development
  - **Mitigation**: Test early in Phase 4; allows time to fix issues before release

- **Risk**: Performance degradation on ARM64
  - **Mitigation**: Benchmark both platforms; document differences; optimize if needed

- **Risk**: Multi-platform Docker images complicate build process
  - **Mitigation**: Use Docker buildx; automate with CI/CD; document build process

- **Risk**: Ollama behaves differently on ARM64 vs AMD64
  - **Mitigation**: Test thoroughly; document behavioral differences; ensure functional parity

- **Risk**: Apple Silicon-specific Docker quirks
  - **Mitigation**: Review Docker Desktop for Mac documentation; test on multiple Mac models if available

## Files/Packages Affected
- **Dockerfile**: May need multi-platform build annotations
- **docker-compose.yml**: May need platform-specific configurations
- **README.md**: Add ARM64 compatibility notes
- **.github/workflows/**: CI/CD pipelines for multi-platform builds
- **docs/troubleshooting.md**: Add ARM64-specific troubleshooting
- **package.json**: Ensure npm package works on ARM64 macOS
- **build scripts**: Update for multi-platform support if needed

## Testing Checklist

### Pre-Test Setup
- [ ] Clean Docker environment on ARM64 Mac
- [ ] Docker Desktop updated to latest version
- [ ] Repository cloned on ARM64 system
- [ ] AMD64 comparison system available

### Build Testing
- [ ] `docker compose build` succeeds on ARM64
- [ ] No architecture-specific build errors
- [ ] Build time measured and documented
- [ ] All images created successfully

### Runtime Testing
- [ ] `docker compose up -d` starts all services
- [ ] PostgreSQL reaches healthy state
- [ ] Ollama reaches healthy state
- [ ] Maproom reaches healthy state
- [ ] No error logs during startup

### Functional Testing
- [ ] E2E tests from LOCAL-4004 pass on ARM64
- [ ] Embedding generation works correctly
- [ ] Search results match AMD64 quality
- [ ] MCP integration works with Claude Desktop
- [ ] Model loading succeeds (nomic-embed-text)

### Performance Testing
- [ ] Embedding throughput measured
- [ ] Search latency measured
- [ ] CPU usage profiled
- [ ] RAM usage profiled
- [ ] Disk I/O measured
- [ ] Results compared to AMD64 baseline

### Documentation
- [ ] Platform compatibility documented
- [ ] Performance differences noted (if any)
- [ ] ARM64-specific setup instructions added
- [ ] Troubleshooting guide updated

## Platform Testing Matrix

| Component | AMD64 | ARM64 (M1/M2/M3) | Status |
|-----------|-------|------------------|--------|
| PostgreSQL + pgvector | ✓ | ? | To test |
| Ollama | ✓ | ? | To test |
| Maproom MCP | ✓ | ? | To test |
| Docker Compose | ✓ | ? | To test |
| npm package | ✓ | ? | To test |
| E2E tests | ✓ | ? | To test |
| MCP integration | ✓ | ? | To test |

## Success Metrics
- **Build Success Rate**: 100% on both platforms
- **Test Pass Rate**: 100% of E2E tests pass on ARM64
- **Performance Delta**: <20% difference between platforms
- **Resource Usage**: Within expected bounds (similar to AMD64)
- **Error Rate**: Zero architecture-specific errors
- **Developer Satisfaction**: Seamless experience on Apple Silicon

## References
- Docker multi-platform builds: https://docs.docker.com/build/building/multi-platform/
- Ollama ARM64 support: https://ollama.com/download/mac
- PostgreSQL Docker ARM64: https://hub.docker.com/_/postgres (multi-platform)
- Rust cross-compilation: https://rust-lang.github.io/rustup/cross-compilation.html
- Apple Silicon Docker: https://docs.docker.com/desktop/install/mac-install/
