# Embedding Provider Setup Guides

Maproom supports multiple embedding providers for semantic code search. Choose the provider that best fits your use case, cost requirements, and deployment environment.

## Available Providers

### [Google Vertex AI](./google-vertex-ai-setup.md)

Production-grade embeddings powered by Google's `text-embedding-gecko@003` model.

**Best for:**
- Production deployments requiring SLA-backed uptime
- Enterprise compliance (SOC 2, ISO, data residency)
- Global scalability with multi-region support
- High-quality 768-dimensional embeddings

**Pricing**: ~$0.00025 per 1,000 characters

**Setup Time**: ~15 minutes

**[→ Read Google Vertex AI Setup Guide](./google-vertex-ai-setup.md)**

---

### [Ollama](./ollama-setup.md) (Local Embeddings)

Free, local embeddings running on your own hardware. No API costs, complete privacy.

**Best for:**
- Development and testing without API costs
- Air-gapped or offline environments
- Privacy-sensitive codebases
- Local-first workflows

**Pricing**: Free (hardware costs only)

**Setup Time**: ~5 minutes (requires Ollama installation)

**[→ Read Ollama Setup Guide](./ollama-setup.md)**

---

### [OpenAI](./openai-setup.md) (text-embedding-3-small)

High-quality embeddings from OpenAI's embedding models.

**Best for:**
- Existing OpenAI API users
- 1536-dimensional embeddings (higher than Google's 768)
- Integration with other OpenAI services

**Pricing**: ~$0.00002 per 1,000 tokens (approximately $0.00003 per 1,000 characters)

**Setup Time**: ~5 minutes

**[→ Read OpenAI Setup Guide](./openai-setup.md)**

---

## Provider Comparison

**[→ Read Full Comparison Guide](./comparison.md)** for detailed cost analysis, performance benchmarks, privacy models, compliance details, and use case recommendations.

### Quick Comparison Table

| Feature | Google Vertex AI | Ollama | OpenAI |
|---------|------------------|--------|--------|
| **Embedding Dimensions** | 768 | Varies by model | 1536 (text-embedding-3-small) |
| **Pricing** | ~$0.00025 / 1K chars | Free | ~$0.00003 / 1K chars |
| **Setup Complexity** | Medium (GCP setup) | Low (local install) | Low (API key) |
| **Latency** | ~100-300ms | ~50-200ms (local) | ~100-300ms |
| **Offline Support** | ❌ No | ✅ Yes | ❌ No |
| **Production SLA** | ✅ Yes | ❌ No | ✅ Yes |
| **Data Privacy** | Cloud (encrypted) | ✅ Local only | Cloud (encrypted) |
| **Regional Support** | ✅ Multi-region | N/A | ✅ Global |
| **Enterprise Compliance** | ✅ SOC 2, ISO | ✅ Self-hosted | ✅ SOC 2, ISO |

**[→ See detailed breakdown in Comparison Guide](./comparison.md)**

---

## Quick Start

### 1. Choose Your Provider

**New to embeddings?** Start with **Ollama** for free local experimentation.

**Production deployment?** Use **Google Vertex AI** or **OpenAI** for reliability and SLA.

**Already using GCP?** Use **Google Vertex AI** for seamless integration.

**Already using OpenAI?** Use **OpenAI** embeddings for consistency.

### 2. Follow Setup Guide

Click on your chosen provider above to view detailed setup instructions.

### 3. Configure Maproom

Set the `EMBEDDING_PROVIDER` environment variable:

```bash
# For Google Vertex AI
export EMBEDDING_PROVIDER="google"

# For Ollama
export EMBEDDING_PROVIDER="ollama"

# For OpenAI
export EMBEDDING_PROVIDER="openai"
```

### 4. Verify Setup

```bash
# Test your configuration
crewchief maproom scan --dry-run

# Should show:
# ✓ Using embedding provider: google (768 dimensions)
# OR
# ✓ Using embedding provider: ollama (384 dimensions)
# OR
# ✓ Using embedding provider: openai (1536 dimensions)
```

---

## Switching Providers

You can switch between providers at any time. See the [Migration Guide](./migration-guide.md) for detailed instructions on:

- Re-generating embeddings with a new provider
- Preserving existing embeddings during migration
- Testing provider performance before committing
- Rollback procedures

---

## Provider-Specific Features

### Google Vertex AI

- **Task types**: Optimize embeddings for different use cases (retrieval, similarity, clustering)
- **Regional endpoints**: Deploy in specific geographic regions for compliance
- **Workload Identity**: Keyless authentication for GKE/Cloud Run
- **Quota management**: Fine-grained control over API usage

**[→ Full feature list in Google setup guide](./google-vertex-ai-setup.md)**

### Ollama

- **Model selection**: Choose from multiple embedding models (e.g., `nomic-embed-text`, `mxbai-embed-large`)
- **GPU acceleration**: Leverage local GPU for faster embedding generation
- **Customizable**: Fine-tune models for domain-specific embeddings

**[→ Full feature list in Ollama setup guide](./ollama-setup.md)**

### OpenAI

- **Latest models**: Access to newest embedding models as they're released
- **Batch processing**: Efficient batch API for large-scale indexing
- **Integration**: Works seamlessly with other OpenAI services

**[→ Full feature list in OpenAI setup guide](./openai-setup.md)**

---

## Cost Optimization

### All Providers

1. **Enable caching** - Maproom caches embeddings automatically to avoid re-generation
2. **Selective indexing** - Use `.maproomignore` to exclude unnecessary files
3. **Incremental updates** - Only re-index changed files

### Cloud Providers (Google, OpenAI)

4. **Batch processing** - Maproom batches requests automatically
5. **Set budget alerts** - Monitor costs in provider dashboards
6. **Use test environments** - Develop with Ollama, deploy with cloud providers

### Google Vertex AI Specific

6. **Regional selection** - Choose closest region to minimize latency
7. **Quota management** - Request quota increases for large codebases
8. **Workload Identity** - Reduce key management overhead

**See provider setup guides for detailed cost estimation and optimization strategies.**

---

## Security Best Practices

### Service Account Keys (Google)

- ✅ Use least-privilege IAM roles (`roles/aiplatform.user`)
- ✅ Rotate keys every 90 days
- ✅ Set file permissions to 600 (owner read/write only)
- ✅ Never commit keys to version control
- ✅ Use Workload Identity for production (GKE, Cloud Run)

**[→ Full security guide](./google-vertex-ai-setup.md#security-best-practices)**

### API Keys (OpenAI)

- ✅ Store in environment variables, not code
- ✅ Rotate keys periodically
- ✅ Use project-scoped keys with minimum permissions
- ✅ Monitor usage for anomalies

### Ollama (Local)

- ✅ No credentials needed - fully local
- ✅ Ensure model files are from trusted sources
- ✅ Keep Ollama updated for security patches

---

## Troubleshooting

### Common Issues Across All Providers

**Issue**: Embeddings not generating
- **Solution**: Verify `EMBEDDING_PROVIDER` environment variable is set correctly

**Issue**: Slow embedding generation
- **Solution**: Check network latency (cloud providers) or hardware resources (Ollama)

### Provider-Specific Troubleshooting

**Google Vertex AI**: [See troubleshooting section](./google-vertex-ai-setup.md#troubleshooting)
- 403 Forbidden errors → IAM permissions
- 429 Quota exceeded → Request quota increase
- Invalid JWT signature → Regenerate service account key

**Ollama**: [See troubleshooting section](./ollama-setup.md#troubleshooting)
- Connection refused → Start Ollama service
- Model not found → Pull embedding model
- Slow generation → Enable GPU acceleration

**OpenAI**: [See troubleshooting section](./openai-setup.md#troubleshooting)
- Invalid API key → Regenerate key
- Rate limit exceeded → Wait or upgrade tier
- Insufficient quota → Add billing credits

---

## Additional Resources

### Official Provider Documentation

- **[Google Vertex AI Documentation](https://cloud.google.com/vertex-ai/docs)**
- **[Ollama Documentation](https://ollama.ai/docs)**
- **[OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)**

### Maproom Documentation

- **[Configuration Guide](../../crates/maproom/docs/configuration_guide.md)** - Full Maproom configuration reference
- **[Performance Tuning](../../crates/maproom/docs/PERFORMANCE_TUNING.md)** - Optimize search performance
- **[Monitoring Guide](../../crates/maproom/docs/monitoring_guide.md)** - Production monitoring

### Community

- **[Maproom GitHub Issues](https://github.com/yourusername/maproom/issues)** - Report bugs, request features
- **[Discussions](https://github.com/yourusername/maproom/discussions)** - Ask questions, share tips

---

## Need Help?

**Can't decide which provider to use?** Consider:
- **Budget constraints** → Start with Ollama (free)
- **Production requirements** → Google Vertex AI or OpenAI
- **Privacy requirements** → Ollama (fully local)
- **Existing cloud investment** → Match your cloud provider (GCP → Google, AWS/Other → OpenAI)

**Having setup issues?** Check the troubleshooting section in your provider's setup guide.

**Want to contribute?** Provider setup improvements are always welcome! Open a PR on GitHub.

---

**Last Updated**: October 2025
