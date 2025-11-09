# Ollama Setup Guide

**Last Updated**: October 2025

## Overview

This guide walks you through setting up Ollama for Maproom's semantic code search. Ollama provides free, local embedding generation with zero API costs - perfect for development, testing, and privacy-sensitive codebases.

### What is Ollama?

[Ollama](https://ollama.ai) is an open-source tool that lets you run large language models and embedding models locally on your own hardware. For Maproom, we use Ollama's `nomic-embed-text` model to generate 768-dimensional vector embeddings for semantic code search.

### When to Use Ollama

**Choose Ollama when you need:**
- **Zero API costs**: Completely free after initial setup
- **Complete privacy**: All code stays on your machine, never sent to external APIs
- **Offline capability**: Works without internet connection once models are downloaded
- **Rapid iteration**: No rate limits, instant feedback during development
- **Air-gapped environments**: Deploy in secure/isolated networks

**Consider alternatives if:**
- **Google Vertex AI**: You need production SLA, enterprise compliance, or global scalability
- **OpenAI**: You prefer higher-dimensional embeddings (1536D vs 768D) or existing OpenAI integration
- **Limited hardware**: Your machine doesn't meet minimum requirements (8GB RAM)

### Cost Implications

**Pricing**: **Free** (no API costs, no subscription fees)

**Hardware costs only:**
- **Small codebase** (10K chunks): ~5-10 minutes CPU time
- **Medium codebase** (100K chunks): ~30-60 minutes CPU time
- **Large codebase** (1M chunks): ~5-10 hours CPU time

**Performance optimization:**
- GPU acceleration dramatically speeds up generation (10x-50x faster)
- Incremental indexing only processes changed files
- Embeddings are cached automatically - no re-generation for unchanged code

---

## Prerequisites

Before starting, ensure you have:

- ✅ **macOS 11+** / **Linux** / **Windows 10+ with WSL2**
- ✅ **8GB RAM minimum** (16GB recommended for large codebases)
- ✅ **5GB disk space** (for Ollama + models)
- ✅ **Optional: NVIDIA GPU** (for 10x-50x faster embedding generation)

---

## Step 1: Install Ollama

Ollama provides simple installers for all major platforms.

### macOS

**Option A: Official installer (recommended)**

```bash
curl -fsSL https://ollama.ai/install.sh | sh
```

**Option B: Homebrew**

```bash
brew install ollama
```

**Verify installation:**

```bash
ollama --version
# Expected output: ollama version 0.1.x
```

### Linux

```bash
curl -fsSL https://ollama.ai/install.sh | sh
```

**For specific Linux distributions:**

**Ubuntu/Debian:**
```bash
curl -fsSL https://ollama.ai/install.sh | sh
```

**Fedora/RHEL:**
```bash
curl -fsSL https://ollama.ai/install.sh | sh
```

**Arch Linux:**
```bash
# Via AUR
yay -S ollama
# or
paru -S ollama
```

**Verify installation:**

```bash
ollama --version
systemctl status ollama  # Check service status
```

### Windows

Ollama requires **WSL2 (Windows Subsystem for Linux)** on Windows.

1. **Install WSL2** (if not already installed):
   ```powershell
   # Run in PowerShell as Administrator
   wsl --install
   ```

2. **Restart your computer**

3. **Open WSL2 terminal** (Ubuntu is default)

4. **Install Ollama in WSL2**:
   ```bash
   curl -fsSL https://ollama.ai/install.sh | sh
   ```

5. **Verify installation**:
   ```bash
   ollama --version
   ```

**Note**: All Maproom commands must be run inside the WSL2 terminal, not in PowerShell.

---

## Step 2: Start Ollama Service

Ollama runs as a background service that listens on `http://localhost:11434`.

### macOS

Ollama starts automatically after installation. To manually control:

```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama manually (if needed)
ollama serve
```

### Linux

```bash
# Start Ollama service
sudo systemctl start ollama

# Enable auto-start on boot (recommended)
sudo systemctl enable ollama

# Check service status
systemctl status ollama

# View logs
journalctl -u ollama -f
```

### Windows (WSL2)

```bash
# Start Ollama in background
ollama serve &

# Or start in foreground (helpful for debugging)
ollama serve
```

**Verify Ollama is running:**

```bash
curl http://localhost:11434/api/tags
```

**Expected output**: JSON response with `{"models":[]}` (empty list initially)

---

## Step 3: Download Embedding Model

Ollama supports multiple embedding models. For Maproom, we recommend `nomic-embed-text` for its excellent quality-to-performance ratio.

### Pull the Model

```bash
ollama pull nomic-embed-text
```

**Expected output:**
```
pulling manifest
pulling 970aa74c0a90... 100% ▕████████████████████████████████████████▏ 274 MB
pulling 8ab4849b038c... 100% ▕████████████████████████████████████████▏  11 KB
pulling b838b869f904... 100% ▕████████████████████████████████████████▏  6.9 KB
verifying sha256 digest
writing manifest
success
```

**Download time**: 1-5 minutes depending on internet speed (model is ~274MB)

### Verify Model Installation

```bash
ollama list
```

**Expected output:**
```
NAME                    ID              SIZE      MODIFIED
nomic-embed-text:latest 970aa74c0a90    274 MB    2 minutes ago
```

### Alternative Embedding Models

Ollama supports multiple embedding models with different tradeoffs:

| Model | Dimensions | Size | Speed | Quality |
|-------|-----------|------|-------|---------|
| `nomic-embed-text` | 768 | 274 MB | Fast | High (recommended) |
| `mxbai-embed-large` | 1024 | 669 MB | Medium | Very High |
| `all-minilm` | 384 | 45 MB | Very Fast | Good |

**To use a different model:**
```bash
ollama pull mxbai-embed-large
export OLLAMA_MODEL=mxbai-embed-large
```

---

## Step 4: Configure Maproom

Ollama is **auto-detected** by Maproom - no configuration required!

### Automatic Detection

If Ollama is running on `http://localhost:11434`, Maproom will automatically use it for embeddings.

### Explicit Configuration (Optional)

If you want to explicitly set Ollama as your provider:

```bash
# Add to ~/.bashrc (update the path if your shell uses a different startup file)
export EMBEDDING_PROVIDER=ollama
```

**Apply changes:**
```bash
source ~/.bashrc
```

### Verify Configuration

```bash
# Check environment variable
echo $EMBEDDING_PROVIDER
# Expected output: ollama (or empty if using auto-detection)

# Test Ollama connection
curl http://localhost:11434/api/tags
# Should return JSON with nomic-embed-text model
```

---

## Step 5: Test Embedding Generation

Now test that Maproom can generate embeddings using Ollama.

### Run a Test Scan

```bash
# Scan a small repository to test
crewchief maproom scan --generate-embeddings --dry-run
```

**Expected output:**
```
✓ Using embedding provider: ollama (768 dimensions)
✓ Model: nomic-embed-text
✓ Scanning repository...
✓ Would index 1,234 code chunks
✓ Dry run complete - no changes made
```

### Generate Real Embeddings

```bash
# Index your codebase with embeddings
crewchief maproom scan --generate-embeddings
```

**Expected output:**
```
✓ Using embedding provider: ollama (768 dimensions)
✓ Model: nomic-embed-text
✓ Scanning repository...
✓ Found 1,234 code chunks
✓ Generating embeddings... (this may take a few minutes)
  [████████████████████████████████████████] 1234/1234
✓ Generated embeddings for 1,234 chunks in 3m 42s
✓ Index updated successfully
```

### Search Test

```bash
# Test semantic search
crewchief maproom search "authentication flow"
```

**Expected output:**
```
Found 5 results:

1. auth/middleware.ts:42-68 (score: 0.89)
   Authentication middleware for Express...

2. auth/jwt.service.ts:12-45 (score: 0.85)
   JWT token validation and generation...
```

---

## Troubleshooting

### "Connection refused to localhost:11434"

**Symptoms:**
```
Error: Failed to connect to Ollama at http://localhost:11434
```

**Causes:**
- Ollama service is not running
- Firewall blocking localhost connections

**Solutions:**

1. **Start Ollama service:**
   ```bash
   # macOS
   ollama serve

   # Linux
   sudo systemctl start ollama
   systemctl status ollama

   # Windows (WSL2)
   ollama serve &
   ```

2. **Check if Ollama is listening:**
   ```bash
   curl http://localhost:11434/api/tags
   ```

3. **Check firewall rules:**
   ```bash
   # Linux
   sudo ufw status
   sudo ufw allow 11434
   ```

---

### "Model nomic-embed-text not found"

**Symptoms:**
```
Error: model 'nomic-embed-text' not found
```

**Cause:** Model hasn't been downloaded

**Solution:**
```bash
# Pull the model
ollama pull nomic-embed-text

# Verify it's downloaded
ollama list
```

---

### Slow Embedding Generation

**Symptoms:**
- Embedding generation takes hours for medium-sized codebases
- CPU usage at 100% during embedding generation

**Causes:**
- Running on CPU without GPU acceleration
- Limited CPU cores allocated to Ollama
- Other resource-intensive processes running

**Solutions:**

1. **Enable GPU acceleration** (if you have NVIDIA GPU):
   ```bash
   # Check if GPU is detected
   nvidia-smi

   # Ollama automatically uses GPU if available
   # Verify GPU usage during embedding generation:
   watch -n 1 nvidia-smi
   ```

2. **Allocate more CPU cores:**
   ```bash
   # Set number of threads (default: all cores)
   export OLLAMA_NUM_THREAD=8
   ollama serve
   ```

3. **Close other applications** to free up CPU resources

4. **Use incremental indexing:**
   ```bash
   # Only re-index changed files
   crewchief maproom scan --generate-embeddings --incremental
   ```

**Performance benchmarks:**
- **CPU only**: ~2-5 chunks/second
- **GPU (RTX 3060)**: ~50-100 chunks/second
- **GPU (RTX 4090)**: ~200-300 chunks/second

---

### Ollama Service Won't Start

**Symptoms:**
```
Failed to start ollama.service: Unit ollama.service not found
```

**Causes:**
- Ollama not installed correctly
- Service not registered on Linux

**Solutions:**

1. **Reinstall Ollama:**
   ```bash
   curl -fsSL https://ollama.ai/install.sh | sh
   ```

2. **Manual service registration (Linux):**
   ```bash
   # Check if service file exists
   ls /etc/systemd/system/ollama.service

   # If missing, reinstall Ollama
   curl -fsSL https://ollama.ai/install.sh | sh

   # Reload systemd
   sudo systemctl daemon-reload
   sudo systemctl enable ollama
   sudo systemctl start ollama
   ```

3. **Run Ollama manually:**
   ```bash
   # Start in foreground for debugging
   ollama serve
   ```

---

### "Out of Memory" Errors

**Symptoms:**
```
Error: failed to allocate memory
```

**Causes:**
- Insufficient RAM (less than 8GB)
- Model too large for available memory

**Solutions:**

1. **Use a smaller model:**
   ```bash
   # all-minilm uses only 45MB vs 274MB for nomic-embed-text
   ollama pull all-minilm
   export OLLAMA_MODEL=all-minilm
   ```

2. **Close other applications** to free memory

3. **Increase swap space (Linux):**
   ```bash
   # Check current swap
   free -h

   # Create 4GB swap file
   sudo fallocate -l 4G /swapfile
   sudo chmod 600 /swapfile
   sudo mkswap /swapfile
   sudo swapon /swapfile
   ```

4. **Process in smaller batches:**
   ```bash
   # Index one directory at a time
   crewchief maproom scan src/ --generate-embeddings
   crewchief maproom scan tests/ --generate-embeddings
   ```

---

## Advanced Configuration

### GPU Acceleration

Ollama automatically uses NVIDIA GPUs when available. To verify GPU usage:

```bash
# Check GPU detection
nvidia-smi

# Monitor GPU usage during embedding generation
watch -n 1 nvidia-smi

# Check CUDA version
nvcc --version
```

**GPU memory usage:**
- `nomic-embed-text`: ~500MB VRAM
- `mxbai-embed-large`: ~1.5GB VRAM

**If GPU is not detected:**

1. **Install NVIDIA drivers:**
   ```bash
   # Ubuntu/Debian
   sudo ubuntu-drivers autoinstall

   # Fedora
   sudo dnf install akmod-nvidia
   ```

2. **Install CUDA toolkit:**
   ```bash
   # Ubuntu
   sudo apt install nvidia-cuda-toolkit

   # Verify installation
   nvcc --version
   ```

3. **Restart Ollama:**
   ```bash
   sudo systemctl restart ollama
   ```

---

### Resource Limits

Control Ollama's resource usage:

```bash
# Limit CPU cores
export OLLAMA_NUM_THREAD=4
ollama serve

# Limit GPU memory (in MB)
export OLLAMA_MAX_VRAM=4096
ollama serve

# Limit CPU usage (systemd on Linux)
sudo systemctl edit ollama
# Add:
# [Service]
# CPUQuota=50%
```

---

### Custom Models

Use alternative embedding models:

```bash
# List available models
ollama list

# Pull alternative model
ollama pull mxbai-embed-large

# Configure Maproom to use it
export OLLAMA_MODEL=mxbai-embed-large
crewchief maproom scan --generate-embeddings
```

**Model comparison:**

| Model | Dimensions | Quality | Speed | Use Case |
|-------|-----------|---------|-------|----------|
| `nomic-embed-text` | 768 | ⭐⭐⭐⭐ | Fast | Balanced (default) |
| `mxbai-embed-large` | 1024 | ⭐⭐⭐⭐⭐ | Slower | Maximum quality |
| `all-minilm` | 384 | ⭐⭐⭐ | Very Fast | Speed priority |

---

### Network Configuration

Run Ollama on a custom port:

```bash
# Start Ollama on different port
OLLAMA_HOST=0.0.0.0:8080 ollama serve

# Configure Maproom to use custom port
export OLLAMA_BASE_URL=http://localhost:8080
```

**Remote Ollama server:**

```bash
# Point Maproom to remote Ollama instance
export OLLAMA_BASE_URL=http://remote-server:11434
crewchief maproom scan --generate-embeddings
```

---

### Model Management

```bash
# List installed models
ollama list

# Remove unused models to free disk space
ollama rm old-model-name

# Update model to latest version
ollama pull nomic-embed-text

# Show model details
ollama show nomic-embed-text
```

---

## Performance Optimization

### For Development

- **Use incremental indexing**: Only process changed files
  ```bash
  crewchief maproom scan --incremental --generate-embeddings
  ```

- **Index only specific paths**:
  ```bash
  crewchief maproom scan src/ --generate-embeddings
  ```

- **Use `.maproomignore`**: Exclude unnecessary files
  ```
  # .maproomignore
  node_modules/
  dist/
  *.test.ts
  ```

### For Large Codebases

- **Enable GPU acceleration**: 10x-50x faster
- **Batch processing**: Maproom handles this automatically
- **Parallel processing**: Ollama processes multiple chunks simultaneously
- **Caching**: Embeddings are cached - only changed files are re-indexed

### Benchmarks

**Medium codebase (50K chunks, 500 chars each):**
- CPU (8 cores): ~45 minutes
- GPU (RTX 3060): ~5 minutes
- GPU (RTX 4090): ~2 minutes

**Large codebase (500K chunks):**
- CPU (8 cores): ~8 hours
- GPU (RTX 3060): ~45 minutes
- GPU (RTX 4090): ~15 minutes

---

## Security & Privacy

### Data Privacy

**✅ Complete privacy**: All embeddings are generated locally
- Code never leaves your machine
- No API calls to external services
- No telemetry or usage tracking

**Ideal for:**
- Proprietary codebases
- Sensitive/classified code
- Air-gapped environments
- GDPR/compliance requirements

### Model Security

**✅ Best practices:**
- Download models from official Ollama registry only
- Verify model checksums: `ollama show nomic-embed-text`
- Keep Ollama updated for security patches: `ollama update` (if available) or reinstall

### Network Security

**✅ Local-only access:**
```bash
# Bind to localhost only (default)
OLLAMA_HOST=127.0.0.1:11434 ollama serve

# For development servers, restrict access
OLLAMA_HOST=0.0.0.0:11434 ollama serve  # ⚠️ Exposes to network
```

**⚠️ Warning**: Only expose Ollama to network if you understand the security implications.

---

## Migration & Compatibility

### Switching from Cloud Providers

If you're migrating from Google Vertex AI or OpenAI to Ollama:

1. **Different embedding dimensions**:
   - OpenAI (text-embedding-3-small): 1536D
   - Google Vertex AI: 768D
   - Ollama (nomic-embed-text): 768D

   You'll need to **re-generate all embeddings** - they're not compatible.

2. **Re-index your codebase**:
   ```bash
   # Set provider to Ollama
   export EMBEDDING_PROVIDER=ollama

   # Clear old embeddings and re-index
   crewchief maproom scan --generate-embeddings --force
   ```

3. **Test search quality**:
   ```bash
   crewchief maproom search "your typical query"
   ```

**See full migration guide**: [Migration Guide](../migration-guide.md) (if available)

---

## Comparison with Other Providers

### Ollama vs Google Vertex AI

| Feature | Ollama | Google Vertex AI |
|---------|--------|------------------|
| **Cost** | ✅ Free | ~$0.00025 / 1K chars |
| **Privacy** | ✅ Local only | Cloud (encrypted) |
| **Setup** | Easy (5 min) | Medium (15 min) |
| **Offline** | ✅ Yes | ❌ No |
| **SLA** | ❌ No | ✅ Yes |
| **GPU Required** | Optional (faster) | N/A |

### Ollama vs OpenAI

| Feature | Ollama | OpenAI |
|---------|--------|--------|
| **Cost** | ✅ Free | ~$0.00003 / 1K chars |
| **Dimensions** | 768 | 1536 |
| **Privacy** | ✅ Local only | Cloud (encrypted) |
| **Setup** | Easy (5 min) | Easy (5 min) |
| **Speed** | Fast (GPU) | Fast |

**See detailed comparison**: [Provider Comparison Guide](./comparison.md)

---

## Quick Reference

### Common Commands

```bash
# Installation
curl -fsSL https://ollama.ai/install.sh | sh

# Start service
ollama serve                           # macOS/Windows
sudo systemctl start ollama            # Linux

# Model management
ollama pull nomic-embed-text          # Download model
ollama list                           # List installed models
ollama show nomic-embed-text          # Model details

# Configuration
export EMBEDDING_PROVIDER=ollama      # Set provider
export OLLAMA_MODEL=nomic-embed-text  # Set model

# Indexing
crewchief maproom scan --generate-embeddings         # Full index
crewchief maproom scan --incremental --generate-embeddings  # Changed files only

# Testing
curl http://localhost:11434/api/tags  # Check service
crewchief maproom search "query"      # Test search
```

---

## Additional Resources

### Official Documentation

- **[Ollama Official Docs](https://ollama.ai/docs)** - Complete Ollama documentation
- **[Ollama GitHub](https://github.com/ollama/ollama)** - Source code and issue tracker
- **[Nomic Embed Model Card](https://huggingface.co/nomic-ai/nomic-embed-text-v1)** - Model details and benchmarks

### Maproom Documentation

- **[Provider Comparison Guide](./comparison.md)** - Compare all providers
- **[Migration Guide](./migration-guide.md)** - Switch between providers
- **[Configuration Guide](../../crates/maproom/docs/configuration_guide.md)** - Full Maproom configuration
- **[Performance Tuning](../../crates/maproom/docs/PERFORMANCE_TUNING.md)** - Optimize search performance

### Community

- **[Ollama Discord](https://discord.gg/ollama)** - Community support
- **[Maproom Discussions](https://github.com/yourusername/maproom/discussions)** - Ask questions, share tips
- **[GitHub Issues](https://github.com/yourusername/maproom/issues)** - Report bugs

---

## Need Help?

**Installation issues?** Check the [Troubleshooting section](#troubleshooting)

**Slow performance?** See [Performance Optimization](#performance-optimization)

**Switching providers?** Read the [Migration Guide](./migration-guide.md)

**Have feedback?** Open an issue or discussion on GitHub

---

**Last Updated**: October 2025
