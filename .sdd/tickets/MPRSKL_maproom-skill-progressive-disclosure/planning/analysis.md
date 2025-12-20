# Analysis: Maproom Skill Progressive Disclosure (MPRSKL)

## Problem Definition

Three interconnected issues affect the usability of maproom for AI agents:

### 1. Factory/Config Dimension Mismatch Bug (CRITICAL)

When Ollama is auto-detected via network, a coordination bug between `factory.rs` and `config.rs` causes embedding dimension mismatch errors:

**Error symptom:**
```
Error: expected 1536 dimensions but got 1024
```

**Root cause analysis:**

The factory (`factory.rs`) auto-detects Ollama at runtime via network probe:
```rust
// factory.rs:174 - Auto-detection succeeds
match detect_ollama_endpoint().await {
    Some(endpoint) => {
        tracing::info!("Ollama detected at: {}", endpoint);
        ("ollama".to_string(), Some(endpoint))
    }
    ...
}
```

However, the factory then calls `EmbeddingConfig::from_env()` without passing the detected provider:
```rust
// factory.rs:213-214
let config = EmbeddingConfig::from_env()?;
let dimension = config.dimension;
```

The config (`config.rs:105-133`) only infers dimensions when `MAPROOM_EMBEDDING_PROVIDER` env var equals "ollama":
```rust
// config.rs:133 - Inference check
if explicit_dimension.is_none() && config.provider == Provider::Ollama {
    if let Some(inferred_dim) = infer_ollama_dimension(&config.model) { ... }
}
```

But `config.provider` defaults to `Provider::OpenAI` (line 24-26) when no env var is set:
```rust
impl Default for Provider {
    fn default() -> Self {
        Self::OpenAI
    }
}
```

**The bug flow:**
1. Factory detects Ollama via network check (factory.rs:174)
2. Factory calls `EmbeddingConfig::from_env()` (factory.rs:213)
3. Config sees no `MAPROOM_EMBEDDING_PROVIDER` env var
4. Config provider stays at default `Provider::OpenAI` (not `Provider::Ollama`)
5. Dimension inference skipped (only happens when `provider == Ollama`)
6. Dimension stays at default 1536 (OpenAI default)
7. Ollama returns 1024-dim embeddings (mxbai-embed-large default)
8. Mismatch error: "expected 1536 dimensions but got 1024"

**Critical Detail:** Dimension inference code EXISTS in config.rs (lines 133-149) but is INSUFFICIENT for fixing this bug because:
- Inference only runs when `config.provider == Provider::Ollama` (line 133 check)
- In zero-config scenarios, factory detects Ollama AFTER calling `from_env()`
- Config defaults to `Provider::OpenAI` when no env var is set
- Result: Inference check evaluates to false, dimension inference never runs
- **User verified:** Bug still occurs with zero-config Ollama (December 2025)

**Why Phase 1 Fix is Necessary:** The `from_env_with_provider()` approach allows the factory to pass the detected provider during config creation, enabling the existing inference logic to run correctly.

### 2. Skill Documentation Architecture

The current maproom-search skill (`SKILL.md`) is comprehensive but not optimized for AI agent consumption:

**Current issues:**
- Too verbose for initial capability discovery (196 lines)
- Mixes basic usage with advanced patterns
- No progressive disclosure structure
- References are comprehensive but not layered

**AI agent needs:**
- Brief initial skill description (capabilities + when to use)
- Clear decision tree for tool selection
- Progressive depth: basic commands -> advanced patterns -> troubleshooting
- Structured output for agent parsing

### 3. CLI Error Messages and UX

When dimension mismatch occurs, the error message doesn't help users diagnose or fix the issue:

**Current error:**
```
Error: expected 1536 dimensions but got 1024
```

**Missing guidance:**
- No indication that this is a provider/config mismatch
- No suggestion to check `MAPROOM_EMBEDDING_PROVIDER`
- No way to verify current configuration before operations
- No `--skip-embeddings` flag to proceed without embedding generation

## Context

### Why This Matters

1. **AI agents are primary users** - Skills are the interface for Claude and other AI agents to use maproom/crewchief effectively
2. **Zero-config is broken** - The dimension mismatch bug prevents the intended zero-config Ollama experience
3. **Error recovery is poor** - Agents cannot self-diagnose or recover from configuration issues

### Existing Code Structure

**Factory/Config Locations:**
- `crates/maproom/src/embedding/factory.rs` - Provider creation with auto-detection
- `crates/maproom/src/embedding/config.rs` - Configuration loading from env vars

**Skill Locations:**
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md`
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/search-best-practices.md`

**CLI Entry Point:**
- `crates/maproom/src/main.rs` - Command definitions and execution

## Existing Solutions

### Industry Patterns for Progressive Disclosure

**Claude Plugins Pattern:**
```
skills/
  {skill-name}/
    SKILL.md           # Brief: capabilities, when to use, quick reference
    references/        # Deeper documentation by topic
      advanced.md
      troubleshooting.md
```

**Man Pages Pattern:**
- Synopsis: one-line command description
- Description: brief overview
- Options: detailed flag documentation
- Examples: common use cases
- See Also: related commands/docs

### Codebase Patterns

**Dimension Inference (Already Exists in config.rs):**
```rust
fn infer_ollama_dimension(model: &str) -> Option<usize> {
    if model.starts_with("nomic-embed-text") {
        Some(768)
    } else if model.starts_with("mxbai-embed-large") {
        Some(1024)
    } else {
        None
    }
}
```

This works correctly when `provider == Provider::Ollama`, but that condition fails due to the factory/config coordination bug.

## Current State

### Factory/Config Flow (Buggy)

```
create_provider_from_env()
    |
    +-- detect_ollama_endpoint() --> Some(endpoint)
    |       (Ollama detected via network)
    |
    +-- EmbeddingConfig::from_env()
    |       |
    |       +-- provider = Provider::OpenAI (default, no env var)
    |       +-- dimension inference SKIPPED (provider != Ollama)
    |       +-- dimension = 1536 (OpenAI default)
    |
    +-- OllamaProvider::new_with_config(..., dimension=1536, ...)
            |
            +-- Ollama returns 1024-dim vectors
            +-- MISMATCH ERROR!
```

### Current Skill Structure

```
maproom/skills/maproom-search/
  SKILL.md                    # 196 lines, comprehensive but flat
  references/
    search-best-practices.md  # 371 lines, detailed patterns
```

## Research Findings

### Fix Strategy Analysis

**Option A: Factory Sets Env Var**
- Factory sets `MAPROOM_EMBEDDING_PROVIDER=ollama` before calling `from_env()`
- Simple, maintains encapsulation
- Risk: env var pollution, affects other code

**Option B: Factory Passes Provider to Config**
- Add parameter to `from_env()` or new constructor
- Clean API, explicit data flow
- Breaking change potential

**Option C: Config Detects Provider from Context (Rejected)**
- Config would need network access
- Violates separation of concerns

**Recommendation: Option B (Pass Provider to Config)**
- Cleaner architecture
- No side effects
- Type-safe provider propagation

### Skill Architecture Analysis

**Current skill is agent-usable but not agent-optimized:**
- Good: Decision tree exists
- Good: Command reference complete
- Gap: No brief capability summary at top
- Gap: References are flat, not layered
- Gap: No troubleshooting section

**Progressive disclosure structure needed:**
```
SKILL.md (brief)
  |
  +-- When to use maproom vs grep vs glob
  +-- Quick reference (5-10 most common commands)
  +-- "See references/ for advanced patterns"

references/
  +-- search-strategies.md (intermediate)
  +-- cli-reference.md (complete command docs)
  +-- troubleshooting.md (error recovery)
```

## Constraints

### Technical Constraints

1. **Backward compatibility** - Existing env var configuration must continue working
2. **Test coverage** - Factory/config tests already exist, must not break
3. **Plugin structure** - Skills must follow claude-code-plugins conventions
4. **Rust API stability** - Config struct is public, changes need care

### Resource Constraints

1. **Single ticket scope** - Bug fix, skill restructure, CLI improvements
2. **No database changes** - Dimension handling is in-memory/env-var only

### Quality Constraints

1. **Coverage thresholds** - Must maintain existing Rust test coverage
2. **Integration tests** - Factory/config interaction needs explicit test
3. **Agent testing** - Skills should be validated with actual agent usage

## Success Criteria

### Primary (Must Have)

1. **Bug Fixed**: `crewchief-maproom scan` with auto-detected Ollama uses correct dimension (1024 for mxbai-embed-large) without any env vars set
2. **Skill Restructured**: SKILL.md is under 50 lines with clear capability summary and references to detailed docs
3. **Error Improved**: Dimension mismatch error includes actionable guidance

### Secondary (Should Have)

4. **CLI Enhancement**: `--skip-embeddings` flag allows scan without embedding generation
5. **Config Display**: Way to show current embedding configuration before operations
6. **Test Coverage**: Explicit integration test for factory/config/dimension flow

### Tertiary (Nice to Have)

7. **Troubleshooting Doc**: Common errors with solutions in references/
8. **Agent Validation**: Skill tested with actual Claude agent invocation
