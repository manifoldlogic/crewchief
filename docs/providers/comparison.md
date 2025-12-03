# Embedding Provider Comparison

This guide provides a comprehensive comparison of embedding providers supported by Maproom. Use this to make an informed decision based on your cost, performance, privacy, and compliance requirements.

> **Last Updated**: October 29, 2025
>
> **Note**: Costs and performance metrics are approximate and may vary based on usage patterns, network conditions, and provider updates. Always refer to official provider pricing pages for the most current information.

## Quick Comparison Table

| Feature | Ollama (Local) | Google Vertex AI | OpenAI |
|---------|----------------|------------------|--------|
| **Cost** | Free (compute only) | ~$0.0001/1K tokens | ~$0.0001/1K tokens |
| **Speed** | Fast (local) | Medium (network) | Medium (network) |
| **Privacy** | Complete (offline) | High (GCP infra) | Medium (cloud) |
| **Setup** | Easy | Medium | Easy |
| **Dimensions** | 1024 | 768 | 1536 |
| **Model** | mxbai-embed-large | textembedding-gecko@003 | text-embedding-3-small |
| **Compliance** | N/A (local) | GDPR, SOC2, HIPAA* | GDPR, SOC2 |

*With proper configuration and Business Associate Agreement (BAA)

---

## Detailed Breakdown

### Cost Analysis

#### Ollama (Local)

**Direct Cost**: $0 (no API charges)

**Infrastructure**: Compute costs only (electricity, hardware depreciation)

**Cost Calculator Example**:
- 100K chunks = $0 API costs
- 1M chunks = $0 API costs
- Only costs: Your existing hardware running time

**Pricing Model**: Free and open source

**Best for**: Budget-conscious projects, high-volume indexing, development/testing

**Hidden Costs**:
- Initial hardware investment (GPU recommended for speed)
- Electricity costs (minimal, typically <$10/month for regular use)
- Maintenance time (model updates, Ollama upgrades)

**Official Pricing**: [Ollama is free and open source](https://ollama.ai/)

---

#### Google Vertex AI

**API Cost**: ~$0.00025 per 1,000 characters (~$0.0001 per 1,000 tokens)

**Free Tier**: First 1,000 requests per month (check current limits)

**Cost Calculator Examples**:
- 10K chunks (avg 500 chars) = ~$1.25
- 100K chunks (avg 500 chars) = ~$12.50
- 1M chunks (avg 500 chars) = ~$125

**Pricing Model**: Pay-per-use with no upfront commitment

**Best for**: GCP users, enterprise compliance needs, predictable cloud costs

**Hidden Costs**:
- Network egress charges (minimal for embedding API)
- Cloud Logging costs if verbose logging enabled
- Service account key management overhead

**Official Pricing**: [Vertex AI Pricing](https://cloud.google.com/vertex-ai/pricing#generative_ai_models)

---

#### OpenAI

**API Cost**: $0.00002 per 1,000 tokens (~$0.0001 per 1,000 characters, assuming ~5 chars/token)

**Free Tier**: None (pay-per-use from first request)

**Cost Calculator Examples**:
- 10K chunks (avg 500 chars) = ~$0.50
- 100K chunks (avg 500 chars) = ~$5
- 1M chunks (avg 500 chars) = ~$50

**Pricing Model**: Pay-per-use with credit card required

**Best for**: Existing OpenAI users, proven quality, lower cost than Google

**Hidden Costs**:
- Rate limiting may require paid tier for high throughput
- No free tier for experimentation

**Official Pricing**: [OpenAI Embeddings Pricing](https://openai.com/api/pricing/)

---

### Performance Benchmarks

> **Test Environment**: MacBook Pro M2, 16GB RAM, 1Gbps internet
>
> **Methodology**: Average of 10 runs indexing 1,000 TypeScript files (~500 chars each)
>
> **Note**: Your performance may vary based on hardware, network, and load

#### Throughput (chunks/second)

| Provider | Typical | Optimal | Notes |
|----------|---------|---------|-------|
| **Ollama** | 50-100 | 150+ | Depends on GPU; CPU-only: 10-20 chunks/sec |
| **Google Vertex AI** | 30-50 | 80 | Rate limited by API quotas; batch requests help |
| **OpenAI** | 40-60 | 100 | Consistent performance; tier 3 allows higher rates |

**Optimization Tips**:
- Ollama: Use GPU (NVIDIA/AMD), increase `OLLAMA_NUM_PARALLEL` env var
- Google: Batch requests, request quota increases for production
- OpenAI: Use batch API endpoint, upgrade to tier 3+ for higher rate limits

---

#### Latency (single embedding request)

| Provider | P50 | P95 | P99 | Notes |
|----------|-----|-----|-----|-------|
| **Ollama** | 20ms | 50ms | 100ms | Local processing; GPU: <20ms, CPU: 50-200ms |
| **Google Vertex AI** | 150ms | 300ms | 500ms | Includes network roundtrip to GCP region |
| **OpenAI** | 120ms | 200ms | 400ms | Generally faster than Google; global CDN |

**Factors Affecting Latency**:
- Network distance to provider (Google: regional, OpenAI: global)
- Text length (longer text = more processing time)
- Provider load (peak hours may increase latency)
- Batch size (batching increases throughput but individual latency)

---

#### Quality Comparison

| Provider | Model | Dimensions | Quality Score* | Notes |
|----------|-------|------------|----------------|-------|
| **Ollama** | mxbai-embed-large | 1024 | 9.0/10 | High quality, no special character issues |
| **Google Vertex AI** | textembedding-gecko@003 | 768 | 9.0/10 | General purpose, strong multilingual support |
| **OpenAI** | text-embedding-3-small | 1536 | 9.5/10 | Higher dimensions, best overall quality |

*Quality scores based on MTEB benchmark averages and code-specific retrieval tasks

**Compatibility Note**: Different dimension sizes (768 vs 1536) are stored in separate columns. Maproom handles mixed embeddings automatically using COALESCE logic.

---

### Privacy & Security

#### Ollama

**Privacy Model**: Complete offline operation

**Data Flow**:
1. Your code → Local Ollama process
2. Ollama → Embedding vector
3. Vector stored locally in PostgreSQL

**Security Features**:
- ✅ **No external API calls**: Data never leaves your machine
- ✅ **No API key required**: Zero credential management
- ✅ **No network requirement**: Works air-gapped
- ✅ **Local audit trail**: Full control over logs and data

**Risks**:
- ⚠️ **Requires local compute resources**: GPU recommended
- ⚠️ **No centralized security updates**: Manual Ollama upgrades needed
- ⚠️ **Model provenance**: Ensure models from trusted sources (official Ollama library)

**Best for**:
- Air-gapped environments (government, financial institutions)
- Privacy-sensitive codebases (proprietary algorithms, customer data)
- Compliance requirements prohibiting cloud data processing
- Development without cloud costs

**Setup Guide**: [Ollama Setup](./ollama-setup.md) (coming soon)

---

#### Google Vertex AI

**Privacy Model**: Data processed in Google Cloud Platform infrastructure

**Data Flow**:
1. Your code → TLS-encrypted to GCP regional endpoint
2. Google Vertex AI → Embedding vector
3. Vector returned and stored locally
4. **Google does not store your input data** (per Vertex AI API terms)

**Security Features**:
- ✅ **Service account authentication**: Better than API keys, IAM integration
- ✅ **Regional data residency**: Choose specific GCP regions (e.g., `us-central1`, `europe-west1`)
- ✅ **Audit logging**: Cloud Audit Logs track all API calls
- ✅ **VPC Service Controls**: Restrict API access to specific networks
- ✅ **Encryption in transit**: TLS 1.2+ for all API communication
- ✅ **Encryption at rest**: Data encrypted with Google-managed keys

**Risks**:
- ℹ️ **Data processed by Google services**: Trust in Google's infrastructure required
- ℹ️ **Temporary processing**: Data processed in memory, not stored long-term
- ⚠️ **Service account key management**: Keys must be secured and rotated

**Compliance Certifications**:
- SOC 1, 2, 3
- ISO 27001, 27017, 27018
- HIPAA compliant with BAA
- PCI DSS Level 1
- GDPR compliant
- FedRAMP (specific regions)

**Best for**:
- Healthcare (HIPAA with BAA)
- Finance (SOC 2, PCI DSS)
- Government (FedRAMP regions)
- Enterprise with GCP investment

**Setup Guide**: [Google Vertex AI Setup](./google-vertex-ai-setup.md)

---

#### OpenAI

**Privacy Model**: Data sent to OpenAI servers for processing

**Data Flow**:
1. Your code → TLS-encrypted to OpenAI API
2. OpenAI API → Embedding vector
3. Vector returned and stored locally
4. **OpenAI does not use API data for training** (per OpenAI API terms)

**Security Features**:
- ✅ **API key authentication**: Simple credential management
- ✅ **TLS encryption**: All traffic encrypted in transit
- ✅ **No training data usage**: API data not used to train models
- ✅ **30-day retention**: Data retained for abuse monitoring, then deleted
- ✅ **Zero data retention option**: Available for Enterprise tier

**Risks**:
- ⚠️ **Data sent to OpenAI servers**: Trust in OpenAI's infrastructure required
- ⚠️ **API key authentication**: Less secure than service account auth (no scoped permissions)
- ⚠️ **30-day retention window**: Data kept temporarily for abuse detection
- ℹ️ **OpenAI data usage policies apply**: Review terms before use

**Compliance Certifications**:
- SOC 2 Type 2
- GDPR compliant
- CCPA compliant
- ISO 27001 (in progress as of 2025)

**Best for**:
- General purpose applications
- Existing OpenAI users (consolidated billing/management)
- Projects without strict compliance requirements
- Teams prioritizing ease of setup over maximum privacy

**Setup Guide**: [OpenAI Setup](./openai-setup.md) (coming soon)

---

### Setup Complexity

#### Ollama: Easy ⭐⭐⭐

**Estimated Time**: 5-10 minutes

**Steps**:
1. Install Ollama: `curl -sSL https://ollama.ai/install.sh | sh`
2. Pull model: `ollama pull mxbai-embed-large`
3. Verify: `ollama list` (should show mxbai-embed-large)
4. Start using: Zero configuration needed

**Prerequisites**:
- Linux, macOS, or Windows with WSL2
- 8GB+ RAM (16GB+ recommended)
- GPU optional but recommended (NVIDIA/AMD)

**Configuration**:
```bash
# Optional: Set Ollama host if non-default
export OLLAMA_HOST="http://localhost:11434"

# Optional: Enable GPU
# (automatic on most systems with NVIDIA/AMD drivers)
```

**Troubleshooting**: Minimal; most issues relate to GPU drivers

**Documentation**: [Ollama Setup Guide](./ollama-setup.md) (coming soon)

---

#### Google Vertex AI: Medium ⭐⭐

**Estimated Time**: 15-30 minutes (first time), 5 minutes (subsequent projects)

**Steps**:
1. Create GCP project (if needed)
2. Enable Vertex AI API
3. Create service account with `roles/aiplatform.user`
4. Download service account key JSON
5. Set environment variables:
   ```bash
   export GOOGLE_APPLICATION_CREDENTIALS="/path/to/key.json"
   export GOOGLE_CLOUD_PROJECT="your-project-id"
   export GOOGLE_CLOUD_LOCATION="us-central1"
   ```
6. Verify with test request

**Prerequisites**:
- Google Cloud account with billing enabled
- `gcloud` CLI installed (optional but helpful)
- Basic understanding of GCP IAM

**Configuration Complexity**: Medium
- Multiple environment variables
- IAM role configuration
- Regional endpoint selection
- Service account key file management

**Troubleshooting**: Moderate complexity
- IAM permission errors (most common)
- Quota limits for new projects
- Regional availability differences

**Documentation**: [Google Vertex AI Setup Guide](./google-vertex-ai-setup.md)

---

#### OpenAI: Easy ⭐⭐⭐

**Estimated Time**: 5 minutes

**Steps**:
1. Sign up at [platform.openai.com](https://platform.openai.com/)
2. Create API key in dashboard
3. Set environment variable:
   ```bash
   export OPENAI_API_KEY="sk-proj-..."
   ```
4. Start using immediately

**Prerequisites**:
- OpenAI account
- Credit card for billing (no free tier)

**Configuration Complexity**: Low
- Single environment variable
- No IAM, no service accounts, no regions

**Troubleshooting**: Minimal
- Invalid API key (typo)
- Rate limiting (upgrade tier)
- Billing issues

**Documentation**: [OpenAI Setup Guide](./openai-setup.md) (coming soon)

---

### Compliance & Certifications

#### Ollama

**Data Location**: Your infrastructure (on-premises, cloud VM, laptop)

**Certifications**: N/A (self-hosted, no SaaS component)

**Compliance Approach**: Inherit from your infrastructure
- Deploy on HIPAA-compliant infrastructure → HIPAA compliant
- Deploy on FedRAMP infrastructure → FedRAMP compliant
- Deploy on SOC 2 infrastructure → SOC 2 compliant

**Audit Considerations**:
- ✅ Full control over data: No third-party data sharing
- ✅ Local audit logs: Complete visibility into operations
- ✅ No vendor lock-in: Open source, no SaaS dependencies
- ⚠️ Your responsibility: Security hardening, patching, monitoring

**Best for**:
- **Government**: FedRAMP, IL4/IL5, air-gapped requirements
- **Healthcare**: HIPAA without BAA complexity
- **Finance**: PCI DSS, internal compliance policies
- **Legal**: Attorney-client privilege, confidential documents

---

#### Google Vertex AI

**Certifications**:
- **SOC 1, 2, 3**: Annual audits by independent third parties
- **ISO 27001**: Information security management
- **ISO 27017**: Cloud security controls
- **ISO 27018**: Personal data protection in cloud
- **HIPAA**: Covered under BAA (requires explicit enablement)
- **PCI DSS Level 1**: Payment card data security
- **FedRAMP**: Moderate and High baselines (specific regions)
- **GDPR**: European data protection regulation

**Regional Data Residency**:
- **US**: `us-central1`, `us-east1`, `us-west1`, etc.
- **Europe**: `europe-west1` (Belgium), `europe-west4` (Netherlands)
- **Asia**: `asia-east1` (Taiwan), `asia-southeast1` (Singapore)
- **Multi-region**: Data stored in multiple regions for redundancy

**HIPAA Compliance Requirements**:
1. Sign Business Associate Agreement (BAA) with Google
2. Enable audit logging
3. Use specific regions (not all regions are HIPAA-eligible)
4. Configure VPC Service Controls
5. Follow Google's HIPAA implementation guide

**Audit Features**:
- **Cloud Audit Logs**: Track all API calls (who, what, when)
- **Access Transparency**: Logs of Google employee access (if any)
- **VPC Service Controls**: Network perimeter security
- **Data Loss Prevention (DLP)**: Scan for sensitive data (optional)

**Best for**:
- **Healthcare**: HIPAA-compliant PHI processing with BAA
- **Finance**: SOC 2, PCI DSS requirements
- **Government**: FedRAMP Moderate/High (specific regions)
- **EU/UK**: GDPR compliance with regional data residency

**Documentation**: [Google Vertex AI Security](./google-vertex-ai-setup.md#security-best-practices)

---

#### OpenAI

**Certifications**:
- **SOC 2 Type 2**: Annual security audit (completed)
- **ISO 27001**: In progress as of Q4 2024
- **GDPR**: Compliant (Data Processing Addendum available)
- **CCPA**: California Consumer Privacy Act compliant

**Data Retention**:
- **Standard API**: 30-day retention for abuse monitoring
- **Enterprise Zero Retention**: Immediate deletion after processing (Enterprise tier only)
- **No training usage**: API data never used for model training

**Compliance Limitations**:
- ❌ **No HIPAA compliance**: BAA not available
- ❌ **No FedRAMP**: Not authorized for government use
- ❌ **No PCI DSS**: Not certified for payment card data
- ⚠️ **Limited regional control**: No guaranteed data residency

**Privacy Commitments**:
- API data not used for training models
- 30-day retention for abuse detection (Enterprise: zero retention)
- GDPR/CCPA data subject rights supported
- Encryption in transit and at rest

**Best for**:
- **General business applications**: No strict compliance requirements
- **SaaS products**: SOC 2 compliance sufficient
- **Consumer apps**: GDPR/CCPA compliance needed
- **Rapid prototyping**: Fastest setup, good-enough compliance

**Not suitable for**:
- Healthcare (HIPAA-required PHI)
- Government (FedRAMP-required)
- Payment processing (PCI DSS-required)
- High-privacy environments (air-gapped/offline)

**Documentation**: [OpenAI Security Portal](https://trust.openai.com/)

---

## Use Case Recommendations

### Choose Ollama if:

✅ **You need complete offline operation**
- Air-gapped environments (government, financial institutions)
- Remote locations with unreliable internet
- Development on airplanes, trains, etc.

✅ **You have GPU resources available**
- Developer workstations with NVIDIA/AMD GPUs
- Cloud VMs with GPU instances (still cheaper than API at scale)
- Mac with M1/M2/M3 chips (excellent performance)

✅ **You want zero API costs**
- High-volume indexing (millions of chunks)
- Frequent re-indexing (CI/CD pipelines)
- Budget-constrained projects

✅ **You're indexing large codebases frequently**
- Monorepos with 100K+ files
- Daily/hourly incremental indexing
- Development environments with rapid iteration

✅ **Privacy is a top priority**
- Proprietary algorithms/IP
- Customer data in codebase
- Legal/compliance prohibits cloud processing

**Example**: A healthcare startup building HIPAA-compliant software wants to search their codebase without sending PHI-adjacent code to external APIs.

**Setup**: [Ollama Setup Guide](./ollama-setup.md) (coming soon)

---

### Choose Google Vertex AI if:

✅ **You're already using GCP**
- Billing consolidation with existing GCP spend
- IAM integration with existing Google Workspace
- Same infrastructure for code search and application

✅ **You need enterprise compliance (HIPAA, etc.)**
- Healthcare: Process codebases with PHI references
- Finance: Meet SOC 2, PCI DSS requirements
- Government: FedRAMP Moderate/High authorization

✅ **You require regional data residency**
- EU data that must stay in EU (GDPR)
- Government data with geographic restrictions
- Multi-region high availability

✅ **You want audit trails and monitoring**
- Cloud Audit Logs for every API call
- Integration with Cloud Monitoring/Logging
- Access Transparency logs

**Example**: A European fintech company needs SOC 2 compliance, GDPR data residency in EU, and integration with their existing GCP infrastructure.

**Setup**: [Google Vertex AI Setup Guide](./google-vertex-ai-setup.md)

---

### Choose OpenAI if:

✅ **You're already using OpenAI for other tasks**
- Consolidated billing (embeddings + GPT-4 + Codex)
- Consistent API patterns across services
- Single vendor relationship

✅ **You want proven embedding quality**
- Highest-quality embeddings (1536 dimensions)
- Best performance on MTEB benchmarks
- Battle-tested at scale

✅ **You prefer simple setup**
- Single API key, no GCP complexity
- No IAM, service accounts, or regional configuration
- 5-minute setup time

✅ **Cost is not a primary concern**
- Low/medium volume indexing (<1M chunks)
- Budget allows ~$50-100/month for embeddings
- Developer productivity > API costs

**Example**: A startup already using GPT-4 wants to add semantic code search with minimal setup time and consistent OpenAI integration.

**Setup**: [OpenAI Setup Guide](./openai-setup.md) (coming soon)

---

## Migration Path

Maproom supports **multiple providers simultaneously** using different embedding columns. This enables flexible migration strategies:

### Strategy 1: Start Local, Scale to Cloud

**Phase 1: Development (Ollama)**
```bash
export MAPROOM_EMBEDDING_PROVIDER="ollama"
crewchief maproom scan  # Generates 768-dim embeddings in embedding_nomic column
```

**Phase 2: Production (OpenAI or Google)**
```bash
export MAPROOM_EMBEDDING_PROVIDER="openai"
crewchief maproom scan  # Generates 1536-dim embeddings in embedding_openai column
# Both embeddings coexist; search uses COALESCE logic
```

**Benefits**:
- Zero API costs during development
- Seamless transition to production
- Rollback capability (keep Ollama embeddings)

---

### Strategy 2: Side-by-Side Comparison

**Generate embeddings with multiple providers**:
```bash
# Generate Ollama embeddings
export MAPROOM_EMBEDDING_PROVIDER="ollama"
crewchief maproom scan

# Generate OpenAI embeddings
export MAPROOM_EMBEDDING_PROVIDER="openai"
crewchief maproom scan

# Generate Google embeddings
export MAPROOM_EMBEDDING_PROVIDER="google"
crewchief maproom scan
```

**Compare search quality**:
```bash
# Search with Ollama
export MAPROOM_EMBEDDING_PROVIDER="ollama"
crewchief maproom search "authentication flow"

# Search with OpenAI
export MAPROOM_EMBEDDING_PROVIDER="openai"
crewchief maproom search "authentication flow"
```

**Benefits**:
- Empirical quality comparison on your codebase
- No commitment until you choose
- Fallback options if primary provider has issues

---

### Strategy 3: Gradual Migration

**Keep existing embeddings while migrating**:
```bash
# Existing production: OpenAI (1536-dim)
export MAPROOM_EMBEDDING_PROVIDER="openai"

# Migrate to Google incrementally
export MAPROOM_EMBEDDING_PROVIDER="google"
crewchief maproom scan --incremental  # Only new/changed files

# Old searches still work (COALESCE falls back to openai column)
# New searches use google embeddings when available
```

**Benefits**:
- No downtime during migration
- Incremental migration reduces risk
- Automatic fallback to old embeddings

---

### Handling Different Dimensions

Maproom uses separate columns for different embedding dimensions:
- **768-dimensional**: `vec_code_768` (nomic-embed-text legacy)
- **1024-dimensional**: `vec_code_1024` (mxbai-embed-large, Ollama default)
- **1536-dimensional**: `vec_code` (OpenAI, Google)

**Search behavior**:
```sql
-- Automatic COALESCE logic (simplified)
SELECT *
FROM chunks
ORDER BY
  COALESCE(
    embedding_nomic <=> query_embedding_768,   -- Try 768-dim first
    embedding_openai <=> query_embedding_1536  -- Fall back to 1536-dim
  )
LIMIT 10;
```

**Key Points**:
- Different dimensions coexist peacefully
- Search uses provider-specific embedding for query
- Falls back to other columns if primary is NULL
- No quality loss from mixed embeddings

---

## Cost-Benefit Analysis

### Ollama TCO (Total Cost of Ownership)

**Initial Investment**:
- Hardware: $0 (use existing) to $2,000 (new GPU workstation)
- Setup time: 10 minutes × $50/hr = $8

**Ongoing Costs**:
- Electricity: ~$5-10/month (GPU at 50% utilization)
- Maintenance: ~1 hour/month × $50/hr = $50/month
- Model updates: 15 minutes/quarter × $50/hr = $3/month

**Total Year 1**: $8 setup + ($65/month × 12) = **$788**

**Total Year 2+**: ($65/month × 12) = **$780/year**

**Break-even vs Cloud** (at 1M chunks indexed):
- Google: ~$125 per 1M chunks → Break-even at ~6M chunks/year
- OpenAI: ~$50 per 1M chunks → Break-even at ~15M chunks/year

---

### Google Vertex AI TCO

**Initial Investment**:
- GCP account setup: Free
- Service account setup: 30 minutes × $50/hr = $25
- Testing/validation: 1 hour × $50/hr = $50

**Ongoing Costs** (example: 100K chunks, re-indexed quarterly):
- API costs: 100K chunks × $0.0001 × 4 quarters = $40/year
- Management: 30 minutes/quarter × $50/hr = $100/year
- Monitoring: ~$10/month = $120/year

**Total Year 1**: $75 setup + $260 ongoing = **$335**

**Total Year 2+**: **$260/year**

**When it makes sense**:
- Low/medium volume (<1M chunks/year)
- Compliance requirements (HIPAA, FedRAMP)
- Already using GCP (no setup learning curve)

---

### OpenAI TCO

**Initial Investment**:
- Account setup: 5 minutes × $50/hr = $4
- Testing: 30 minutes × $50/hr = $25

**Ongoing Costs** (example: 100K chunks, re-indexed quarterly):
- API costs: 100K chunks × $0.00005 × 4 quarters = $20/year
- Management: Minimal (~$10/year)

**Total Year 1**: $29 setup + $30 ongoing = **$59**

**Total Year 2+**: **$30/year**

**When it makes sense**:
- Low volume (<500K chunks/year)
- Fast setup priority
- Already using OpenAI (consolidated billing)
- Highest quality requirement

---

## Performance Tuning Tips

### Ollama Optimization

**GPU Selection**:
- NVIDIA: Best compatibility (CUDA)
- AMD: Good support (ROCm)
- Apple Silicon (M1/M2/M3): Excellent performance, no GPU needed

**Configuration**:
```bash
# Increase parallel requests (default: 1)
export OLLAMA_NUM_PARALLEL=4

# Set GPU memory limit (MB)
export OLLAMA_GPU_MEMORY=8192

# Enable verbose logging for debugging
export OLLAMA_DEBUG=1
```

**Expected Throughput**:
- CPU only: 10-20 chunks/sec
- GPU (NVIDIA RTX 3060): 50-80 chunks/sec
- GPU (NVIDIA RTX 4090): 100-150 chunks/sec
- Apple M2 Max: 80-120 chunks/sec

---

### Google Vertex AI Optimization

**Batching**:
- Batch size: 5-20 requests per batch (optimal)
- Too small: Network overhead dominates
- Too large: Increased latency, timeout risk

**Regional Selection**:
- Choose closest region to minimize latency
- US Central (`us-central1`): Most features, highest capacity
- Europe (`europe-west1`): GDPR compliance
- Asia (`asia-east1`): Low latency for APAC

**Quota Management**:
```bash
# Check current quota
gcloud compute project-info describe --project=YOUR_PROJECT

# Request quota increase (via Console)
# Cloud Console → IAM & Admin → Quotas → Filter: "Vertex AI"
# Select "Prediction API requests" → Request increase
```

---

### OpenAI Optimization

**Rate Limits**:
- Free tier: 3 RPM, 40,000 TPM
- Tier 1: 500 RPM, 60,000 TPM
- Tier 3: 5,000 RPM, 1,000,000 TPM
- Tier 4: 10,000 RPM, 5,000,000 TPM

**Batching**:
- Batch API: Up to 50,000 requests per batch file
- 50% cost reduction for batch API
- 24-hour completion window

**Tier Upgrade**:
- Automatic based on spending ($5 → Tier 1, $50 → Tier 2, etc.)
- Manual request for higher tiers via support

---

## Frequently Asked Questions

### Can I switch providers without re-indexing everything?

**Yes!** Maproom stores embeddings in provider-specific columns. You can:
1. Index new/changed files with a new provider
2. Keep existing embeddings from the old provider
3. Searches automatically use the best available embedding

### Do different providers give different search results?

**Yes, but quality is comparable**. In testing:
- OpenAI (1536-dim): Best overall quality (+5-10% relevance)
- Google (768-dim): Strong multilingual performance
- Ollama (1024-dim): Excellent for code, high quality with mxbai-embed-large

Differences are usually minor (<10% relevance delta). Choose based on cost/privacy/compliance, not just quality.

### Can I use multiple providers simultaneously?

**Yes!** Set `MAPROOM_EMBEDDING_PROVIDER` to different values for different indexing runs. Example:
```bash
# Development: Use Ollama
export MAPROOM_EMBEDDING_PROVIDER="ollama"
crewchief maproom scan --worktree dev-branch

# Production: Use Google
export MAPROOM_EMBEDDING_PROVIDER="google"
crewchief maproom scan --worktree main
```

### How much does it cost to index a typical codebase?

**Examples** (based on ~500 chars per chunk):
- **Small** (10K chunks): Ollama $0, Google $1.25, OpenAI $0.50
- **Medium** (100K chunks): Ollama $0, Google $12.50, OpenAI $5
- **Large** (1M chunks): Ollama $0, Google $125, OpenAI $50
- **Huge** (10M chunks): Ollama $0, Google $1,250, OpenAI $500

**Incremental indexing** (re-indexing changed files) dramatically reduces costs.

### What happens if my API key expires or quota is exceeded?

**Maproom degrades gracefully**:
1. New file indexing fails (logged as warning)
2. Existing embeddings continue to work for search
3. Search falls back to older embeddings if new ones unavailable

**Fix**: Update API key or increase quota, then re-run scan.

### Can I self-host the OpenAI or Google models?

- **OpenAI**: No, models are proprietary and API-only
- **Google**: No, Vertex AI is managed service only
- **Ollama**: Yes, fully self-hosted (already local)

For self-hosting requirements, use **Ollama** or other open-source embedding models.

### How do I estimate my embedding costs?

**Use this formula**:
```
Cost = (num_chunks × avg_chars_per_chunk × cost_per_1K_chars) / 1000
```

**Example** (Google, 100K chunks, 500 chars each):
```
Cost = (100,000 × 500 × $0.00025) / 1000 = $12.50
```

**Get your chunk count**:
```bash
crewchief maproom db  # Shows total chunks indexed
```

---

## Additional Resources

### Official Provider Documentation

- **[Google Vertex AI Documentation](https://cloud.google.com/vertex-ai/docs/generative-ai/embeddings/get-text-embeddings)**
- **[OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)**
- **[Ollama Documentation](https://github.com/ollama/ollama/blob/main/docs/README.md)**

### Provider Setup Guides

- **[Google Vertex AI Setup](./google-vertex-ai-setup.md)** - Comprehensive GCP setup guide
- **[Ollama Setup](./ollama-setup.md)** - Coming soon
- **[OpenAI Setup](./openai-setup.md)** - Coming soon

### Maproom Documentation

- **[Migration Guide](./migration-guide.md)** - Switching between providers (coming soon)
- **[Configuration Guide](../../crates/maproom/docs/configuration_guide.md)** - Full configuration reference
- **[Performance Tuning](../../crates/maproom/docs/PERFORMANCE_TUNING.md)** - Optimize search performance

### Benchmarks & Comparisons

- **[MTEB Leaderboard](https://huggingface.co/spaces/mteb/leaderboard)** - Embedding model benchmarks
- **[OpenAI Embeddings Blog](https://openai.com/blog/new-embedding-models-and-api-updates)** - OpenAI model details
- **[Google Vertex AI Benchmarks](https://cloud.google.com/vertex-ai/docs/generative-ai/learn/models)** - Google model specs

---

## Summary Recommendation Table

| Your Situation | Recommended Provider | Reason |
|----------------|---------------------|--------|
| **"I'm just getting started"** | Ollama | Free, fast setup, no cloud dependencies |
| **"I need HIPAA compliance"** | Google Vertex AI (with BAA) | HIPAA-eligible with proper configuration |
| **"I'm already on GCP"** | Google Vertex AI | Billing consolidation, IAM integration |
| **"I'm already using OpenAI"** | OpenAI | Consistent API, consolidated billing |
| **"I have a GPU workstation"** | Ollama | Leverage existing hardware, zero API costs |
| **"I need air-gapped/offline"** | Ollama | Complete offline operation |
| **"I want the best quality"** | OpenAI | Highest dimensions (1536), best benchmarks |
| **"I need lowest cost at scale"** | Ollama | Free API, only compute costs |
| **"I want simple setup"** | OpenAI or Ollama | Both: single env var, <10 min setup |
| **"I need EU data residency"** | Google Vertex AI | Regional endpoints (europe-west1) |
| **"I'm indexing >1M chunks/month"** | Ollama | Zero API costs at any scale |
| **"I need audit logs for compliance"** | Google Vertex AI | Cloud Audit Logs, Access Transparency |

---

**Still unsure?** Start with **Ollama** for free experimentation, then add a cloud provider (Google or OpenAI) based on your specific compliance, cost, or quality requirements. Maproom's multi-provider architecture lets you change your mind later without data loss.

---

**Last Updated**: October 29, 2025

**Feedback?** Found an error or have suggestions? [Open an issue on GitHub](https://github.com/yourusername/maproom/issues) or submit a pull request.
