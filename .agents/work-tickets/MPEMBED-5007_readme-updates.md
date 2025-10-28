# Ticket: MPEMBED-5007: Update README with provider options

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- verify-ticket
- commit-ticket

## Summary
Update main README with provider options section, Ollama zero-config quick start, and FAQ about embedding dimensions. Make multi-provider support a prominent feature.

## Background
This ticket updates the main README to highlight the multi-provider embedding support as a key feature. The README should guide new users to the easiest path (Ollama) while documenting alternatives.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-5-mcp-documentation.md

## Acceptance Criteria
- [ ] Provider options section prominently placed
- [ ] Ollama quick start with 3-step setup
- [ ] Links to detailed provider guides
- [ ] FAQ section with dimension questions
- [ ] Updated feature list mentions multi-provider
- [ ] Configuration examples for all providers
- [ ] Migration notice for existing users

## Technical Requirements
- Update README.md in repository root
- Maintain existing structure and tone
- Keep quick start simple (< 5 steps)
- Link to comprehensive docs for details
- Test all code examples
- Update table of contents if present

## Implementation Notes
**README.md updates:**
```markdown
# CrewChief Maproom

Semantic code search powered by embeddings and PostgreSQL.

## Features

- **🔍 Semantic Code Search** - Find code by concept, not just keywords
- **🎯 Multi-Provider Embeddings** - Choose Ollama (free), OpenAI, or Google Vertex AI
- **⚡ Zero-Config Setup** - Auto-detects Ollama, works out of the box
- **🗃️ PostgreSQL Storage** - Reliable vector storage with pgvector
- **🔄 Incremental Indexing** - Fast updates for changed files
- **🌐 MCP Integration** - Works with Claude, Cursor, and other AI tools

## Quick Start (Zero Config)

### 1. Install Ollama (Free, Local)
```bash
curl -sSL https://ollama.ai/install.sh | sh
ollama pull nomic-embed-text
```

### 2. Index Your Repository
```bash
crewchief maproom scan --generate-embeddings
```

### 3. Search Your Code
```bash
crewchief maproom search "authentication middleware"
```

**That's it!** No API keys, no configuration, no costs.

---

## Embedding Providers

Maproom supports three embedding providers:

| Provider | Cost | Setup | Dimensions | Best For |
|----------|------|-------|------------|----------|
| **Ollama** | Free | Easy | 768 | Local dev, privacy, cost |
| **OpenAI** | ~$0.0001/1K | Easy | 1536 | Proven quality |
| **Google Vertex AI** | ~$0.00025/1K | Medium | 768 | Enterprise, compliance |

See [Provider Comparison](docs/providers/comparison.md) for detailed breakdown.

### Ollama (Recommended for Most Users)

**Advantages:**
- ✅ Completely free
- ✅ Works offline
- ✅ Zero configuration
- ✅ Fast (local processing)
- ✅ Complete privacy (data never leaves your machine)

**Setup:** See [Quick Start](#quick-start-zero-config) above

### OpenAI

**Advantages:**
- ✅ Proven embedding quality
- ✅ Simple API setup
- ✅ Reliable cloud service

**Setup:**
```bash
export OPENAI_API_KEY="sk-proj-..."
export EMBEDDING_PROVIDER=openai
crewchief maproom scan --generate-embeddings
```

See [OpenAI Setup Guide](docs/providers/openai-setup.md)

### Google Vertex AI

**Advantages:**
- ✅ Enterprise compliance (HIPAA, SOC2)
- ✅ Regional data residency
- ✅ GCP integration

**Setup:**
```bash
export GOOGLE_PROJECT_ID="your-project"
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/key.json"
export EMBEDDING_PROVIDER=google
crewchief maproom scan --generate-embeddings
```

See [Google Setup Guide](docs/providers/google-vertex-ai-setup.md)

---

## Configuration

Maproom auto-detects your provider:

1. Checks `EMBEDDING_PROVIDER` env var (explicit)
2. Detects Ollama on localhost:11434
3. Falls back to OpenAI if `OPENAI_API_KEY` present
4. Falls back to Google if `GOOGLE_PROJECT_ID` present

### Explicit Provider Selection
```bash
export EMBEDDING_PROVIDER=ollama  # or openai, google
```

### Mixed Embeddings

You can use multiple providers simultaneously! The database stores 768-dim and 1536-dim embeddings in separate columns. Search automatically uses COALESCE to prefer 768-dim embeddings when both exist.

**Migration:** See [Migration Guide](docs/guides/provider-migration.md)

---

## FAQ

### What embedding dimensions does Maproom use?

- **Ollama**: 768 dimensions (nomic-embed-text model)
- **Google**: 768 dimensions (textembedding-gecko model)
- **OpenAI**: 1536 dimensions (text-embedding-3-small model)

### Can I switch providers without re-indexing?

Yes! Existing embeddings are preserved. New embeddings go in separate columns. See [Migration Guide](docs/guides/provider-migration.md).

### Which provider should I use?

- **Start with Ollama** - Free, fast, and private
- **Use OpenAI** - If you're already an OpenAI customer
- **Use Google** - If you need compliance certifications or GCP integration

See [Provider Comparison](docs/providers/comparison.md) for detailed guidance.

### Do I need a GPU for Ollama?

No, but it helps. Ollama works on CPU (slower) or GPU (faster).

### How much does it cost?

- **Ollama**: $0 (free)
- **OpenAI**: ~$5 per 100K chunks
- **Google**: ~$12.50 per 100K chunks

### Can I use this offline?

Yes with Ollama! It runs entirely locally with no internet required.

---

## For Existing Users

**⚠️ Notice for existing Maproom users:**

If you already have OpenAI embeddings:
- Your existing embeddings are **preserved**
- New embeddings use separate columns
- Search works across both embedding types
- No re-indexing required

See [Migration Guide](docs/guides/provider-migration.md) for details.

---

## Documentation

- [Provider Comparison](docs/providers/comparison.md)
- [Ollama Setup](docs/providers/ollama-setup.md)
- [OpenAI Setup](docs/providers/openai-setup.md)
- [Google Vertex AI Setup](docs/providers/google-vertex-ai-setup.md)
- [Migration Guide](docs/guides/provider-migration.md)
- [MCP Integration](docs/mcp/README.md)

---

## Installation

[... existing installation section ...]

## Usage

[... existing usage section ...]
```

**Key Changes:**
1. Add multi-provider to feature list
2. Promote Ollama quick start
3. Add provider comparison table
4. Document all three providers
5. Add migration notice for existing users
6. Expand FAQ with provider questions
7. Link to detailed provider docs

## Dependencies
- MPEMBED-5004 (Comparison docs)
- MPEMBED-5005 (Setup guides)
- MPEMBED-5006 (Migration guide)

## Risk Assessment
- **Risk**: README becomes too long
  - **Mitigation**: Keep main sections brief, link to detailed docs

## Files/Packages Affected
- README.md (modify - add provider documentation)
