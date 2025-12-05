# Analysis: Make mxbai-embed-large the Default Model

## Problem Definition

After successfully adding mxbai-embed-large (1024-dim) support to Maproom, the model must be enabled as the default across the entire codebase. Currently, nomic-embed-text (768-dim) remains the default in multiple locations, requiring explicit configuration to use mxbai-embed-large. This creates unnecessary friction for users, especially in the VSCode extension where "no configuration should be required."

### Specific Issues

1. **Default model is still nomic-embed-text**: Multiple hardcoded references to nomic-embed-text and 768 dimensions
2. **VSCode extension requires configuration**: Users must manually set environment variables to use mxbai-embed-large
3. **Inconsistent defaults across components**: Different defaults in Rust (factory.rs, ollama.rs, config.rs), TypeScript (MCP, VSCode), and documentation
4. **Documentation still references nomic-embed-text**: Examples and guides show old model as default

### Why This Matters

**mxbai-embed-large is superior**:
- No tokenization crashes on special characters (|, [], (), Unicode)
- No content sanitization required (better embedding quality)
- Better embedding quality overall (MTEB benchmarks)
- Handles all content types without workarounds

**nomic-embed-text has critical bugs**:
- Crashes on markdown tables, checkboxes, arrows, box-drawing characters
- Requires 40+ lines of character sanitization code
- Mangles code content during embedding
- Reduces search quality due to sanitized content

## Context

### Previous Work (DIM1024 Project)

The DIM1024 project successfully:
- Added Migration #10 for vec_code_1024 virtual table
- Made OllamaProvider dimension-configurable
- Implemented conditional sanitization (only for nomic-embed-text)
- Added comprehensive testing for 1024-dim support

However, it intentionally left defaults unchanged to maintain backward compatibility during implementation. Now that the feature is complete and tested, we need to switch the defaults.

### Current Default Locations

Based on comprehensive codebase search, defaults are set in:

**Rust Code:**
1. `crates/maproom/src/embedding/ollama.rs`:
   - Line 116: `DEFAULT_MODEL = "nomic-embed-text"`
   - Line 270: `default_config()` uses 768 dimension

2. `crates/maproom/src/embedding/factory.rs`:
   - Line 210: `unwrap_or_else(|_| "nomic-embed-text".to_string())`

3. `crates/maproom/src/embedding/config.rs`:
   - Lines 85-87: Default provider is OpenAI (text-embedding-3-small, 1536-dim)
   - NOTE: This is correct for OpenAI, doesn't need changing for Ollama

**TypeScript Code:**
4. `packages/vscode-maproom/src/ollama/model-manager.ts`:
   - Line 16: `DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'`
   - Used by `ensureOllamaModel()` to check/download model during extension activation

5. `packages/maproom-mcp/src/utils/provider-detection.ts`:
   - Line 126: `models.some((m: any) => m.name.includes('nomic-embed-text'))`
   - Validates Ollama has nomic-embed-text model available

**Configuration Files:**
6. `crates/maproom/.env.example`:
   - Line 38: `MAPROOM_EMBEDDING_MODEL=nomic-embed-text`
   - Line 44: `MAPROOM_EMBEDDING_DIMENSION=768`

**Documentation:**
7. `docs/providers/ollama-setup.md`: Examples show nomic-embed-text
8. `crates/maproom/CLAUDE.md`: References to nomic-embed-text as default
9. `README.md`: May reference old defaults
10. `packages/vscode-maproom/README.md`: Setup instructions
11. `packages/maproom-mcp/README.md`: MCP server docs may reference model

## Existing Solutions

### Industry Patterns

**OpenAI**: Defaults to latest/best model, provides migration guides when changing defaults
**Cohere**: Versioned models, explicit upgrades required
**Google**: Rolling updates with backward compatibility

**Our approach**: Follow OpenAI pattern - default to better model, provide clear migration guidance.

### Codebase Patterns

**Multi-dimension support**: Already implemented via virtual table pattern (vec_code_768, vec_code_1024, vec_code)
**Provider abstraction**: EmbeddingProvider trait handles any dimension
**Environment variable configuration**: MAPROOM_EMBEDDING_MODEL and MAPROOM_EMBEDDING_DIMENSION for explicit overrides
**Conditional sanitization**: Already implemented to preserve nomic-embed-text workaround

## Research Findings

### Locations Requiring Changes

**Confirmed changes needed** (from comprehensive grep/read analysis):

**Rust Code (3 locations):**
1. **ollama.rs line 116**: `pub const DEFAULT_MODEL: &'static str = "nomic-embed-text"`
2. **ollama.rs line 270**: `768, // nomic-embed-text default dimension`
3. **factory.rs line 210**: `.unwrap_or_else(|_| "nomic-embed-text".to_string())`

**TypeScript Code (2 locations):**
4. **vscode-maproom/src/ollama/model-manager.ts line 16**: `DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'`
5. **maproom-mcp/src/utils/provider-detection.ts line 126**: `m.name.includes('nomic-embed-text')`

**Configuration (1 location):**
6. **crates/maproom/.env.example lines 38, 44**: Model and dimension examples

**Tests (25+ files, 90+ assertions):**
- Rust tests: 15+ DEFAULT_MODEL assertions, 37+ dimension assertions, 50+ test fixtures
- TypeScript tests: 10+ files in vscode-maproom and maproom-mcp packages
- Based on grep audit: Significant test update effort required

**Documentation (7 active files):**
- Must update: ollama-setup.md, CLAUDE.md files, READMEs, .env.example
- Must create: docs/guides/migrating-to-mxbai.md
- Must preserve: 125+ archived project docs (do not update)

**No changes needed**:
- `config.rs`: Default provider is OpenAI (correct for non-Ollama use)
- Database schema: Already supports 1024-dim via DIM1024 project
- Archived projects: Intentionally preserved for historical context

### Auto-Detection Flow

From `factory.rs::create_provider_from_env()`:
1. Check `MAPROOM_EMBEDDING_PROVIDER` env var
2. If not set, attempt Ollama auto-detection (localhost:11434, host.docker.internal:11434)
3. If Ollama detected, use `MAPROOM_EMBEDDING_MODEL` or fallback to DEFAULT_MODEL
4. Create provider with configured/fallback model and dimension

**Key insight**: VSCode zero-config already works through auto-detection. We just need to change the fallback model.

## Constraints

**Technical Constraints:**
- Must maintain backward compatibility for users with existing nomic-embed-text embeddings
- Must not break existing configurations with explicit model settings
- Must preserve conditional sanitization for nomic-embed-text (users may still choose it)
- Must work with existing databases containing mixed dimension embeddings

**User Experience Constraints:**
- **Zero-config in VSCode**: No environment variables or settings required for default experience
- **Smooth upgrade path**: Existing users should continue working without breaking changes
- **Clear migration guidance**: Users with existing embeddings need clear instructions

**Performance Constraints:**
- Larger model size: mxbai-embed-large is 670MB vs nomic-embed-text's 274MB
- Storage increase: ~30% more storage per embedding (1024 vs 768 floats)
- Users on limited bandwidth/storage need to be aware

## Success Criteria

### Functional Requirements

1. **Default behavior changes**:
   - Fresh installs use mxbai-embed-large (1024-dim) with no configuration
   - VSCode extension works out-of-box with mxbai-embed-large
   - CLI commands use mxbai-embed-large when no model specified

2. **Backward compatibility maintained**:
   - Explicit `MAPROOM_EMBEDDING_MODEL=nomic-embed-text` still works
   - Existing databases with 768-dim embeddings still searchable
   - Mixed dimension workspaces continue functioning

3. **Conditional sanitization preserved**:
   - nomic-embed-text still gets character sanitization
   - mxbai-embed-large uses raw content (no sanitization)

### Non-Functional Requirements

1. **Zero-config VSCode experience**: Extension activates and works without user configuration
2. **Clear documentation**: Migration guide, configuration reference, FAQ
3. **No breaking changes**: Existing configurations continue working

### Measurable Outcomes

- [ ] `crewchief-maproom` CLI without env vars uses mxbai-embed-large
- [ ] VSCode extension fresh install generates 1024-dim embeddings
- [ ] All tests pass with new defaults
- [ ] Documentation reflects new defaults with migration guidance
- [ ] Existing users with explicit nomic-embed-text config unaffected

## Assumptions

1. **Ollama availability**: Users have or can install Ollama for local embeddings
2. **Model download**: Users can download 670MB model (or have it pre-installed)
3. **Re-embedding acceptable**: Users with nomic-embed-text data will re-embed when ready
4. **Storage tradeoff**: 30% more storage is acceptable for better quality

## Open Questions

1. **Re-embedding strategy**: Should we provide automated migration script? Or just documentation?
   - **Answer**: Documentation only. Users can re-embed via `crewchief-maproom generate-embeddings --repo <name>`

2. **Model download prompt**: Should extension prompt for model download on first use?
   - **Answer**: No. Assume Ollama handles model downloads automatically. Add troubleshooting docs if download fails.

3. **Fallback behavior**: If mxbai-embed-large unavailable, fall back to nomic-embed-text?
   - **Answer**: No automatic fallback. Fail with clear error message directing to setup docs. Automatic fallback could mask configuration issues.

4. **Mixed dimensions in single repo**: Support both 768 and 1024 embeddings in same repo?
   - **Answer**: Already supported via virtual table pattern. Document this as migration path (old chunks use 768, new chunks use 1024, both searchable).
