# Architecture: Make mxbai-embed-large the Default Model

## Overview

Change all default model references from nomic-embed-text (768-dim) to mxbai-embed-large (1024-dim) across Rust code and documentation. This is a configuration change, not an architectural change - the multi-dimension infrastructure already exists from the DIM1024 project.

**Key Principle**: Update defaults in a single, coordinated change. No new features, no schema changes, just configuration updates and documentation.

## Design Decisions

### Decision 1: Update Rust DEFAULT_MODEL Constant

**Context**: OllamaProvider has hardcoded `DEFAULT_MODEL = "nomic-embed-text"`.

**Decision**: Change `DEFAULT_MODEL` from "nomic-embed-text" to "mxbai-embed-large".

**Location**: `crates/maproom/src/embedding/ollama.rs` line 116

**Before**:
```rust
pub const DEFAULT_MODEL: &'static str = "nomic-embed-text";
```

**After**:
```rust
pub const DEFAULT_MODEL: &'static str = "mxbai-embed-large";
```

**Rationale**:
- Single source of truth for Ollama defaults
- Used by `default_config()` and factory fallback
- Affects CLI, daemon, and VSCode behavior

### Decision 2: Update default_config() Dimension

**Context**: `default_config()` method hardcodes dimension=768 for nomic-embed-text.

**Decision**: Change dimension from 768 to 1024 to match mxbai-embed-large.

**Location**: `crates/maproom/src/embedding/ollama.rs` line 270

**Before**:
```rust
pub fn default_config() -> Result<Self, EmbeddingError> {
    Self::new(
        Self::DEFAULT_ENDPOINT.to_string(),
        Self::DEFAULT_MODEL.to_string(),
        768, // nomic-embed-text default dimension
    )
}
```

**After**:
```rust
pub fn default_config() -> Result<Self, EmbeddingError> {
    Self::new(
        Self::DEFAULT_ENDPOINT.to_string(),
        Self::DEFAULT_MODEL.to_string(),
        1024, // mxbai-embed-large default dimension
    )
}
```

**Rationale**:
- Must match DEFAULT_MODEL's actual dimension
- Prevents dimension mismatch errors
- Ensures zero-config generates correct embeddings

### Decision 3: Update Factory Fallback Model

**Context**: factory.rs fallback when `MAPROOM_EMBEDDING_MODEL` env var not set.

**Decision**: Change fallback from "nomic-embed-text" to "mxbai-embed-large".

**Location**: `crates/maproom/src/embedding/factory.rs` line 210

**Before**:
```rust
let model = env::var("MAPROOM_EMBEDDING_MODEL")
    .unwrap_or_else(|_| "nomic-embed-text".to_string());
```

**After**:
```rust
let model = env::var("MAPROOM_EMBEDDING_MODEL")
    .unwrap_or_else(|_| "mxbai-embed-large".to_string());
```

**Rationale**:
- Consistency with OllamaProvider::DEFAULT_MODEL
- Affects auto-detection path (most common user scenario)
- Enables zero-config VSCode experience

### Decision 4: Preserve EmbeddingConfig Defaults

**Context**: `EmbeddingConfig::default()` has provider=OpenAI, dimension=1536.

**Decision**: NO CHANGE to config.rs defaults.

**Rationale**:
- `EmbeddingConfig::default()` is for OpenAI provider, not Ollama
- Ollama-specific defaults come from OllamaProvider, not EmbeddingConfig
- Auto-detection in factory.rs handles provider selection correctly
- Changing config.rs would break OpenAI users

### Decision 5: Update VSCode Extension Default Model

**Context**: VSCode extension has hardcoded `DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'` in model-manager.ts used during activation to check if model exists and pull if needed.

**Decision**: Change `DEFAULT_EMBEDDING_MODEL` from "nomic-embed-text" to "mxbai-embed-large".

**Location**: `packages/vscode-maproom/src/ollama/model-manager.ts` line 16

**Before**:
```typescript
export const DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'
```

**After**:
```typescript
export const DEFAULT_EMBEDDING_MODEL = 'mxbai-embed-large'
```

**Test Updates Required**:
- Update test assertion at line 359: `expect(DEFAULT_EMBEDDING_MODEL).toBe('mxbai-embed-large')`
- Update any mock expectations in model-manager.test.ts

**Rationale**:
- Extension's model manager must match daemon's default model
- Ensures `ensureOllamaModel()` downloads correct model during activation
- Maintains zero-config experience (extension, MCP, daemon all use same default)
- Prevents confusion when extension checks for different model than daemon uses

### Decision 6: Update MCP Server Model Validation

**Context**: MCP server's provider detection checks specifically for nomic-embed-text model to validate Ollama configuration.

**Decision**: Change model check from "nomic-embed-text" to "mxbai-embed-large".

**Location**: `packages/maproom-mcp/src/utils/provider-detection.ts` line 126

**Before**:
```typescript
const hasEmbedModel = models.some(
  (m: any) => m.name.includes('nomic-embed-text')
)
```

**After**:
```typescript
const hasEmbedModel = models.some(
  (m: any) => m.name.includes('mxbai-embed-large')
)
```

**Test Updates Required**:
- Update test mocks in provider-detection.test.ts (10+ test cases)
- Update warning message to suggest `ollama pull mxbai-embed-large`

**Rationale**:
- MCP server detection must validate correct default model exists
- Prevents false negatives when properly configured Ollama has mxbai but not nomic
- Ensures accurate status reporting to MCP clients
- Maintains consistency across all integration layers

### Decision 7: Update Documentation and Configuration Examples

**Context**: Multiple docs reference nomic-embed-text as default or primary example.

**Decision**: Update all documentation to show mxbai-embed-large as default, add migration guide.

**Files to update**:
1. `docs/providers/ollama-setup.md` - Change examples to mxbai-embed-large
2. `crates/maproom/CLAUDE.md` - Update default references
3. `README.md` - Update quickstart if it mentions model
4. `packages/vscode-maproom/README.md` - Update setup instructions
5. `packages/maproom-mcp/README.md` - Update MCP server docs
6. `crates/maproom/.env.example` - Update example values
7. Create `docs/guides/migrating-to-mxbai.md` - Migration guide for existing users

**Migration Guide Specification**:
- **Target Audience**: CLI users, VSCode users, MCP server users
- **Location**: `docs/guides/migrating-to-mxbai.md`
- **Required Sections**:
  1. Executive summary (why/what changed)
  2. Zero-config users (no action needed)
  3. Explicit config users (how to keep nomic-embed-text)
  4. Re-embedding guide with specific commands
  5. Storage impact calculator (33% increase)
  6. Troubleshooting FAQ (8+ common issues)
  7. Model comparison table

**Documentation Categorization**:
- **Active documentation** (7 files): Must update to reflect new defaults
- **Historical examples** (preserved): Archived project docs provide historical context
- **Archived projects** (do not touch): `.crewchief/archive/` and `.crewchief/projects/DIM1024_*`

**Rationale**:
- Documentation is the primary learning resource
- Consistency across all docs prevents confusion
- Migration guide addresses existing user concerns
- Preserving archived docs maintains historical context

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Default model | mxbai-embed-large | Better quality, no crashes, no sanitization |
| Default dimension | 1024 | Matches mxbai-embed-large output |
| Configuration method | Constants + env vars | Existing pattern, zero-config support |
| Backward compat | Explicit env vars | Users can override defaults if needed |
| Documentation | Migration guide | Clear path for existing users |

## Complete File Change List

### Code Changes (6 locations)

**Rust (3 files):**
1. `crates/maproom/src/embedding/ollama.rs` line 116: `DEFAULT_MODEL = "mxbai-embed-large"`
2. `crates/maproom/src/embedding/ollama.rs` line 270: dimension 768 → 1024
3. `crates/maproom/src/embedding/factory.rs` line 210: fallback "nomic-embed-text" → "mxbai-embed-large"

**TypeScript (2 files):**
4. `packages/vscode-maproom/src/ollama/model-manager.ts` line 16: `DEFAULT_EMBEDDING_MODEL = 'mxbai-embed-large'`
5. `packages/maproom-mcp/src/utils/provider-detection.ts` line 126: `m.name.includes('mxbai-embed-large')`

**Configuration (1 file):**
6. `crates/maproom/.env.example` lines 38, 44: Update model and dimension examples

### Test Changes (25+ files, 90+ assertions)

**Rust tests:**
- 15+ DEFAULT_MODEL assertions
- 37+ dimension assertions
- 50+ test fixtures with model references

**TypeScript tests:**
- `packages/vscode-maproom/src/ollama/model-manager.test.ts` (8+ assertions)
- `packages/maproom-mcp/tests/provider-detection.test.ts` (10+ test cases)
- Other test files with mock expectations

### Documentation Changes (7 files)

**Must update:**
1. `docs/providers/ollama-setup.md`
2. `crates/maproom/CLAUDE.md`
3. `README.md`
4. `packages/vscode-maproom/README.md`
5. `packages/maproom-mcp/README.md`
6. `crates/maproom/.env.example`
7. `docs/guides/migrating-to-mxbai.md` (new file)

**Must NOT update (preserve for historical context):**
- `.crewchief/archive/projects/**/*.md` (125+ files)
- `.crewchief/projects/DIM1024_*/**/*.md`

## Component Design

### Component 1: Rust Embedding Defaults

**Files**:
- `/workspace/crates/maproom/src/embedding/ollama.rs`
- `/workspace/crates/maproom/src/embedding/factory.rs`

**Changes**:
1. `ollama.rs` line 116: `DEFAULT_MODEL = "mxbai-embed-large"`
2. `ollama.rs` line 270: dimension from 768 to 1024
3. `factory.rs` line 210: fallback from "nomic-embed-text" to "mxbai-embed-large"`

**Interfaces**: No interface changes, only default value changes

**Testing**:
- Update tests that assert DEFAULT_MODEL == "nomic-embed-text"
- Update tests that assert default dimension == 768
- Verify factory without env vars uses mxbai-embed-large
- Verify explicit nomic-embed-text config still works

### Component 2: Test Updates

**Files**:
- `crates/maproom/src/embedding/ollama.rs` (test section)
- `crates/maproom/src/embedding/factory.rs` (test section)
- Integration tests if they hardcode model expectations

**Changes**:
- `test_ollama_provider_default_config()`: Assert DEFAULT_MODEL == "mxbai-embed-large"
- `test_ollama_provider_default_config()`: Assert dimension == 1024
- Any tests using `default_config()`: Update dimension expectations
- Add test for backward compat: explicit nomic-embed-text still works

### Component 3: Documentation

**Files**:
- `/workspace/docs/providers/ollama-setup.md`
- `/workspace/crates/maproom/CLAUDE.md`
- `/workspace/README.md`
- `/workspace/packages/vscode-maproom/README.md`
- `/workspace/docs/guides/migrating-to-mxbai.md` (new file)

**Changes**:
1. Replace nomic-embed-text examples with mxbai-embed-large
2. Update dimension references from 768 to 1024
3. Add "Using nomic-embed-text" sections for users who prefer old model
4. Create comprehensive migration guide

**Migration Guide Topics**:
- Why the change (better quality, no crashes)
- How to keep using nomic-embed-text (explicit env vars)
- How to re-embed existing content
- Storage and performance implications
- Troubleshooting common issues
- FAQ section

### Component 4: Environment Examples

**Files**:
- `/workspace/crates/maproom/.env.example`

**Changes**:
- Update example from nomic-embed-text to mxbai-embed-large
- Update example dimension from 768 to 1024
- Add comment explaining backward compatibility

## Data Flow

### VSCode Zero-Config Flow (Corrected)

**Before (Current - Inconsistent defaults):**
```
User installs VSCode extension
    ↓
Extension activates
    ↓ calls ensureOllamaModel(DEFAULT_EMBEDDING_MODEL)
    ↓ checks if "nomic-embed-text" exists  ← VSCode default
    ↓ if missing, prompts to pull model
Extension spawns MCP server
    ↓ MCP detects Ollama
    ↓ validates "nomic-embed-text" model exists  ← MCP validation
MCP server spawns Rust daemon
    ↓
Daemon calls factory::create_provider_from_env()
    ↓ (no MAPROOM_EMBEDDING_MODEL env var)
Detects Ollama at localhost:11434
    ↓
Uses fallback model "nomic-embed-text"  ← Rust default
    ↓
Creates OllamaProvider(endpoint, "nomic-embed-text", 768)
    ↓
Applies character sanitization before embedding
    ↓
Stores in vec_code_768 table
```

**After (New - Consistent defaults across all layers):**
```
User installs VSCode extension
    ↓
Extension activates
    ↓ calls ensureOllamaModel(DEFAULT_EMBEDDING_MODEL)
    ↓ checks if "mxbai-embed-large" exists  ← VSCode default CHANGED
    ↓ if missing, prompts to pull model
Extension spawns MCP server
    ↓ MCP detects Ollama
    ↓ validates "mxbai-embed-large" model exists  ← MCP validation CHANGED
MCP server spawns Rust daemon
    ↓
Daemon calls factory::create_provider_from_env()
    ↓ (no MAPROOM_EMBEDDING_MODEL env var)
Detects Ollama at localhost:11434
    ↓
Uses fallback model "mxbai-embed-large"  ← Rust default CHANGED
    ↓
Creates OllamaProvider(endpoint, "mxbai-embed-large", 1024)  ← CHANGED
    ↓
No sanitization (mxbai handles all characters)  ← CHANGED
    ↓
Stores in vec_code_1024 table  ← CHANGED
```

**Key Insight**: All three layers (VSCode extension, MCP server, Rust daemon) must use consistent defaults for true zero-config experience. Original planning missed VSCode and MCP layers.

## Integration Points

### With Existing Systems

**Database**: No schema changes required. vec_code_1024 table already exists from DIM1024 project.

**Search**: Already supports mixed dimensions. Can search across 768, 1024, and 1536 dim embeddings simultaneously.

**CLI**: Automatically picks up new defaults. No CLI code changes needed.

**MCP Server**: No TypeScript changes needed. Relies on Rust daemon defaults.

**VSCode Extension**: No extension changes needed. Relies on MCP server/daemon defaults.

### Backward Compatibility

**Explicit Configuration (No Impact)**:
```bash
# This configuration continues working unchanged
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
```

**Existing Embeddings (No Impact)**:
- vec_code_768 table remains intact
- Old embeddings still searchable
- New embeddings use vec_code_1024
- Mixed dimensions supported (already tested)

**Sanitization Logic (Preserved)**:
```rust
// In ollama.rs embed_batch_raw() - unchanged
if self.model == "nomic-embed-text" {
    // Apply sanitization workaround
    texts.map(|t| Self::sanitize_for_nomic(&t))
} else {
    // Use raw text (mxbai-embed-large and other models)
    texts  // No sanitization
}
```

## Performance Considerations

### Model Size Impact
- nomic-embed-text: 274 MB
- mxbai-embed-large: 670 MB
- **Impact**: One-time download, ~2.4x larger
- **Mitigation**: Document in migration guide, Ollama handles download

### Storage Per Embedding
- 768-dim: 768 floats × 4 bytes = 3,072 bytes
- 1024-dim: 1024 floats × 4 bytes = 4,096 bytes
- **Impact**: ~33% more storage per embedding
- **Mitigation**: Storage is cheap, quality improvement is worth it

### Search Performance
- **Impact**: None
- **Validation**: Already tested in DIM1024 project
- **Reason**: sqlite-vec handles both dimensions efficiently

### Embedding Throughput
- **Impact**: Minimal (slightly slower due to larger model)
- **Validation**: Already tested in EMBPERF project
- **Mitigation**: Parallel batch processing compensates

## Maintainability

### Code Simplicity
- **Reduced complexity**: No sanitization for default model
- **Fewer bug reports**: No special character crashes
- **Easier testing**: Raw content doesn't require sanitization test cases

### Future Additions
- Pattern established for adding new Ollama models
- Dimension-agnostic architecture allows any dimension
- Clear documentation for model comparison

### Migration Path
- Users can stay on nomic-embed-text if needed
- Mixed dimensions allow gradual migration
- Clear documentation eases transition

## Rollback Plan

If critical issues arise:

**Code Rollback** (simple):
1. Revert ollama.rs: `DEFAULT_MODEL = "nomic-embed-text"`, dimension = 768
2. Revert factory.rs: fallback to "nomic-embed-text"
3. Revert vscode-maproom/model-manager.ts: `DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'`
4. Revert maproom-mcp/provider-detection.ts: check for nomic-embed-text
5. Revert test assertions

**Documentation Rollback** (if needed):
1. Revert doc changes
2. Keep migration guide (still useful for users who switched)

**Database Rollback**: NOT NEEDED
- Both tables exist (vec_code_768 and vec_code_1024)
- All embeddings preserved
- No data loss

**Rollback Testability**:
- Rollback tested in separate branch before merge to ensure clean revert path
- Test sequence: Apply changes → verify tests pass → revert changes → verify tests still pass
- Validates rollback doesn't introduce new failures

**Risk Assessment**: Low
- Changes are configuration-only
- Full backward compatibility maintained
- No breaking changes to APIs or schemas
- Clean revert path validated
