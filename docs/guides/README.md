# Maproom User Guides

Comprehensive guides for using Maproom's semantic code search and embedding features.

## Available Guides

### [Provider Migration Guide](./provider-migration.md)

**Migrate between embedding providers without losing data**

This guide covers:
- Switching from OpenAI to Ollama (and vice versa)
- Preserving existing embeddings during migration
- Running multiple providers simultaneously
- Re-indexing strategies and cost comparisons
- Step-by-step migration scenarios with code examples
- Rollback procedures and troubleshooting

**Who should read this**: Existing Maproom users who want to change embedding providers, optimize costs, or test different providers.

**Read the guide**: [Provider Migration Guide →](./provider-migration.md)

---

## Related Documentation

### Provider Setup

New to Maproom? Start with the provider setup guides:

- **[Provider Setup Guides](../providers/README.md)** - Choose and configure your embedding provider
- **[OpenAI Setup](../providers/openai-setup.md)** - Set up OpenAI embeddings
- **[Ollama Setup](../providers/ollama-setup.md)** - Set up local Ollama embeddings
- **[Google Vertex AI Setup](../providers/google-vertex-ai-setup.md)** - Set up Google Cloud embeddings
- **[Provider Comparison](../providers/comparison.md)** - Compare features, costs, and performance

### Technical Documentation

For developers and advanced users:

- **[Configuration Guide](../../crates/maproom/docs/configuration_guide.md)** - Full configuration reference
- **[Performance Tuning](../../crates/maproom/docs/PERFORMANCE_TUNING.md)** - Optimize search performance
- **[Database Architecture](../architecture/DATABASE_ARCHITECTURE.md)** - PostgreSQL setup and schemas

---

## Quick Navigation

**Just starting?** → [Provider Setup Guides](../providers/README.md)

**Switching providers?** → [Migration Guide](./provider-migration.md)

**Comparing options?** → [Provider Comparison](../providers/comparison.md)

**Need troubleshooting?** → See individual provider guides

---

**Last Updated**: October 2025
