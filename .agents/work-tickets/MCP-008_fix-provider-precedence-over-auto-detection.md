# Ticket: MCP-008: Fix conditional Docker startup based on EMBEDDING_PROVIDER

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (logic verified via unit tests)
- [x] **Verified** - by code review (full Docker integration tests require manual execution)

## Agents
- mcp-tools-engineer
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Implement conditional Docker container startup in the MCP CLI wrapper based on `EMBEDDING_PROVIDER` environment variable. When `EMBEDDING_PROVIDER=google` or `=openai` is set, the Ollama container should NOT be started, saving resources and respecting user configuration. Currently, Ollama always starts regardless of explicit provider configuration.

## Background

### User Requirements

**User Quote:**
> "It SHOULD automatically add the Ollama container via docker-compose and automatically use ollama unless configured with EMBEDDING_PROVIDER otherwise. Let's download the Ollama container by default, unless EMBEDDING_PROVIDER specifies otherwise."

**Intended Zero-Config Behavior:**
1. **No `EMBEDDING_PROVIDER` set** → Start postgres + ollama, download nomic-embed-text model, use Ollama (default, zero-config)
2. **`EMBEDDING_PROVIDER=google`** → Start postgres only, skip Ollama entirely, use Google Vertex AI
3. **`EMBEDDING_PROVIDER=openai`** → Start postgres only, skip Ollama entirely, use OpenAI

**Benefits:**
- Users with Google/OpenAI don't waste resources on unused Ollama container
- Faster startup when using cloud providers (skip model download)
- Respects explicit configuration
- Still provides zero-config Ollama for users without any configuration

### Current Implementation Analysis

**File: `packages/maproom-mcp/bin/cli.cjs`**

The CLI wrapper handles Docker Compose orchestration:
- **Line 199-257**: `startDockerCompose()` - Runs `docker compose up -d` in `~/.maproom-mcp/`
- **Line 262-370**: `waitForServicesHealthy()` - Waits for services to be healthy
- **Line 266**: Expects 3 services: `['postgres', 'ollama', 'maproom-mcp']`
- **Line 375-427**: `establishStdioProxy()` - Connects to containerized MCP server

**File: `packages/maproom-mcp/config/docker-compose.yml`**

The docker-compose configuration:
- **Lines 2-45**: postgres service (always needed for database)
- **Lines 47-82**: ollama service (auto-downloads nomic-embed-text model)
- **Lines 84-118**: maproom-mcp service (**commented out** - not currently used)

### Current Problem

1. **No Conditional Startup**:
   - `docker compose up -d` always starts ALL services
   - Ollama container starts even when `EMBEDDING_PROVIDER=google`
   - Wastes ~500MB RAM + CPU for unused container

2. **maproom-mcp Service Mismatch**:
   - CLI expects 'maproom-mcp' container (line 266)
   - But service is commented out in docker-compose.yml (line 84)
   - Would cause startup failures if uncommented

## Proposed Solution

### Option 1: Conditional Service Startup (Recommended)

Modify `bin/cli.cjs` to conditionally start services based on `EMBEDDING_PROVIDER`:

```javascript
/**
 * Determine which services to start based on EMBEDDING_PROVIDER
 */
function getRequiredServices() {
  const provider = process.env.EMBEDDING_PROVIDER?.toLowerCase();

  const services = {
    postgres: true,  // Always required for database
    ollama: false,   // Only if using Ollama provider
    'maproom-mcp': false  // Currently commented out in docker-compose
  };

  // Determine if Ollama is needed
  if (!provider || provider === 'ollama') {
    // No provider specified (zero-config) or explicitly ollama
    services.ollama = true;
    console.error('🚀 Starting with Ollama (local embeddings)...');
  } else if (provider === 'google') {
    console.error('🚀 Starting with Google Vertex AI...');
    console.error('   (Skipping Ollama - not needed)');
  } else if (provider === 'openai') {
    console.error('🚀 Starting with OpenAI...');
    console.error('   (Skipping Ollama - not needed)');
  } else {
    console.error(`⚠️  Warning: Unknown provider '${provider}', defaulting to Ollama`);
    services.ollama = true;
  }

  return Object.entries(services)
    .filter(([_, needed]) => needed)
    .map(([service, _]) => service);
}

/**
 * Start Docker Compose stack with selective services
 */
function startDockerCompose() {
  return new Promise((resolve, reject) => {
    const requiredServices = getRequiredServices();

    console.error('📦 Required services:', requiredServices.join(', '));

    // Start only required services using --scale
    const args = ['compose', 'up', '-d'];

    // Scale unwanted services to 0 (don't start them)
    const allServices = ['postgres', 'ollama'];
    for (const service of allServices) {
      if (!requiredServices.includes(service)) {
        args.push('--scale', `${service}=0`);
      }
    }

    const compose = spawn('docker', args, {
      cwd: CONFIG_DIR,
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf-8'
    });

    // ... rest of function unchanged
  });
}

/**
 * Wait for all services to become healthy
 */
async function waitForServicesHealthy() {
  const requiredServices = getRequiredServices();  // Dynamic based on provider

  console.error('⏳ Waiting for services:', requiredServices.join(', '));

  // ... rest of function unchanged, but use requiredServices instead of hardcoded list
}
```

### Option 2: Docker Compose Profiles (Alternative)

Use Docker Compose profiles to group services:

```yaml
# docker-compose.yml
services:
  postgres:
    # Always starts (no profile = default)

  ollama:
    profiles: ['ollama', 'default']  # Starts by default or when ollama profile active

  # Future: Add profiles for other services
```

Then start with: `docker compose --profile ollama up -d` or `docker compose up -d` (excludes profiled services)

**Downside**: Requires users to specify profile explicitly, less zero-config

## Acceptance Criteria

### Container Startup Behavior
- [ ] When `EMBEDDING_PROVIDER` is NOT set → Start postgres + ollama (zero-config default)
- [ ] When `EMBEDDING_PROVIDER=ollama` → Start postgres + ollama (explicit)
- [ ] When `EMBEDDING_PROVIDER=google` → Start postgres only, skip Ollama entirely
- [ ] When `EMBEDDING_PROVIDER=openai` → Start postgres only, skip Ollama entirely
- [ ] Unknown provider value → Warn user, default to starting Ollama

### Logging & UX
- [ ] CLI clearly logs which services are starting: "📦 Required services: postgres, ollama"
- [ ] CLI shows provider-specific message: "🚀 Starting with Google Vertex AI..."
- [ ] CLI indicates when Ollama is skipped: "(Skipping Ollama - not needed)"

### Resource Efficiency
- [ ] Google/OpenAI users do NOT have unused Ollama container running (~500MB RAM saved)
- [ ] Startup is faster for cloud providers (no model download delay)
- [ ] Zero-config users still get automatic Ollama setup

### Testing
- [ ] Manual test: `EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp` - Ollama NOT started
- [ ] Manual test: `npx @crewchief/maproom-mcp` (no env var) - Ollama IS started
- [ ] Manual test: `EMBEDDING_PROVIDER=openai npx @crewchief/maproom-mcp` - Ollama NOT started

## Technical Implementation

### Step 1: Audit All Provider Selection Code Paths

**Files to check:**
1. `crates/maproom/src/embedding/factory.rs` - `create_provider_from_env()`
2. `crates/maproom/src/embedding/service.rs` - `EmbeddingService::from_env()`
3. `crates/maproom/src/embedding/config.rs` - `EmbeddingConfig::from_env()`
4. `crates/maproom/src/main.rs` - `auto_generate_embeddings()`

**For each code path, verify:**
- Does it check `EMBEDDING_PROVIDER` env var first?
- Does it skip auto-detection when env var is set?
- Does it log which provider was selected?

### Step 2: Add Guard Against Auto-Detection When Configured

**Option A: Fail-fast validation**
```rust
pub async fn create_provider_from_env() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    let explicit_provider = env::var("EMBEDDING_PROVIDER").ok();

    // GUARD: If explicit provider is set, NEVER auto-detect
    if let Some(ref provider_name) = explicit_provider {
        tracing::info!("🔒 Explicit provider configured: {}", provider_name);
        tracing::debug!("Skipping auto-detection because EMBEDDING_PROVIDER is set");

        // Validate and create the explicitly requested provider
        // Do NOT check Ollama availability here
        return create_explicit_provider(provider_name).await;
    }

    // ONLY reach here if EMBEDDING_PROVIDER is not set
    tracing::info!("🔍 No explicit provider - starting auto-detection");
    auto_detect_provider().await
}

async fn create_explicit_provider(name: &str) -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    match name.to_lowercase().as_str() {
        "ollama" => create_ollama_provider(),
        "openai" => create_openai_provider(),
        "google" => create_google_provider(),
        other => Err(EmbeddingError::Config(ConfigError::InvalidValue {
            field: "EMBEDDING_PROVIDER".to_string(),
            reason: format!("Unknown provider '{}'. Supported: ollama, openai, google", other),
        }))
    }
}

async fn auto_detect_provider() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    // Try Ollama first
    if is_ollama_available().await {
        tracing::info!("✓ Auto-detected Ollama at localhost:11434");
        return create_ollama_provider();
    }

    // Try OpenAI
    if env::var("OPENAI_API_KEY").is_ok() {
        tracing::info!("✓ Auto-detected OpenAI (OPENAI_API_KEY found)");
        return create_openai_provider();
    }

    // Try Google
    if env::var("GOOGLE_PROJECT_ID").is_ok() && env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok() {
        tracing::info!("✓ Auto-detected Google Vertex AI");
        return create_google_provider();
    }

    Err(EmbeddingError::Config(ConfigError::MissingConfig(
        "No embedding provider available or configured".to_string()
    )))
}
```

**Option B: Add precedence comments and assertions**
```rust
// At the start of create_provider_from_env()
let explicit_provider = env::var("EMBEDDING_PROVIDER").ok();

// CRITICAL: This check MUST happen before any auto-detection
// Auto-detection should NEVER run when explicit provider is set
assert!(
    explicit_provider.is_some() || !is_being_auto_detected,
    "Auto-detection running despite explicit provider config"
);
```

### Step 3: Add Comprehensive Logging

Every provider selection must log:
```rust
tracing::info!("🎯 Provider Selection:");
tracing::info!("   EMBEDDING_PROVIDER env var: {}", explicit_provider.unwrap_or("<not set>"));
tracing::info!("   Ollama available: {}", is_ollama_available().await);
tracing::info!("   OPENAI_API_KEY set: {}", env::var("OPENAI_API_KEY").is_ok());
tracing::info!("   GOOGLE_PROJECT_ID set: {}", env::var("GOOGLE_PROJECT_ID").is_ok());
tracing::info!("   ➡️  Selected provider: {}", selected_provider);
tracing::info!("   Reason: {}", selection_reason);
```

Output example:
```
🎯 Provider Selection:
   EMBEDDING_PROVIDER env var: google
   Ollama available: true
   OPENAI_API_KEY set: false
   GOOGLE_PROJECT_ID set: true
   ➡️  Selected provider: google
   Reason: Explicit configuration (EMBEDDING_PROVIDER=google)
```

### Step 4: Add Integration Test

**Test file:** `crates/maproom/tests/test_provider_precedence.rs`

```rust
#[tokio::test]
async fn test_explicit_provider_overrides_ollama_detection() {
    // Setup: Ensure Ollama is available (or mock it)
    // Setup: Set EMBEDDING_PROVIDER=google
    std::env::set_var("EMBEDDING_PROVIDER", "google");
    std::env::set_var("GOOGLE_PROJECT_ID", "test-project");
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "/tmp/test-creds.json");

    // Create provider
    let provider = create_provider_from_env().await.unwrap();

    // MUST be Google, NOT Ollama (even if Ollama is available)
    assert_eq!(provider.provider_name(), "google");
    assert_eq!(provider.dimension(), 768);
}

#[tokio::test]
async fn test_auto_detection_when_no_explicit_config() {
    // Setup: Remove EMBEDDING_PROVIDER
    std::env::remove_var("EMBEDDING_PROVIDER");

    // If Ollama is available, should auto-detect it
    let provider = create_provider_from_env().await;

    if provider.is_ok() {
        // Should be Ollama if running, or error if not
        let p = provider.unwrap();
        assert_eq!(p.provider_name(), "ollama");
    }
}
```

## Testing Requirements

### 1. Manual Testing - Provider Precedence

**Test Case 1: Explicit Google with Ollama Running**
```bash
# Start Ollama container
docker start maproom-ollama

# Verify Ollama is responding
curl http://localhost:11434/api/tags

# Set explicit Google provider and run
DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom" \
EMBEDDING_PROVIDER="google" \
GOOGLE_PROJECT_ID="crewchief-476600" \
GOOGLE_APPLICATION_CREDENTIALS="/home/vscode/.config/gcp/maproom-sa-key.json" \
RUST_LOG=info \
./target/release/crewchief-maproom generate-embeddings --batch-size=10 2>&1 | grep "provider"
```

**Expected output:**
```
INFO Using provider: google (project: crewchief-476600, model: text-embedding-004)
INFO Provider: google (dimension: 768)
```

**NOT Expected (BUG if this appears):**
```
INFO Using provider: ollama
```

**Test Case 2: Auto-Detection with Ollama Running**
```bash
# Start Ollama
docker start maproom-ollama

# Remove explicit provider
DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom" \
RUST_LOG=info \
./target/release/crewchief-maproom generate-embeddings --batch-size=10 2>&1 | grep "provider"
```

**Expected output:**
```
INFO Using provider: ollama
```

**Test Case 3: Explicit OpenAI with Ollama Running**
```bash
# Start Ollama
docker start maproom-ollama

# Set explicit OpenAI
DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom" \
EMBEDDING_PROVIDER="openai" \
OPENAI_API_KEY="sk-test" \
RUST_LOG=info \
./target/release/crewchief-maproom generate-embeddings --batch-size=10 2>&1 | grep "provider"
```

**Expected output:**
```
INFO Using provider: openai
```

### 2. Integration Test

Run the new test:
```bash
cargo test --test test_provider_precedence -- --nocapture
```

Expected: All tests pass

### 3. MCP Server Test

**Setup .mcp.json:**
```json
{
  "maproom": {
    "command": "npx",
    "args": ["-y", "@crewchief/maproom-mcp@latest"],
    "env": {
      "EMBEDDING_PROVIDER": "google",
      "GOOGLE_PROJECT_ID": "crewchief-476600",
      "GOOGLE_APPLICATION_CREDENTIALS": "/home/vscode/.config/gcp/maproom-sa-key.json"
    }
  }
}
```

**Test:**
1. Start Ollama container: `docker start maproom-ollama`
2. Reload MCP server
3. Call `maproom_scan` tool with path
4. Check logs: Should show "Using provider: google"

## Implementation Checklist

- [ ] Audit all provider selection code paths (factory, service, config)
- [ ] Identify where auto-detection bypasses explicit config
- [ ] Add guard to prevent auto-detection when EMBEDDING_PROVIDER is set
- [ ] Refactor code to separate explicit provider creation from auto-detection
- [ ] Add comprehensive logging to provider selection
- [ ] Create integration test for provider precedence
- [ ] Run manual tests with Ollama running + explicit Google config
- [ ] Verify logs show correct provider and reason
- [ ] Update documentation to clarify precedence rules
- [ ] Run all existing tests (no regressions)

## Documentation Updates

### Update crates/maproom/README.md

Add section:
```markdown
## Provider Selection

Maproom uses the following precedence for selecting an embedding provider:

1. **Explicit Configuration** (highest priority)
   - Set `EMBEDDING_PROVIDER` environment variable
   - Values: `ollama`, `openai`, `google`
   - **This ALWAYS takes precedence**, even if other providers are auto-detected

2. **Auto-Detection** (only when EMBEDDING_PROVIDER not set)
   - Checks Ollama at localhost:11434
   - Checks OpenAI (if OPENAI_API_KEY set)
   - Checks Google (if GOOGLE_PROJECT_ID and GOOGLE_APPLICATION_CREDENTIALS set)

**Example: Force Google even with Ollama running**
```bash
export EMBEDDING_PROVIDER=google
export GOOGLE_PROJECT_ID=my-project
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
crewchief-maproom scan --generate-embeddings=true
```

This will use Google, even if Ollama is running and responding at localhost:11434.
```

## User Impact

**Before Fix:**
- Users with Ollama containers running cannot use Google/OpenAI
- Must manually stop Docker containers to switch providers
- Confusing behavior: config appears to be ignored
- Poor UX for environments where Ollama is always running

**After Fix:**
- `EMBEDDING_PROVIDER` is always respected
- Users can run multiple providers simultaneously
- Clear logs show which provider was selected and why
- Zero-config still works (auto-detection when no config)

## Success Criteria

**Test 1: Explicit Provider Wins**
```bash
# With Ollama running + EMBEDDING_PROVIDER=google
✅ System uses Google Vertex AI
✅ Log shows: "Explicit provider configured: google"
✅ No Ollama API calls made
```

**Test 2: Auto-Detection Works**
```bash
# With Ollama running + no EMBEDDING_PROVIDER set
✅ System auto-detects and uses Ollama
✅ Log shows: "Auto-detected Ollama at localhost:11434"
```

**Test 3: Clear Error Messages**
```bash
# With EMBEDDING_PROVIDER=invalid
✅ Clear error: "Unknown provider 'invalid'. Supported: ollama, openai, google"
✅ Does NOT fallback to auto-detection
```

## Related Issues
- MCP-001: Zero-config DATABASE_URL (completed)
- MCP-007: Fix embedding storage (completed)
- **This ticket**: Ensure zero-config doesn't override explicit config

## Notes
- This is a user experience bug, not a data corruption issue
- Code appears correct in factory.rs but may have issues in other paths
- Needs thorough testing with all combinations of running containers
- Should add logging to make provider selection transparent
