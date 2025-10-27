# Ticket: LOCAL-1001: Create Maproom Dockerfile with multi-stage build

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create a multi-stage Dockerfile for the Maproom MCP service that builds the Rust binary and packages it into a minimal, secure runtime image with health checks and proper configuration.

## Background
This is the first ticket in the LOCAL project, which implements a fully containerized Maproom MCP service with local LLM embeddings (Ollama + nomic-embed-text), bundled PostgreSQL, and zero-configuration deployment via npm.

The Dockerfile is foundational infrastructure that will enable the containerized deployment of Maproom. It needs to follow Docker best practices for multi-stage builds to minimize final image size while maintaining security and reliability.

## Acceptance Criteria
- [ ] Dockerfile.maproom exists in project root directory
- [ ] Multi-stage build successfully reduces final image size
- [ ] Image builds successfully with `docker build -f Dockerfile.maproom .`
- [ ] Health check is configured and working properly
- [ ] Final image size is < 500MB
- [ ] Binary runs and responds to health checks correctly

## Technical Requirements
- **Stage 1 (Builder)**: Use `rust:1.75-slim` as build base
  - Build Rust binary from `crates/maproom` directory
  - Include all necessary build dependencies
- **Stage 2 (Runtime)**: Use `debian:bookworm-slim` as runtime base
  - Install minimal runtime dependencies: ca-certificates, libssl3
  - Copy binary from builder stage
  - Expose port 3000 for MCP service
- **Health Check**: Configure with curl to verify service responsiveness
- **Optimization**: Minimize layers and image size
- **Security**: Use non-root user where possible, minimal attack surface

## Implementation Notes

### Dockerfile Structure
The Dockerfile should follow this reference architecture:

```dockerfile
# Stage 1: Build
FROM rust:1.75-slim as builder
# Build steps for Maproom binary from crates/maproom

# Stage 2: Runtime
FROM debian:bookworm-slim
# Install ca-certificates and libssl3
# Copy binary from builder
# Expose port 3000
# Add health check with curl
# Set proper ENTRYPOINT and CMD
```

### Build Context
- The build context is the project root
- The Maproom source is in `crates/maproom/`
- Binary output should be copied to a standard location in the runtime image

### Health Check
- Use curl to ping the service endpoint
- Configure appropriate interval, timeout, and retries
- Health check should validate service is responding, not just container running

### Image Size Optimization
- Multi-stage build to exclude build tools from final image
- Use slim base images
- Clean up package manager caches
- Consider stripping debug symbols if appropriate

## Dependencies
- None (this is the first ticket in the LOCAL project)

## Risk Assessment
- **Risk**: Build may fail due to missing Rust dependencies
  - **Mitigation**: Use well-tested rust:1.75-slim base image and document all required dependencies
- **Risk**: Runtime errors due to missing shared libraries
  - **Mitigation**: Include ca-certificates and libssl3 in runtime image; test binary execution
- **Risk**: Image size exceeds 500MB target
  - **Mitigation**: Multi-stage build, slim base images, and careful dependency management
- **Risk**: Health check may be too aggressive or too lenient
  - **Mitigation**: Test health check parameters with actual service startup times

## Files/Packages Affected
- `/workspace/Dockerfile.maproom` (new file)
