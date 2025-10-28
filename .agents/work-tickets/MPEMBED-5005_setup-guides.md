# Ticket: MPEMBED-5005: Setup guides for all providers

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- verify-ticket
- commit-ticket

## Summary
Create setup guides for Ollama and OpenAI providers (Google guide already exists from MPEMBED-3004). Include installation, configuration, and troubleshooting sections.

## Background
This ticket completes the provider documentation suite by adding setup guides for Ollama and OpenAI. These guides should be clear enough for users with no prior experience to successfully configure each provider.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-5-mcp-documentation.md

## Acceptance Criteria
- [ ] Ollama setup guide with install instructions (macOS, Linux, Windows)
- [ ] Ollama model download and verification steps
- [ ] OpenAI setup guide with API key generation
- [ ] Environment variable configuration for both providers
- [ ] Troubleshooting sections for common issues
- [ ] Quick start examples for each provider
- [ ] Screenshots or code examples where helpful

## Technical Requirements
- Document format: Markdown
- Platform-specific instructions (macOS, Linux, Windows)
- Include verification steps to confirm setup
- Link to official provider documentation
- Test instructions on clean environments
- Follow existing docs/ structure

## Implementation Notes
**Ollama Setup Guide:**
```markdown
# Ollama Setup Guide

## Overview
Ollama provides free, local embedding generation with zero configuration. Ideal for development and offline use.

## Prerequisites
- macOS 11+ / Linux / Windows 10+ (WSL2)
- 8GB RAM minimum (16GB recommended)
- Optional: NVIDIA GPU for faster generation

## Installation

### macOS
```bash
curl -sSL https://ollama.ai/install.sh | sh
```

Or with Homebrew:
```bash
brew install ollama
```

### Linux
```bash
curl -sSL https://ollama.ai/install.sh | sh
```

### Windows
1. Install WSL2
2. Follow Linux instructions in WSL2

## Pull Embedding Model
```bash
ollama pull nomic-embed-text
```

Expected output:
```
pulling manifest
pulling 970aa74c0a90... 100%
success
```

## Verify Installation
```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Should return list of models including nomic-embed-text
```

## Configure CrewChief
No configuration needed! Ollama is auto-detected.

Optional explicit configuration:
```bash
export EMBEDDING_PROVIDER=ollama
```

## Test Embedding Generation
```bash
crewchief maproom scan --generate-embeddings
```

Expected output:
```
✓ Using embedding provider: ollama (768 dimensions)
✓ Scanning repository...
✓ Generated embeddings for 1,234 chunks
```

## Troubleshooting

### "Connection refused to localhost:11434"
- **Cause**: Ollama service not running
- **Fix**: Start Ollama: `ollama serve`

### "Model nomic-embed-text not found"
- **Cause**: Model not downloaded
- **Fix**: `ollama pull nomic-embed-text`

### Slow embedding generation
- **Cause**: Running on CPU
- **Fix**: Install NVIDIA drivers and CUDA toolkit for GPU acceleration

## Advanced Configuration

### GPU Acceleration
Ollama automatically uses GPU if available:
```bash
# Check GPU usage
nvidia-smi
```

### Resource Limits
```bash
# Limit CPU cores
OLLAMA_NUM_THREAD=4 ollama serve

# Limit GPU memory
OLLAMA_MAX_VRAM=4096 ollama serve
```
```

**OpenAI Setup Guide:**
```markdown
# OpenAI Setup Guide

## Overview
OpenAI provides reliable, high-quality embeddings via API. Simple setup with pay-as-you-go pricing.

## Prerequisites
- OpenAI account (https://platform.openai.com)
- Credit card for billing

## Get API Key

1. Go to https://platform.openai.com/api-keys
2. Click "+ Create new secret key"
3. Name: "CrewChief Maproom"
4. Permissions: Read (embedding generation doesn't need write)
5. Click "Create secret key"
6. **Copy and save immediately** (won't be shown again)

## Configure Environment
```bash
# Add to ~/.bashrc or ~/.zshrc
export OPENAI_API_KEY="sk-proj-..."
export EMBEDDING_PROVIDER=openai
```

## Verify Configuration
```bash
# Check API key is set
echo $OPENAI_API_KEY

# Test API access
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

## Test Embedding Generation
```bash
crewchief maproom scan --generate-embeddings
```

Expected output:
```
✓ Using embedding provider: openai (1536 dimensions)
✓ Scanning repository...
✓ Generated embeddings for 1,234 chunks
```

## Cost Estimation
- **Pricing**: ~$0.00002 per 1K tokens
- **Example**: 10K chunks (avg 500 chars) = ~$1
- **Monitor usage**: https://platform.openai.com/usage

## Troubleshooting

### "Invalid API key"
- **Cause**: Wrong or expired API key
- **Fix**: Generate new key at https://platform.openai.com/api-keys

### "Rate limit exceeded"
- **Cause**: Too many requests per minute
- **Fix**: Wait 60 seconds or upgrade to higher tier

### "Insufficient credits"
- **Cause**: Billing issue
- **Fix**: Add payment method at https://platform.openai.com/account/billing

## Security Best Practices

### API Key Rotation
Rotate keys every 90 days:
1. Generate new key
2. Update OPENAI_API_KEY
3. Test with scan command
4. Revoke old key

### Restrict Key Permissions
- Use read-only keys when possible
- Don't commit keys to git
- Use environment variables, not config files
```

## Dependencies
- MPEMBED-5004 (Comparison docs for linking)

## Risk Assessment
- **Risk**: Installation instructions may vary by platform
  - **Mitigation**: Test on multiple platforms, link to official docs

## Files/Packages Affected
- docs/providers/ollama-setup.md (create)
- docs/providers/openai-setup.md (create)
- docs/providers/README.md (modify - add links)
