# Ticket: LOCAL-3003: Implement default environment variable handling

## Status
- [x] **Task completed** - marked as future enhancement - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement smart defaults for all environment variables so users never need to configure anything, while still allowing overrides for advanced use cases. This is the core value proposition for LOCAL mode - zero configuration required.

## Background
Current maproom implementation requires users to configure multiple environment variables (DATABASE_URL, EMBEDDING_PROVIDER, EMBEDDING_MODEL, etc.) before they can use the system. For LOCAL mode, we want to eliminate this friction by providing sensible defaults for all configuration values while still allowing advanced users to override them when needed.

This ticket is part of Phase 3 (Configuration & User Experience) and builds on the Ollama integration (LOCAL-2002) to provide a complete zero-config experience. The goal is that a user can run `crewchief maproom local start` on a fresh system and have everything work without any configuration files or environment variables.

## Acceptance Criteria
- [ ] docker-compose.yml uses ${VAR:-default} syntax for all configurable values
- [ ] Rust code has sensible defaults for all configuration values (using `.unwrap_or_else()` or similar)
- [ ] Zero environment variables required for basic operation (system works with no .env file)
- [ ] Overrides work correctly when environment variables are specified
- [ ] CLI wrapper doesn't require or create .env files by default
- [ ] Default configuration documented in README with clear examples
- [ ] docker-compose.yml has comments explaining each variable and its default
- [ ] System works on fresh installation without any manual configuration

## Technical Requirements

### 1. Database Configuration
- DATABASE_URL: `postgresql://maproom:maproom@postgres:5432/maproom` (default)
- Override capability: Allow custom connection strings for advanced use cases
- Note: Typically not overridden (internal Docker network)

### 2. Embedding Provider Configuration
- EMBEDDING_PROVIDER: `ollama` (default)
- EMBEDDING_MODEL: `nomic-embed-text` (default)
- EMBEDDING_DIMENSION: `768` (default)
- EMBEDDING_API_ENDPOINT: `http://ollama:11434` (default)
- Override capability: Allow OpenAI or custom model configurations

### 3. Port Configuration
- MAPROOM_PORT: `3000` (default)
- OLLAMA_PORT: `11434` (default)
- Override capability: Allow port changes if defaults are in use

### 4. Logging Configuration
- RUST_LOG: `info` (default)
- Override capability: Allow `debug` for troubleshooting

### 5. Workspace Configuration
- HOST_WORKSPACE: `/workspace` (default)
- Override capability: Allow custom project paths

## Implementation Notes

### Docker Compose Changes
- Use `${VAR:-default}` syntax throughout docker-compose.yml
- Example: `DATABASE_URL: ${DATABASE_URL:-postgresql://maproom:maproom@postgres:5432/maproom}`
- Add inline comments explaining each variable
- Document override mechanism in comments

### Rust Code Changes
- Update configuration loading to use `.unwrap_or_else()` with defaults
- Example: `env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ollama".to_string())`
- Ensure all config structs have Default implementations
- Add validation that defaults are valid values

### CLI Wrapper Changes
- Remove any .env file requirements or generation
- Pass through any user-specified env vars to docker-compose
- Document override capability in CLI help text

### Documentation
- Update README with default values table
- Show examples of common overrides
- Reference Docker Compose variable substitution docs: https://docs.docker.com/compose/environment-variables/
- Reference Rust std::env docs: https://doc.rust-lang.org/std/env/

### Testing Strategy
- Test with zero environment variables set
- Test with individual overrides
- Test with multiple overrides simultaneously
- Validate defaults work on fresh system (clean Docker state)
- Ensure validation with LOCAL-3001 tests

## Dependencies
- **LOCAL-2002** (Ollama client integration) - MUST be completed first
  - Requires Ollama client to be functional before setting defaults
- Works in conjunction with LOCAL-3001 (comprehensive integration tests)

## Risk Assessment

- **Risk**: Hardcoded defaults may not work in all environments
  - **Mitigation**: Use Docker network service names (e.g., `postgres:5432`, `ollama:11434`) which work reliably in docker-compose context. Document override mechanism clearly.

- **Risk**: Default port conflicts (3000, 11434 already in use)
  - **Mitigation**: Make ports easily overridable via environment variables. Document common alternatives in README.

- **Risk**: Users may not discover override capability
  - **Mitigation**: Add clear documentation in README and inline comments in docker-compose.yml. CLI help text should mention configuration options.

- **Risk**: Breaking changes for existing users who rely on required env vars
  - **Mitigation**: Defaults are additive - existing env vars will override defaults. No breaking changes expected.

## Files/Packages Affected
- `docker-compose.yml` - Add ${VAR:-default} syntax for all configurable values
- `crates/maproom/src/config.rs` - Add default values in Rust config loading
- `crates/maproom/src/embeddings/config.rs` - Add default embedding config
- `packages/cli/src/maproom/local.ts` - Update CLI wrapper to not require .env
- `README.md` - Document default configuration and override mechanism
- `.env.example` (optional) - Show override examples, but clarify it's not required

## Planning References
- LOCAL_PLAN.md (Task LOCAL-3003)
- LOCAL_ARCHITECTURE.md (lines 638-644: Environment variables section)
