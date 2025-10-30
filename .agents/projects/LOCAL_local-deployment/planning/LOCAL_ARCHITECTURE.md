# LOCAL: Local LLM Embedding Service - Technical Architecture

**Project Slug**: LOCAL
**Created**: 2025-10-26
**Status**: Architecture Design

## Architecture Overview

### System Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                          User's System                             │
│                                                                    │
│  .mcp.json Configuration:                                          │
│  {                                                                 │
│    "mcpServers": {                                                 │
│      "maproom": {                                                  │
│        "command": "npx",                                           │
│        "args": ["-y", "@crewchief/maproom-mcp"]                    │
│      }                                                             │
│    }                                                               │
│  }                                                                 │
│                                                                    │
│         │                                                          │
│         ▼                                                          │
│  ┌──────────────────┐                                             │
│  │  npm Package     │  (Wrapper - @crewchief/maproom-mcp)         │
│  │  CLI Wrapper     │  • Embeds docker-compose.yml                │
│  │                  │  • Runs docker-compose up -d                │
│  └────────┬─────────┘  • Proxies stdio to MCP container           │
│           │                                                        │
│           ▼                                                        │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │             Docker Compose Stack                        │     │
│  │                                                         │     │
│  │  ┌──────────┐    ┌──────────┐    ┌──────────────┐     │     │
│  │  │ Maproom  │───▶│  Ollama  │    │  PostgreSQL  │     │     │
│  │  │   MCP    │    │  Server  │    │  + pgvector  │     │     │
│  │  └────┬─────┘    └────┬─────┘    └──────┬───────┘     │     │
│  │       │               │                  │             │     │
│  │       │        nomic-embed-text          │             │     │
│  │       │          (768 dim)               │             │     │
│  │       └──────────────────────────────────┘             │     │
│  │                                                         │     │
│  │  Volumes:                                               │     │
│  │  • maproom-data → /data (PostgreSQL)                   │     │
│  │  • ollama-models → /root/.ollama (Model cache)         │     │
│  │  • maproom-config → /config (MCP config)               │     │
│  └─────────────────────────────────────────────────────────┘     │
│           │                │                │                     │
│           ▼                ▼                ▼                     │
│      Port 3000       Port 11434       Port 5432                   │
│    (MCP Server)     (Ollama API)   (Internal only)                │
└────────────────────────────────────────────────────────────────────┘
```

### Component Architecture

#### 0. npm CLI Wrapper (@crewchief/maproom-mcp)

**Purpose**: Provide zero-configuration entry point for users via npx

**Package Structure**:
```
@crewchief/maproom-mcp/
├── package.json
├── bin/
│   └── cli.js              # Main executable
├── config/
│   ├── docker-compose.yml  # Embedded compose config
│   ├── init.sql            # PostgreSQL schema
│   └── postgresql.conf     # DB tuning
└── README.md
```

**package.json**:
```json
{
  "name": "@crewchief/maproom-mcp",
  "version": "1.0.0",
  "description": "Maproom MCP server with local LLM embeddings - zero configuration required",
  "bin": {
    "maproom-mcp": "./bin/cli.js"
  },
  "scripts": {
    "test": "node bin/cli.js --test",
    "dev": "node bin/cli.js"
  },
  "keywords": ["mcp", "embeddings", "ollama", "semantic-search", "code-search"],
  "author": "CrewChief",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/crewchief/maproom-mcp.git"
  }
}
```

**CLI Wrapper (bin/cli.js)**:
```javascript
#!/usr/bin/env node
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

const CONFIG_DIR = path.join(os.homedir(), '.maproom-mcp');
const COMPOSE_FILE = path.join(CONFIG_DIR, 'docker-compose.yml');

// Ensure config directory exists
if (!fs.existsSync(CONFIG_DIR)) {
  fs.mkdirSync(CONFIG_DIR, { recursive: true });
}

// Copy docker-compose.yml to config directory
const embeddedCompose = path.join(__dirname, '..', 'config', 'docker-compose.yml');
if (!fs.existsSync(COMPOSE_FILE)) {
  fs.copyFileSync(embeddedCompose, COMPOSE_FILE);
  console.log('✅ Initialized Maproom configuration');
}

// Check if Docker is running and has Compose plugin
function checkDocker() {
  return new Promise((resolve, reject) => {
    const check = spawn('docker', ['compose', 'version']);
    check.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error('Docker Compose plugin not found. Please install Docker Desktop or Docker Compose v2.'));
    });
  });
}

// Start Docker Compose stack
async function startStack() {
  await checkDocker();

  console.log('🚀 Starting Maproom MCP with local LLM...');

  const compose = spawn('docker', [
    'compose',
    '-f', COMPOSE_FILE,
    'up', '-d'
  ], {
    cwd: CONFIG_DIR,
    stdio: 'inherit'
  });

  return new Promise((resolve, reject) => {
    compose.on('close', (code) => {
      if (code === 0) {
        console.log('✅ Maproom MCP is ready!');
        resolve();
      } else {
        reject(new Error(`docker compose exited with code ${code}`));
      }
    });
  });
}

// Wait for services to be healthy
async function waitForHealth() {
  const maxRetries = 30;
  let retries = 0;

  while (retries < maxRetries) {
    const check = spawn('docker', [
      'compose',
      '-f', COMPOSE_FILE,
      'ps', '--services', '--filter', 'status=running'
    ], { cwd: CONFIG_DIR });

    const output = await new Promise((resolve) => {
      let stdout = '';
      check.stdout.on('data', (data) => stdout += data);
      check.on('close', () => resolve(stdout));
    });

    const running = output.trim().split('\n').length;
    if (running >= 3) {  // All 3 services running
      console.log('✅ All services healthy');
      return true;
    }

    retries++;
    await new Promise(resolve => setTimeout(resolve, 2000));
  }

  throw new Error('Services failed to start within timeout');
}

// Main entry point
async function main() {
  try {
    await startStack();
    await waitForHealth();

    // Now connect to MCP server and proxy stdio
    console.log('🔌 Connecting to MCP server...');

    const mcp = spawn('docker', [
      'compose',
      '-f', COMPOSE_FILE,
      'exec', '-T', 'maproom',
      '/usr/local/bin/crewchief-maproom', 'serve', '--stdio'
    ], {
      cwd: CONFIG_DIR,
      stdio: ['inherit', 'inherit', 'inherit']
    });

    mcp.on('close', (code) => {
      process.exit(code);
    });

    process.on('SIGINT', () => {
      console.log('\n🛑 Shutting down gracefully...');
      mcp.kill('SIGTERM');
    });

  } catch (error) {
    console.error('❌ Error:', error.message);
    process.exit(1);
  }
}

main();
```

**Local Development**:
```bash
# Clone the repository
git clone https://github.com/crewchief/maproom-mcp.git
cd maproom-mcp

# Install dependencies
pnpm install

# Test locally with npx (links to local version)
npm link

# Now test in .mcp.json with local version
npx @crewchief/maproom-mcp

# Or test directly
node bin/cli.js

# For development, use absolute path in .mcp.json
{
  "mcpServers": {
    "maproom": {
      "command": "node",
      "args": ["/absolute/path/to/maproom-mcp/bin/cli.js"]
    }
  }
}

# Publish to npm (after testing)
npm version patch
npm publish --access public
```

**User Experience Flow**:
1. User adds npx command to `.mcp.json`
2. npx downloads package to cache (~500KB)
3. CLI wrapper runs `docker-compose up -d`
4. Docker pulls images (first time: ~2-3 minutes for Ollama + PostgreSQL + Maproom)
5. Wrapper waits for health checks
6. Wrapper proxies stdio to MCP container
7. Subsequent runs: ~10-20 seconds (Docker caches images)

**Migration from Legacy `maproom-mcp`**:

The existing `maproom-mcp` package (manual PostgreSQL + API keys) will be deprecated in favor of this containerized version. Migration strategy:

1. **Final `maproom-mcp` Update** (legacy package):
```javascript
#!/usr/bin/env node
console.warn(`
⚠️  DEPRECATION NOTICE: maproom-mcp has moved to @crewchief/maproom-mcp

The new version includes:
  • 🐳 Fully containerized with Docker
  • 🚀 Local LLM embeddings (no API keys required)
  • 📦 Bundled PostgreSQL
  • 🔌 Zero-configuration setup

Please update your .mcp.json:
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}

This compatibility wrapper will forward to the new package.
The legacy manual-setup version will be removed in 6 months.
`);

// Forward to new package
const { spawn } = require('child_process');
spawn('npx', ['-y', '@crewchief/maproom-mcp', ...process.argv.slice(2)], {
  stdio: 'inherit'
}).on('exit', process.exit);
```

2. **npm Deprecation**:
```bash
npm deprecate maproom-mcp "Package moved to @crewchief/maproom-mcp. Please update your config."
```

3. **Timeline**:
   - **Month 1-3**: Both packages work, warnings in legacy
   - **Month 4-6**: Announce removal date
   - **Month 6+**: Remove legacy or make it a permanent redirect

4. **Release Process**:
```bash
# Legacy package - one final update
cd packages/legacy-maproom-mcp
npm version patch
npm publish
npm deprecate maproom-mcp "Moved to @crewchief/maproom-mcp. See migration guide."

# New package - active development
cd packages/maproom-mcp
npm version patch
npm publish --access public

# Verify Docker Compose v2 is installed
docker compose version
```

#### 1. Maproom MCP Service

**Base Image**: `rust:1.75-slim` (multi-stage build)

**Responsibilities**:
- MCP server API (stdio + HTTP/SSE transport)
- Code indexing and parsing (tree-sitter)
- Search query handling (hybrid FTS + vector)
- Embedding request orchestration
- Health monitoring and metrics

**Configuration**:
```toml
# maproom.toml (embedded in container)
[embedding]
provider = "ollama"
model = "nomic-embed-text"
dimension = 768
api_endpoint = "http://ollama:11434"
batch_size = 50
cache_max_entries = 10000
cache_ttl_seconds = 3600

[database]
url = "postgresql://maproom:maproom@postgres:5432/maproom"
pool_size = 10
connection_timeout = 30

[search]
default_mode = "hybrid"
fts_weight = 0.3
vector_weight = 0.7
enable_graph_signals = true
```

**Dockerfile** (Multi-stage):
```dockerfile
# Stage 1: Build
FROM rust:1.75-slim AS builder
WORKDIR /build
COPY crates/maproom ./
RUN cargo build --release --bin crewchief-maproom

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/crewchief-maproom /usr/local/bin/
COPY config/maproom.toml /etc/maproom/maproom.toml

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s \
  CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/usr/local/bin/crewchief-maproom"]
CMD ["serve", "--config", "/etc/maproom/maproom.toml"]
```

#### 2. Ollama Service

**Base Image**: `ollama/ollama:latest`

**Responsibilities**:
- Host nomic-embed-text model
- Provide OpenAI-compatible embedding API
- Handle model lifecycle (load, cache, unload)
- GPU acceleration (if available)

**Init Container Pattern**:
```bash
#!/bin/sh
# init-ollama.sh - Runs before main Ollama service

echo "Starting Ollama server..."
ollama serve &
OLLAMA_PID=$!

echo "Waiting for Ollama to be ready..."
while ! curl -s http://localhost:11434/api/tags > /dev/null; do
    sleep 1
done

echo "Pulling nomic-embed-text model..."
ollama pull nomic-embed-text

echo "Model ready. Starting main service..."
wait $OLLAMA_PID
```

**GPU Support** (Optional):
```yaml
deploy:
  resources:
    reservations:
      devices:
        - driver: nvidia
          count: 1
          capabilities: [gpu]
```

#### 3. PostgreSQL Service

**Base Image**: `pgvector/pgvector:pg16`

**Responsibilities**:
- Store code chunks and metadata
- Vector similarity search (pgvector)
- Full-text search (tsvector + GIN index)
- Relationship graph (chunk_edges table)

**Schema Initialization**:
```sql
-- init.sql (auto-loaded on first startup)

CREATE EXTENSION IF NOT EXISTS vector;

CREATE SCHEMA IF NOT EXISTS maproom;

CREATE TABLE maproom.repositories (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE maproom.worktrees (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER REFERENCES maproom.repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(repo_id, name)
);

CREATE TABLE maproom.files (
    id SERIAL PRIMARY KEY,
    worktree_id INTEGER REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
    relpath TEXT NOT NULL,
    file_type TEXT NOT NULL,
    size_bytes BIGINT,
    last_modified TIMESTAMPTZ,
    git_hash TEXT,
    UNIQUE(worktree_id, relpath)
);

CREATE TABLE maproom.chunks (
    id BIGSERIAL PRIMARY KEY,
    file_id INTEGER REFERENCES maproom.files(id) ON DELETE CASCADE,
    symbol_name TEXT,
    kind TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    signature TEXT,
    docstring TEXT,
    preview TEXT NOT NULL,

    -- Embeddings (768 dimensions for nomic-embed-text)
    code_embedding vector(768),
    text_embedding vector(768),

    -- Full-text search
    fts_tokens tsvector,

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for hybrid search
CREATE INDEX idx_chunks_code_embedding ON maproom.chunks
    USING ivfflat (code_embedding vector_cosine_ops)
    WITH (lists = 100);

CREATE INDEX idx_chunks_text_embedding ON maproom.chunks
    USING ivfflat (text_embedding vector_cosine_ops)
    WITH (lists = 100);

CREATE INDEX idx_chunks_fts ON maproom.chunks USING GIN (fts_tokens);

CREATE INDEX idx_chunks_file_id ON maproom.chunks(file_id);
CREATE INDEX idx_chunks_kind ON maproom.chunks(kind);

-- Chunk relationships (imports, calls, etc.)
CREATE TABLE maproom.chunk_edges (
    id BIGSERIAL PRIMARY KEY,
    from_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
    to_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
    edge_type TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(from_chunk_id, to_chunk_id, edge_type)
);

CREATE INDEX idx_edges_from ON maproom.chunk_edges(from_chunk_id);
CREATE INDEX idx_edges_to ON maproom.chunk_edges(to_chunk_id);
CREATE INDEX idx_edges_type ON maproom.chunk_edges(edge_type);

-- Statistics table for monitoring
CREATE TABLE maproom.stats (
    id SERIAL PRIMARY KEY,
    metric_name TEXT NOT NULL,
    metric_value NUMERIC NOT NULL,
    recorded_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_stats_name_time ON maproom.stats(metric_name, recorded_at DESC);
```

**Configuration**:
```conf
# postgresql.conf overrides
max_connections = 100
shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1
effective_io_concurrency = 200
work_mem = 4MB
min_wal_size = 1GB
max_wal_size = 4GB
```

## Docker Compose Configuration

### Complete docker-compose.yml

```yaml
version: '3.8'

services:
  postgres:
    image: pgvector/pgvector:pg16
    container_name: maproom-postgres
    environment:
      POSTGRES_DB: maproom
      POSTGRES_USER: maproom
      POSTGRES_PASSWORD: maproom
      PGDATA: /var/lib/postgresql/data/pgdata
    volumes:
      - maproom-data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql:ro
      - ./postgresql.conf:/etc/postgresql/postgresql.conf:ro
    networks:
      - maproom-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U maproom -d maproom"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    restart: unless-stopped

  ollama:
    image: ollama/ollama:latest
    container_name: maproom-ollama
    volumes:
      - ollama-models:/root/.ollama
      - ./init-ollama.sh:/usr/local/bin/init-ollama.sh:ro
    ports:
      - "${OLLAMA_PORT:-11434}:11434"
    networks:
      - maproom-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:11434/api/tags"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 120s  # Allow time for model download
    restart: unless-stopped
    # Optional GPU support
    # deploy:
    #   resources:
    #     reservations:
    #       devices:
    #         - driver: nvidia
    #           count: 1
    #           capabilities: [gpu]
    command: >
      sh -c "
        ollama serve &
        OLLAMA_PID=$$!
        echo 'Waiting for Ollama server...'
        until curl -s http://localhost:11434/api/tags > /dev/null; do sleep 1; done
        echo 'Pulling nomic-embed-text model...'
        ollama pull nomic-embed-text
        echo 'Model ready!'
        wait $$OLLAMA_PID
      "

  maproom:
    build:
      context: .
      dockerfile: Dockerfile.maproom
    container_name: maproom-mcp
    depends_on:
      postgres:
        condition: service_healthy
      ollama:
        condition: service_healthy
    environment:
      DATABASE_URL: postgresql://maproom:maproom@postgres:5432/maproom
      EMBEDDING_PROVIDER: ollama
      EMBEDDING_MODEL: nomic-embed-text
      EMBEDDING_DIMENSION: 768
      EMBEDDING_API_ENDPOINT: http://ollama:11434
      RUST_LOG: ${RUST_LOG:-info}
    volumes:
      - maproom-config:/config
      - ${HOST_WORKSPACE:-/workspace}:/workspace:ro
    ports:
      - "${MAPROOM_PORT:-3000}:3000"
    networks:
      - maproom-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s
    restart: unless-stopped

volumes:
  maproom-data:
    driver: local
  ollama-models:
    driver: local
  maproom-config:
    driver: local

networks:
  maproom-network:
    driver: bridge
```

### Simplified User Script

**run-maproom-local.sh** (for local development/testing):
```bash
#!/bin/bash
set -e

echo "🚀 Starting Maproom Local with embedded LLM..."

# Check Docker availability
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker Desktop."
    exit 1
fi

# Check Docker Compose v2 plugin
if ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose v2 plugin not found."
    echo "   Please install Docker Desktop (includes Compose v2)"
    echo "   or install Compose v2 plugin: https://docs.docker.com/compose/install/"
    exit 1
fi

# Create volumes if they don't exist
docker volume create maproom-data || true
docker volume create ollama-models || true
docker volume create maproom-config || true

# Start the stack
docker compose up -d

echo ""
echo "✅ Maproom Local is starting!"
echo ""
echo "📊 Services:"
echo "  • Maproom MCP: http://localhost:3000"
echo "  • Ollama API:  http://localhost:11434"
echo "  • PostgreSQL:  localhost:5432 (internal only)"
echo ""
echo "📦 First startup will download the embedding model (~200MB)."
echo "   This may take 1-2 minutes. Check progress with:"
echo "   docker compose logs -f ollama"
echo ""
echo "🔍 Check status:"
echo "   docker compose ps"
echo ""
echo "🛑 Stop services:"
echo "   docker compose down"
echo ""
```

## Code Changes Required

### 1. Rust Embedding Client Modifications

**File**: `crates/maproom/src/embedding/client.rs`

**Changes**: Add Ollama provider support

```rust
// Add to client.rs

use crate::embedding::config::{EmbeddingConfig, Provider};

impl OpenAIClient {
    /// Create client from config (supports OpenAI and Ollama)
    pub fn new(config: EmbeddingConfig) -> Result<Self, EmbeddingError> {
        config.validate()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(EmbeddingError::Network)?;

        Ok(Self {
            client,
            config: Arc::new(config),
            metrics: Arc::new(CostMetrics::default()),
        })
    }

    /// Build API request for OpenAI or Ollama
    async fn try_embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>, EmbeddingError> {
        let api_key = self.config.api_key.as_ref();

        // Build request based on provider
        let request = match self.config.provider {
            Provider::OpenAI => {
                let key = api_key.ok_or_else(|| {
                    EmbeddingError::Config(ConfigError::MissingConfig("API key".to_string()))
                })?;

                self.client
                    .post(&self.config.api_endpoint_url())
                    .header("Authorization", format!("Bearer {}", key))
                    .header("Content-Type", "application/json")
            },
            Provider::Ollama => {
                // Ollama doesn't require API key
                self.client
                    .post(&self.config.api_endpoint_url())
                    .header("Content-Type", "application/json")
            },
            Provider::Local => {
                // Custom local server
                self.client
                    .post(&self.config.api_endpoint_url())
                    .header("Content-Type", "application/json")
            },
        };

        // Ollama uses slightly different request format
        let body = if self.config.provider == Provider::Ollama {
            serde_json::json!({
                "model": self.config.model,
                "prompt": texts,
            })
        } else {
            serde_json::json!({
                "input": texts,
                "model": self.config.model,
                "dimensions": self.config.dimension,
            })
        };

        let response = request.json(&body).send().await?;

        // Handle response...
        // (rest of existing code)
    }
}
```

### 2. Configuration Updates

**File**: `crates/maproom/src/embedding/config.rs`

**Changes**: Add Ollama-specific configuration

```rust
impl EmbeddingConfig {
    /// Get API endpoint for provider
    pub fn api_endpoint_url(&self) -> String {
        if let Some(endpoint) = &self.api_endpoint {
            endpoint.clone()
        } else {
            match self.provider {
                Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
                Provider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
                Provider::Ollama => "http://localhost:11434/api/embeddings".to_string(),
                Provider::Local => "http://localhost:8080/embeddings".to_string(),
            }
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // API key only required for cloud providers
        if matches!(self.config.provider, Provider::OpenAI | Provider::Cohere)
            && self.api_key.is_none() {
            return Err(ConfigError::MissingConfig(
                format!("API key for {:?} provider", self.provider)
            ));
        }

        // Ollama and Local don't need API keys

        // Validate dimension matches model
        match (self.provider, self.model.as_str()) {
            (Provider::Ollama, "nomic-embed-text") if self.dimension != 768 => {
                return Err(ConfigError::InvalidValue {
                    field: "dimension".to_string(),
                    reason: "nomic-embed-text requires dimension=768".to_string(),
                });
            },
            _ => {}
        }

        // Rest of validation...
        Ok(())
    }
}
```

### 3. Provider Enum Extension

**File**: `crates/maproom/src/embedding/config.rs`

**Changes**: Add Ollama variant

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// OpenAI embedding API
    OpenAI,
    /// Cohere embedding API
    Cohere,
    /// Ollama local server
    Ollama,  // NEW
    /// Local embedding model
    Local,
}

impl std::str::FromStr for Provider {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Self::OpenAI),
            "cohere" => Ok(Self::Cohere),
            "ollama" => Ok(Self::Ollama),  // NEW
            "local" => Ok(Self::Local),
            _ => Err(ConfigError::InvalidValue {
                field: "provider".to_string(),
                reason: format!("Unknown provider: {}", s),
            }),
        }
    }
}
```

## Testing Strategy

### Integration Tests

**File**: `tests/integration/local_embedding_test.rs`

```rust
#[tokio::test]
async fn test_ollama_embedding_generation() {
    let config = EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nomic-embed-text".to_string(),
        dimension: 768,
        api_endpoint: Some("http://localhost:11434".to_string()),
        api_key: None,
        // ... other config
    };

    let service = EmbeddingService::new(config).unwrap();

    let text = "function calculateTotal(items) { return items.reduce((sum, item) => sum + item.price, 0); }";
    let embedding = service.embed_text(text).await.unwrap();

    assert_eq!(embedding.len(), 768);
    assert!(embedding.iter().any(|&x| x != 0.0));
}

#[tokio::test]
async fn test_batch_embedding_with_ollama() {
    let service = create_ollama_service();

    let texts = vec![
        "export function greet(name: string): string".to_string(),
        "class UserService { constructor() {} }".to_string(),
        "const API_ENDPOINT = 'https://api.example.com'".to_string(),
    ];

    let embeddings = service.embed_batch(texts).await.unwrap();

    assert_eq!(embeddings.len(), 3);
    for emb in embeddings {
        assert_eq!(emb.len(), 768);
    }
}
```

### Docker Health Checks

```bash
#!/bin/bash
# health-check.sh

# Check PostgreSQL
pg_isready -U maproom -d maproom || exit 1

# Check Ollama
curl -f http://ollama:11434/api/tags || exit 1

# Check Maproom MCP
curl -f http://localhost:3000/health || exit 1

echo "All services healthy ✅"
```

## Docker Image Distribution Strategy

### Overview

The Maproom Docker image can be distributed in two ways:

1. **Build from Source** (Phase 1 - MVP)
2. **Pre-built Images** (Phase 2 - Production)

### Phase 1: Build from Source (MVP)

**Implementation**: npm package includes Dockerfile, users build locally on first run

**docker-compose.yml**:
```yaml
services:
  maproom:
    build:
      context: .
      dockerfile: Dockerfile.maproom
    container_name: maproom-mcp
```

**npm package includes**:
```
@crewchief/maproom-mcp/
├── config/
│   ├── docker-compose.yml
│   ├── Dockerfile.maproom        # Multi-stage Rust build
│   ├── init.sql
│   └── postgresql.conf
└── bin/cli.js
```

**User Experience**:
- First run: ~5-10 minutes (builds Rust binary from source)
- Subsequent runs: ~10-20 seconds (Docker cache)

**Pros**:
- ✅ No Docker Hub setup required
- ✅ Users always get latest code
- ✅ Simpler release process

**Cons**:
- ❌ Slow first-time setup
- ❌ Requires build tools (Rust, cargo) in Docker image

---

### Phase 2: Pre-built Images (Production-Ready)

**Implementation**: Publish images to Docker Hub, users pull pre-built binaries

#### Step 1: One-Time Human Setup (HUMAN ACTION REQUIRED)

**Required Human Actions** (cannot be automated):

1. **Create Docker Hub Account** (5 minutes)
   - Visit https://hub.docker.com
   - Sign up (requires email verification)
   - Accept Terms of Service

2. **Generate Docker Hub Access Token** (2 minutes)
   - Go to Account Settings → Security → Access Tokens
   - Click "New Access Token"
   - Name: `github-actions-maproom`
   - Permissions: Read, Write, Delete
   - **Copy and save the token** - you'll add it to GitHub next

3. **Add GitHub Secrets** (2 minutes)
   - Go to your GitHub repository → Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Add two secrets:
     - Name: `DOCKERHUB_USERNAME`, Value: Your Docker Hub username
     - Name: `DOCKERHUB_TOKEN`, Value: The token from step 2

**Total human time**: ~10 minutes (one-time setup)

---

#### Step 2: Automated Setup (AGENT CAN DO THIS)

**GitHub Actions Workflow**:

Create `.github/workflows/docker-publish.yml`:

```yaml
name: Build and Publish Docker Images

on:
  push:
    tags:
      - 'v*.*.*'  # Trigger on version tags like v1.0.0
  workflow_dispatch:  # Allow manual trigger

jobs:
  build-and-push:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to Docker Hub
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKERHUB_USERNAME }}
          password: ${{ secrets.DOCKERHUB_TOKEN }}

      - name: Extract version from tag
        id: version
        run: |
          if [[ $GITHUB_REF == refs/tags/* ]]; then
            VERSION=${GITHUB_REF#refs/tags/v}
          else
            VERSION=dev-${GITHUB_SHA::8}
          fi
          echo "VERSION=$VERSION" >> $GITHUB_OUTPUT
          echo "Building version: $VERSION"

      - name: Build and push Maproom image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: Dockerfile.maproom
          platforms: linux/amd64,linux/arm64
          push: true
          tags: |
            crewchief/maproom-mcp:${{ steps.version.outputs.VERSION }}
            crewchief/maproom-mcp:latest
          cache-from: type=registry,ref=crewchief/maproom-mcp:buildcache
          cache-to: type=registry,ref=crewchief/maproom-mcp:buildcache,mode=max

      - name: Update Docker Hub description
        uses: peter-evans/dockerhub-description@v3
        with:
          username: ${{ secrets.DOCKERHUB_USERNAME }}
          password: ${{ secrets.DOCKERHUB_TOKEN }}
          repository: crewchief/maproom-mcp
          short-description: "Maproom MCP server with local LLM embeddings (Ollama + nomic-embed-text)"
```

**What this does**:
- Builds Docker images for both AMD64 and ARM64 (Apple Silicon)
- Pushes to Docker Hub with version tag (e.g., `1.0.0`) and `latest`
- Uses build cache to speed up subsequent builds
- Updates Docker Hub repository description automatically

**Agent can create this file**: An agent can write this workflow file to `.github/workflows/docker-publish.yml` automatically.

---

#### Step 3: Update docker-compose.yml (AGENT CAN DO THIS)

**Task for agent**: Update the docker-compose.yml to support both pre-built images and local builds.

**Change from**:
```yaml
services:
  maproom:
    build:
      context: .
      dockerfile: Dockerfile.maproom
```

**Change to**:
```yaml
services:
  maproom:
    image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}
    # Fallback to build if image not available (for development)
    build:
      context: .
      dockerfile: Dockerfile.maproom
```

**Environment variable override**:
```bash
# Use specific version
MAPROOM_VERSION=1.0.0 npx @crewchief/maproom-mcp

# Use latest (default)
npx @crewchief/maproom-mcp

# Force local build (development)
MAPROOM_IMAGE="" docker compose up -d
```

**Agent can**: Modify docker-compose.yml, test locally, commit changes.

---

#### Step 4: Create Docker Hub Repository (AGENT CAN DO THIS VIA CLI)

**Agent can automate** using Docker Hub API or `docker` CLI:

```bash
# Login with token (agent can use DOCKERHUB_TOKEN env var)
echo "$DOCKERHUB_TOKEN" | docker login -u "$DOCKERHUB_USERNAME" --password-stdin

# Create repository via Docker Hub API
curl -X POST \
  -H "Authorization: JWT $DOCKERHUB_JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "crewchief",
    "name": "maproom-mcp",
    "description": "Maproom MCP server with local LLM embeddings",
    "is_private": false
  }' \
  https://hub.docker.com/v2/repositories/

# Or just push an image - Docker Hub auto-creates public repos on first push
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t crewchief/maproom-mcp:latest \
  --push \
  -f Dockerfile.maproom \
  .
```

**Agent can**: Create repository, set description, configure settings.

---

#### Step 5: Release Process (MOSTLY AUTOMATED)

**Agent can handle entire release** except the final `git push`:

```bash
# Agent executes:

# 1. Update npm package version
cd packages/maproom-mcp
npm version patch  # or minor, or major

# 2. Create git tag locally
VERSION=$(node -p "require('./package.json').version")
git tag "v${VERSION}"

# 3. Commit and prepare for push
git add .
git commit -m "Release v${VERSION}"

# 4. HUMAN ACTION: Review and push
# Human reviews the changes and runs:
git push origin main
git push origin "v${VERSION}"

# 5. GitHub Actions automatically (triggered by tag push):
#    - Builds multi-platform images (5-10 minutes)
#    - Pushes to Docker Hub
#    - Tags as both v1.0.1 and latest

# 6. Agent can publish npm package after CI passes
npm publish --access public
```

**What requires human action**:
- ✅ **One-time setup**: Docker Hub account + GitHub secrets (~10 min)
- ✅ **Per release**: Review changes and `git push` (30 seconds)

**What agent automates**:
- ✅ Version bumping
- ✅ Creating git tags
- ✅ Creating GitHub Actions workflow
- ✅ Updating docker-compose.yml
- ✅ Creating Docker Hub repository
- ✅ Publishing to npm
- ✅ Running tests
- ✅ Generating changelogs

**Automated Flow**:
```
1. Agent: npm version patch, git tag v1.0.1
2. Human: Review and git push
3. GitHub Actions: Build Docker images (5-10 minutes, automatic)
4. GitHub Actions: Push to Docker Hub (automatic)
5. Agent: npm publish --access public (after CI passes)
6. Users: npx pulls new package, Docker pulls new image
```

---

#### Step 6: Monitoring Builds (HUMAN OR AGENT)

**Agent can monitor and report**:

```bash
# Check GitHub Actions status
gh run list --workflow docker-publish.yml --limit 5

# View specific run logs
gh run view <run-id> --log

# Check Docker Hub tags via API
curl -s https://hub.docker.com/v2/repositories/crewchief/maproom-mcp/tags \
  | jq '.results[] | {name, last_updated, full_size}'

# Wait for CI to pass before publishing npm
gh run watch <run-id>
```

**Agent can**:
- Poll GitHub Actions for build completion
- Retry failed builds
- Notify on completion
- Auto-publish npm after Docker images are ready
- Generate release notes from commits

---

### Hybrid Approach (Recommended)

**Support both build and pull** in docker-compose.yml:

```yaml
services:
  maproom:
    # Try to use pre-built image
    image: ${MAPROOM_IMAGE:-crewchief/maproom-mcp:latest}

    # Fallback to local build if image not found or MAPROOM_IMAGE=""
    build:
      context: .
      dockerfile: Dockerfile.maproom
```

**Benefits**:
- Default: Fast downloads from Docker Hub
- Development: Can force local builds with `MAPROOM_IMAGE=""`
- Offline: Docker caches pulled images
- Flexibility: Users can choose their workflow

---

### Build Times Comparison

| Scenario | Build from Source | Pre-built Image |
|----------|------------------|-----------------|
| **First run** | 5-10 minutes | 2-3 minutes |
| **Subsequent runs** | 10-20 seconds | 10-20 seconds |
| **Internet required** | Yes (dependencies) | Yes (image pull) |
| **Disk usage** | ~2.5GB | ~2GB |
| **Build tools needed** | In Docker image | None |

---

### Multi-Platform Support

**Platforms to support**:
- `linux/amd64` - Intel/AMD processors (most cloud, Windows/WSL, older Macs)
- `linux/arm64` - Apple Silicon (M1/M2/M3/M4 Macs)

**Why multi-platform matters**:
- Apple Silicon Macs are increasingly common among developers
- ARM64 images run natively (no emulation overhead)
- Single `docker compose up` works on all platforms

**How GitHub Actions handles this**:
```yaml
platforms: linux/amd64,linux/arm64
```
- Builds both architectures in parallel
- Pushes multi-platform manifest to Docker Hub
- Docker automatically pulls correct architecture for user's machine

---

### Human vs Agent Responsibilities Summary

#### ONE-TIME SETUP (Human: ~10 minutes)

**Human must do**:
1. Create Docker Hub account (requires email verification)
2. Generate Docker Hub access token
3. Add secrets to GitHub repository (`DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`)

**Agent can do everything else**:
- Create `.github/workflows/docker-publish.yml`
- Create Docker Hub repository via API
- Update docker-compose.yml
- Configure npm package
- Write documentation

#### PER-RELEASE (Human: ~30 seconds)

**Agent prepares release**:
1. Run tests
2. Bump version (`npm version patch`)
3. Create git tag
4. Generate changelog
5. Commit changes

**Human reviews and approves**:
1. Review changes: `git log`
2. Push to trigger CI: `git push origin main && git push origin v1.0.1`

**Agent completes release automatically**:
1. Monitor GitHub Actions build
2. Wait for Docker images to be published
3. Publish npm package: `npm publish --access public`
4. Post release notes

**Total human time per release**: 30 seconds

---

### Migration Path

**Week 1-2 (MVP)**: Build from source
- Agent ships npm package with Dockerfile
- Users build locally
- Validate functionality
- **Human action**: None (agents handle everything)

**Week 3-4 (Production)**: Pre-built images
- **Human action**: One-time Docker Hub + GitHub setup (~10 min)
- Agent creates GitHub Actions workflow
- Agent creates Docker Hub repository
- Agent updates npm package to use `image:`
- Test automated builds

**Post-Launch**: Continuous delivery
- **Human action per release**: Review and `git push` (~30 seconds)
- Agent handles: version bump, tag creation, changelog, npm publish
- GitHub Actions: Builds Docker images automatically
- Users get fast downloads, multi-platform support

---

### Cost Analysis

**Docker Hub Free Tier**:
- ✅ Unlimited public repositories
- ✅ Unlimited pulls
- ✅ 1 private repository
- ✅ No cost for public images

**GitHub Actions Free Tier**:
- ✅ 2,000 minutes/month for public repos
- Build time per release: ~10-15 minutes
- Can release ~130 times/month for free

**Total Cost**: $0/month for public open-source project

---

### Security Considerations

1. **Docker Hub Token**:
   - ✅ Use GitHub Secrets (encrypted)
   - ✅ Limit permissions to Read/Write (not Admin)
   - ✅ Rotate tokens annually

2. **Image Scanning**:
   ```yaml
   - name: Scan image for vulnerabilities
     uses: aquasecurity/trivy-action@master
     with:
       image-ref: crewchief/maproom-mcp:latest
       format: 'sarif'
       output: 'trivy-results.sarif'
   ```

3. **Signed Images** (Advanced):
   ```bash
   # Enable Docker Content Trust
   export DOCKER_CONTENT_TRUST=1
   docker push crewchief/maproom-mcp:latest
   ```

## Deployment Variants

### Variant 1: Development (Default)
- CPU-only
- Moderate resource limits
- Hot-reload support
- Debug logging enabled

### Variant 2: Production
- Optional GPU acceleration
- Stricter resource limits
- Health monitoring
- Structured logging

### Variant 3: Lightweight
- Smaller embedding model (all-MiniLM-L6-v2)
- Reduced batch sizes
- Lower memory limits
- For resource-constrained environments

## Security Considerations

1. **Network Isolation**: Internal services not exposed to host
2. **Volume Permissions**: Proper file permissions on volumes
3. **Secret Management**: No hardcoded credentials
4. **Resource Limits**: Prevent resource exhaustion
5. **Health Monitoring**: Automatic restart on failure

## Monitoring and Observability

### Metrics to Track
- Embedding generation rate (chunks/sec)
- API latency (p50, p95, p99)
- Cache hit rate
- Resource usage (CPU, RAM, disk)
- Error rates by type

### Logging Structure
```json
{
  "timestamp": "2025-10-26T10:30:00Z",
  "level": "INFO",
  "service": "maproom-mcp",
  "message": "Generated embeddings for 50 chunks",
  "duration_ms": 1234,
  "batch_size": 50,
  "cache_hits": 10,
  "api_calls": 40
}
```

## Next Steps

See LOCAL_PLAN.md for implementation phases and agent assignments.
