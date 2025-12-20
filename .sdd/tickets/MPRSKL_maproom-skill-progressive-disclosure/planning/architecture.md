# Architecture: Maproom Skill Progressive Disclosure (MPRSKL)

## Overview

This ticket implements three coordinated changes:

1. **Bug Fix**: Propagate auto-detected provider from factory to config for correct dimension inference
2. **Skill Restructure**: Reorganize maproom skill with progressive disclosure pattern
3. **CLI Improvements**: Better error messages and configuration visibility

The architecture maintains backward compatibility while enabling zero-config Ollama workflows.

## Design Decisions

### Decision 1: Provider Propagation via Constructor Parameter

**Context:** Factory auto-detects Ollama but config doesn't know about it, causing dimension mismatch.

**Decision:** Add `from_env_with_provider(provider: Option<Provider>)` to `EmbeddingConfig`.

**Rationale:**
- Clean API without side effects (vs setting env var)
- Type-safe provider propagation
- Backward compatible (`from_env()` still works for explicit config)
- Follows Rust patterns for optional overrides

**Note on Existing Code:** Dimension inference logic already exists in config.rs (lines 133-149) and is correct. However, it's unreachable in zero-config scenarios because provider detection happens in factory AFTER config creation. The `from_env_with_provider()` method enables the factory to propagate the detected provider, making the existing inference logic accessible. This was verified by user testing in December 2025 - the bug still exists despite the presence of inference code.

**Implementation:**
```rust
// config.rs - New constructor
impl EmbeddingConfig {
    /// Load configuration with explicit provider override.
    /// Used when provider is detected at runtime (e.g., Ollama auto-detection).
    pub fn from_env_with_provider(provider_override: Option<Provider>) -> Result<Self, EmbeddingError> {
        let mut config = Self::default();

        // Apply override first if provided
        if let Some(p) = provider_override {
            config.provider = p;
        }

        // Then load from env (env vars can still override)
        if let Ok(provider) = env::var("MAPROOM_EMBEDDING_PROVIDER") {
            config.provider = provider.parse()?;
        }

        // ... rest of from_env logic with correct provider now set
    }
}

// factory.rs - Use new constructor
"ollama" => {
    let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama))?;
    // Now config.provider == Ollama, dimension inference works!
}
```

### Decision 2: Progressive Disclosure Skill Structure

**Context:** Current SKILL.md is 196 lines, comprehensive but not agent-optimized.

**Decision:** Three-tier documentation structure with brief SKILL.md and layered references.

**Rationale:**
- Agents see capabilities quickly (under 50 lines)
- Detailed docs available on demand
- Follows man page conventions
- Enables different agent interaction patterns

**Structure:**
```
maproom/skills/maproom-search/
  SKILL.md                           # Tier 1: Brief (50 lines)
    - Capability summary (2-3 sentences)
    - When to use maproom vs grep vs glob
    - Quick command reference (5 most common)
    - "See references/ for more"

  references/
    search-best-practices.md         # Tier 2: Existing (keep as-is)
    cli-reference.md                 # Tier 2: Complete command docs (NEW)
    troubleshooting.md               # Tier 2: Error recovery (NEW)
```

### Decision 3: Improved Error Messages with Actionable Guidance

**Context:** Current dimension mismatch error is opaque: "expected 1536 dimensions but got 1024"

**Decision:** Enhance error messages with configuration context and fix suggestions.

**Rationale:**
- Self-service debugging (agents can understand and potentially fix)
- Reduces support burden
- Follows error message best practices (what, why, how to fix)

**Implementation:**
```rust
// When dimension mismatch detected:
EmbeddingError::DimensionMismatch {
    expected: 1536,
    actual: 1024,
    message: format!(
        "Dimension mismatch: expected {} but got {}.\n\n\
         This usually means the embedding provider configuration doesn't match the actual provider.\n\
         Current config: provider={}, dimension={}\n\n\
         Solutions:\n\
         1. Set MAPROOM_EMBEDDING_PROVIDER=ollama if using Ollama\n\
         2. Set MAPROOM_EMBEDDING_DIMENSION=1024 to match your model\n\
         3. Use --skip-embeddings to scan without generating embeddings",
        expected, actual, provider_name, dimension
    )
}
```

### Decision 4: Scan --skip-embeddings Flag (Rename Existing)

**Context:** Users need ability to scan without embedding generation when there are config issues.

**Decision:** The `--generate-embeddings` flag already exists (defaults to true). Document better and potentially add `--skip-embeddings` alias for clarity.

**Rationale:**
- Existing flag serves this purpose
- Better documentation in help text
- Alias improves discoverability

**Implementation:**
```rust
// Already exists at main.rs:316-317
/// Automatically generate embeddings after scanning (default: true)
#[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
generate_embeddings: bool,

// Usage: --generate-embeddings=false or --no-generate-embeddings
```

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Config API | New `from_env_with_provider()` | Backward compatible, type-safe |
| Error format | Structured message with context | Agent-parseable, actionable |
| Skill format | Markdown with YAML frontmatter | Plugin convention |
| Test framework | Existing (serial_test, tokio) | Consistency |

## Component Design

### Component 1: EmbeddingConfig Extension

**Responsibilities:**
- Load configuration from environment variables
- Accept optional provider override from caller
- Infer dimension based on provider and model

**Interface:**
```rust
impl EmbeddingConfig {
    pub fn new() -> Self;
    pub fn default() -> Self;
    pub fn from_env() -> Result<Self, EmbeddingError>;
    // NEW
    pub fn from_env_with_provider(provider: Option<Provider>) -> Result<Self, EmbeddingError>;
    pub fn validate(&self) -> Result<(), ConfigError>;
}
```

**Changes:**
- Add `from_env_with_provider()` method
- Refactor `from_env()` to call `from_env_with_provider(None)`
- Keep all existing behavior for backward compatibility

### Component 2: Factory Provider Coordination

**Responsibilities:**
- Detect available embedding providers
- Create provider instances with correct configuration
- Propagate detected provider to config

**Changes:**
```rust
// factory.rs - Ollama branch
"ollama" => {
    // BEFORE (bug)
    let config = EmbeddingConfig::from_env()?;

    // AFTER (fix)
    let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama))?;

    // Rest unchanged
    let dimension = config.dimension;  // Now correctly 1024 for mxbai-embed-large
}
```

### Component 3: Skill Documentation

**SKILL.md (Tier 1 - Brief)**

Target: Under 50 lines, agent-scannable.

```markdown
---
name: maproom-search
description: Semantic code search for exploring unfamiliar codebases and finding implementations by concept.
---

# Maproom Search

Semantic code search using SQLite FTS and optional vector embeddings.

## When to Use

| Tool | Use Case |
|------|----------|
| maproom | Find code by concept ("authentication", "error handling") |
| Grep | Exact text/regex matches |
| Glob | File path patterns |

## Quick Reference

```bash
# Check indexed repositories
crewchief-maproom status

# Full-text search
crewchief-maproom search --repo <repo> --query "<query>"

# Vector search (requires embeddings)
crewchief-maproom vector-search --repo <repo> --query "<query>"

# Get context for a chunk
crewchief-maproom context --chunk-id <id>
```

## Learn More

- [Search Best Practices](./references/search-best-practices.md) - Query patterns and strategies
- [CLI Reference](./references/cli-reference.md) - Complete command documentation
- [Troubleshooting](./references/troubleshooting.md) - Common errors and solutions
```

**references/troubleshooting.md (Tier 2 - New)**

```markdown
# Maproom Troubleshooting

## Common Errors

### Dimension Mismatch
Error: `expected 1536 dimensions but got 1024`

**Cause:** Embedding provider configuration doesn't match actual provider.

**Solution:**
1. Check current config: `crewchief-maproom status --json`
2. Set provider explicitly: `export MAPROOM_EMBEDDING_PROVIDER=ollama`
3. Or skip embeddings: `crewchief-maproom scan --generate-embeddings=false`

### Repository Not Found
...

### Vector Search Unavailable
...
```

### Component 4: CLI Error Enhancement

**Responsibilities:**
- Provide actionable error messages
- Include configuration context
- Suggest fixes

**Location:** Error handling in `crates/maproom/src/embedding/error.rs` or provider implementations.

## Data Flow

### Fixed Auto-Detection Flow

```
create_provider_from_env()
    |
    +-- detect_ollama_endpoint() --> Some("http://localhost:11434")
    |
    +-- provider_name = "ollama"
    |
    +-- EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama))
    |       |
    |       +-- config.provider = Provider::Ollama (from override)
    |       +-- config.model = "mxbai-embed-large" (default for Ollama)
    |       +-- infer_ollama_dimension("mxbai-embed-large") --> 1024
    |       +-- config.dimension = 1024 (CORRECT!)
    |
    +-- OllamaProvider::new_with_config(..., dimension=1024, ...)
            |
            +-- Ollama returns 1024-dim vectors
            +-- SUCCESS!
```

### Progressive Disclosure Information Flow

```
Agent needs maproom help
    |
    +-- Reads SKILL.md (50 lines)
    |       |
    |       +-- Understands: semantic search for code exploration
    |       +-- Knows: when to use vs grep/glob
    |       +-- Has: 4 most common commands
    |
    +-- Needs more? --> references/search-best-practices.md
    |                       |
    |                       +-- Query transformation patterns
    |                       +-- Task-specific strategies
    |                       +-- Anti-patterns to avoid
    |
    +-- Has error? --> references/troubleshooting.md
                           |
                           +-- Error diagnosis
                           +-- Solution steps
                           +-- Workarounds
```

## Integration Points

### Existing Systems

| System | Integration | Impact |
|--------|-------------|--------|
| `from_env()` | Calls `from_env_with_provider(None)` | Backward compatible |
| Factory tests | Add test for auto-detection flow | New coverage |
| CLI --generate-embeddings | Already exists | Better documentation |
| Skills structure | New references/ files | Additive |

### Plugin System

Skills follow claude-code-plugins conventions:
- `SKILL.md` at skill root with YAML frontmatter
- `references/` directory for supporting docs
- No changes to plugin.json needed

## Performance Considerations

**No performance impact:**
- Config loading is startup-only
- Provider propagation is a single enum value
- Skill documentation is static files
- No runtime overhead

## Maintainability

### Code Organization

```
crates/maproom/src/embedding/
  config.rs        # EmbeddingConfig with new constructor
  factory.rs       # Provider creation with fixed coordination
  error.rs         # Enhanced error messages

.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/
  SKILL.md                    # Brief, agent-optimized
  references/
    search-best-practices.md  # Existing (unchanged)
    cli-reference.md          # NEW: complete CLI docs
    troubleshooting.md        # NEW: error recovery
```

### Testing Strategy

1. **Unit tests** for `from_env_with_provider()`
2. **Integration test** for full auto-detection flow
3. **Skill validation** via agent test session

### Documentation Updates

- `crates/maproom/CLAUDE.md` - Note the fix and new config API
- Skill docs - Self-documenting via restructure
- Error messages - Self-documenting via improved text
