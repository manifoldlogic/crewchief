# OpenAI Setup Guide

**Last Updated**: October 2025

## Overview

This guide walks you through setting up OpenAI embeddings for Maproom's semantic code search. OpenAI provides high-quality, reliable embeddings through their `text-embedding-3-small` model with simple API-key based authentication.

### What is OpenAI Embeddings?

[OpenAI Embeddings API](https://platform.openai.com/docs/guides/embeddings) converts text into dense vector representations (embeddings) that capture semantic meaning. For Maproom, we use `text-embedding-3-small` to generate 1536-dimensional embeddings for semantic code search.

### When to Use OpenAI

**Choose OpenAI when you need:**
- **High-dimensional embeddings**: 1536 dimensions (vs 768 for Google/Ollama) for potentially better search precision
- **Simple setup**: API key authentication, no service accounts or GCP projects
- **Existing OpenAI integration**: Already using OpenAI for other services (ChatGPT, GPT-4, etc.)
- **Reliable API**: Production-grade SLA with global availability
- **Cost-effective**: ~$0.00002 per 1K tokens (~$0.00003 per 1K chars), cheaper than Google Vertex AI

**Consider alternatives if:**
- **Ollama**: You want free, local embeddings without any API costs
- **Google Vertex AI**: You need GCP integration, data residency, or specific compliance requirements
- **Privacy concerns**: Your codebase cannot be sent to external APIs

### Cost Implications

**Pricing**: Approximately **$0.00002 per 1,000 tokens** (~**$0.00003 per 1,000 characters**)

**Example costs for embedding a codebase:**
- **Small codebase** (10K chunks, avg 500 chars): ~$0.15
- **Medium codebase** (100K chunks, avg 500 chars): ~$1.50
- **Large codebase** (1M chunks, avg 500 chars): ~$15.00

**Free tier**: New OpenAI accounts receive **$5 in free credits** for the first 3 months.

**Cost comparison:**
- **OpenAI**: ~$0.00003 / 1K chars
- **Google Vertex AI**: ~$0.00025 / 1K chars (8x more expensive)
- **Ollama**: Free (hardware costs only)

**Cost optimization tips:**
- Cache embeddings to avoid re-generating for unchanged code
- Use batch processing (Maproom does this automatically)
- Set up billing alerts in OpenAI dashboard
- Monitor usage at [platform.openai.com/usage](https://platform.openai.com/usage)

---

## Prerequisites

Before starting, ensure you have:

- ✅ **OpenAI account** - [Sign up here](https://platform.openai.com/signup)
- ✅ **Payment method on file** - [Add billing details](https://platform.openai.com/account/billing)
- ✅ **Active credits or approved billing** - Required even for free tier usage

**Note**: OpenAI requires a valid payment method even during free trial period.

---

## Step 1: Create OpenAI Account

If you don't already have an OpenAI account:

1. Go to [https://platform.openai.com/signup](https://platform.openai.com/signup)
2. Sign up with email, Google, or Microsoft account
3. Verify your email address
4. Complete account setup

**Note**: OpenAI Platform account is separate from ChatGPT account. You need a Platform account for API access.

---

## Step 2: Set Up Billing

OpenAI requires billing information to use the Embeddings API.

### Add Payment Method

1. Navigate to [Billing Settings](https://platform.openai.com/account/billing)
   - **Navigation path**: `OpenAI Dashboard > Settings > Billing`

2. Click **"Add payment details"**

3. Enter your credit card information

4. Set billing preferences:
   - **Automatic recharge**: Recommended for production use
   - **Recharge threshold**: Minimum balance before auto-recharge (e.g., $5)
   - **Recharge amount**: How much to add when threshold is reached (e.g., $10)

### Set Budget Alerts (Recommended)

Protect against unexpected charges:

1. In **Billing Settings**, click **"Usage limits"**

2. Set a **hard limit** (spending cap):
   ```
   Hard limit: $50/month
   ```
   API requests will be rejected when limit is reached.

3. Set a **soft limit** (email notification):
   ```
   Soft limit: $10/month
   ```
   You'll receive an email when 80% of limit is reached.

**Recommended limits for development:**
- Small projects: $5-10/month
- Medium projects: $20-50/month
- Large projects: $100+/month

---

## Step 3: Generate API Key

API keys authenticate your requests to OpenAI's API.

### Create API Key

1. Navigate to [API Keys page](https://platform.openai.com/api-keys)
   - **Navigation path**: `OpenAI Dashboard > API keys`
   - Direct link: [https://platform.openai.com/api-keys](https://platform.openai.com/api-keys)

2. Click **"+ Create new secret key"**

3. **Configure key settings**:
   - **Name**: `CrewChief Maproom` (descriptive name for tracking)
   - **Permissions**: `Read-only` (embeddings don't need write access)
     - **Note**: If read-only isn't available, use default permissions
   - **Project**: Select your project (or use default)

4. Click **"Create secret key"**

5. **Copy and save immediately**:
   ```
   sk-proj-abc123...xyz789
   ```

   ⚠️ **Warning**: This key is shown only once. Save it securely immediately.

### Secure Storage Best Practices

**❌ NEVER:**
- Commit API keys to version control (git)
- Share keys in Slack, email, or public forums
- Store keys in plaintext files in your project directory
- Use keys in frontend code or browser-accessible locations

**✅ ALWAYS:**
- Store keys in environment variables
- Use `.env` files (and add to `.gitignore`)
- Set file permissions to 600 (owner read/write only)
- Rotate keys every 90 days
- Use separate keys for development and production

---

## Step 4: Configure Environment Variables

Store your API key securely using environment variables.

### macOS / Linux

**Option A: Shell configuration file (persistent)**

Add to `~/.bashrc`, `~/.zshrc`, or `~/.bash_profile`:

```bash
# Open your shell config
nano ~/.bashrc  # or ~/.zshrc

# Add these lines:
export OPENAI_API_KEY="sk-proj-abc123...xyz789"
export EMBEDDING_PROVIDER="openai"

# Save and reload
source ~/.bashrc  # or source ~/.zshrc
```

**Option B: Project-specific `.env` file**

Create `.env` in your project root:

```bash
# .env file
OPENAI_API_KEY=sk-proj-abc123...xyz789
EMBEDDING_PROVIDER=openai
```

Add `.env` to `.gitignore`:

```bash
echo ".env" >> .gitignore
```

Load environment variables:

```bash
# Using direnv (recommended)
direnv allow

# Or manually
export $(cat .env | xargs)

# Or with Node.js
npm install dotenv
# Then in your code: require('dotenv').config()
```

**Option C: Session-specific (temporary)**

```bash
# Set for current terminal session only
export OPENAI_API_KEY="sk-proj-abc123...xyz789"
export EMBEDDING_PROVIDER="openai"
```

### Windows (PowerShell)

**Option A: User environment variable (persistent)**

```powershell
# Set permanently for current user
[System.Environment]::SetEnvironmentVariable('OPENAI_API_KEY', 'sk-proj-abc123...xyz789', 'User')
[System.Environment]::SetEnvironmentVariable('EMBEDDING_PROVIDER', 'openai', 'User')

# Restart PowerShell for changes to take effect
```

**Option B: Session-specific (temporary)**

```powershell
# Set for current PowerShell session
$env:OPENAI_API_KEY = "sk-proj-abc123...xyz789"
$env:EMBEDDING_PROVIDER = "openai"
```

### Windows (WSL2)

Follow the macOS/Linux instructions above inside your WSL2 terminal.

---

## Step 5: Verify Configuration

Test that your API key is configured correctly before indexing.

### Check Environment Variables

```bash
# Verify variables are set
echo $OPENAI_API_KEY
# Should output: sk-proj-abc123...xyz789

echo $EMBEDDING_PROVIDER
# Should output: openai
```

### Test API Access

```bash
# Test API key with a simple request
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

**Expected output**: JSON response with list of available models:
```json
{
  "object": "list",
  "data": [
    {
      "id": "text-embedding-3-small",
      "object": "model",
      ...
    },
    ...
  ]
}
```

**Error responses:**

- **401 Unauthorized**: Invalid API key
  ```json
  {
    "error": {
      "message": "Incorrect API key provided",
      "type": "invalid_request_error"
    }
  }
  ```
  **Fix**: Verify you copied the full key correctly

- **403 Forbidden**: Billing issue
  ```json
  {
    "error": {
      "message": "You exceeded your current quota",
      "type": "insufficient_quota"
    }
  }
  ```
  **Fix**: Add payment method or check billing status

---

## Step 6: Test Embedding Generation

Now test that Maproom can generate embeddings using OpenAI.

### Run a Test Scan

```bash
# Dry run to verify configuration
crewchief maproom scan --generate-embeddings --dry-run
```

**Expected output:**
```
✓ Using embedding provider: openai (1536 dimensions)
✓ Model: text-embedding-3-small
✓ Scanning repository...
✓ Would index 1,234 code chunks
✓ Estimated cost: ~$0.18
✓ Dry run complete - no changes made
```

### Generate Real Embeddings

```bash
# Index your codebase with embeddings
crewchief maproom scan --generate-embeddings
```

**Expected output:**
```
✓ Using embedding provider: openai (1536 dimensions)
✓ Model: text-embedding-3-small
✓ Scanning repository...
✓ Found 1,234 code chunks
✓ Generating embeddings...
  [████████████████████████████████████████] 1234/1234
✓ Generated embeddings for 1,234 chunks in 45s
✓ Estimated cost: ~$0.18
✓ Index updated successfully
```

**Note**: OpenAI API is fast - expect ~20-50 chunks/second depending on network latency.

### Search Test

```bash
# Test semantic search
crewchief maproom search "authentication flow"
```

**Expected output:**
```
Found 5 results:

1. auth/middleware.ts:42-68 (score: 0.91)
   Authentication middleware for Express...

2. auth/jwt.service.ts:12-45 (score: 0.88)
   JWT token validation and generation...
```

---

## Troubleshooting

### "Invalid API key provided"

**Symptoms:**
```
Error: Incorrect API key provided: sk-proj-abc1******xyz9
```

**Causes:**
- API key was copied incorrectly (missing characters)
- API key has been revoked or expired
- Using old key format (keys should start with `sk-proj-`)

**Solutions:**

1. **Regenerate API key**:
   - Go to [https://platform.openai.com/api-keys](https://platform.openai.com/api-keys)
   - Click "Create new secret key"
   - Copy the full key (it's long - usually 40-60 characters)
   - Update your environment variable

2. **Verify key in environment**:
   ```bash
   echo $OPENAI_API_KEY
   # Make sure the full key is printed, no truncation
   ```

3. **Check for whitespace**:
   ```bash
   # Trim whitespace
   export OPENAI_API_KEY="$(echo $OPENAI_API_KEY | xargs)"
   ```

---

### "Rate limit exceeded"

**Symptoms:**
```
Error: Rate limit reached for requests
Error: Rate limit reached for tokens per min (TPM)
```

**Causes:**
- Exceeded requests per minute (RPM) limit
- Exceeded tokens per minute (TPM) limit
- Free tier limits are lower than paid tiers

**Solutions:**

1. **Wait 60 seconds** and retry:
   ```bash
   # OpenAI rate limits reset every minute
   sleep 60
   crewchief maproom scan --generate-embeddings
   ```

2. **Check your rate limits**:
   - Go to [Settings > Limits](https://platform.openai.com/account/limits)
   - Free tier: ~60 RPM, 150,000 TPM
   - Tier 1 (after $5 spent): ~500 RPM, 1M TPM
   - Tier 2 (after $50 spent): ~3,000 RPM, 5M TPM

3. **Upgrade tier**:
   - Higher usage tier automatically unlocked after spending thresholds
   - See [Rate Limits Documentation](https://platform.openai.com/docs/guides/rate-limits)

4. **Use batch processing** (Maproom handles this automatically):
   - Maproom batches requests to stay within rate limits
   - For very large codebases, indexing may take longer on free tier

---

### "You exceeded your current quota"

**Symptoms:**
```
Error: You exceeded your current quota, please check your plan and billing details
```

**Causes:**
- Insufficient credits (free trial exhausted)
- Billing issue (declined payment)
- Hard spending limit reached

**Solutions:**

1. **Check billing status**:
   - Go to [Billing Overview](https://platform.openai.com/account/billing/overview)
   - Verify payment method is valid
   - Check current balance and usage

2. **Add credits**:
   - Click "Add to credit balance"
   - Add at least $5-10 for development use

3. **Increase spending limit**:
   - Go to [Usage Limits](https://platform.openai.com/account/limits)
   - Increase hard limit if it's too low
   - Note: Limits protect against unexpected charges

4. **Wait for auto-recharge** (if configured):
   - Check auto-recharge settings
   - Ensure recharge threshold and amount are appropriate

---

### "Connection timeout" or "API unreachable"

**Symptoms:**
```
Error: Request to OpenAI API timed out
Error: Could not connect to api.openai.com
```

**Causes:**
- Network connectivity issues
- Firewall blocking OpenAI API
- Corporate proxy blocking requests

**Solutions:**

1. **Check internet connection**:
   ```bash
   ping api.openai.com
   curl https://api.openai.com/v1/models -I
   ```

2. **Check firewall rules**:
   ```bash
   # Ensure HTTPS (port 443) is allowed
   # OpenAI API: api.openai.com
   ```

3. **Configure proxy** (if behind corporate firewall):
   ```bash
   export HTTPS_PROXY=http://proxy.company.com:8080
   export HTTP_PROXY=http://proxy.company.com:8080
   ```

4. **Try different network**:
   - Test on mobile hotspot to rule out network issues

---

### Slow Embedding Generation

**Symptoms:**
- Embedding generation slower than expected
- Taking several minutes for small codebases

**Causes:**
- Network latency to OpenAI API
- Rate limiting (throttling)
- Large batch sizes

**Solutions:**

1. **Check network latency**:
   ```bash
   # Measure API response time
   time curl https://api.openai.com/v1/models \
     -H "Authorization: Bearer $OPENAI_API_KEY"
   ```

2. **Check rate limits**:
   - High latency might indicate soft rate limiting
   - Upgrade to higher tier for more RPM/TPM

3. **Use incremental indexing**:
   ```bash
   # Only process changed files
   crewchief maproom scan --incremental --generate-embeddings
   ```

**Expected performance:**
- Network latency: ~100-300ms per request
- Throughput: ~20-50 chunks/second (depending on batch size and tier)
- Medium codebase (50K chunks): ~15-30 minutes

---

## Security Best Practices

### API Key Management

**✅ Do:**
- Store keys in environment variables, not code
- Use separate keys for development and production
- Rotate keys every 90 days
- Set file permissions to 600 for config files: `chmod 600 .env`
- Use read-only keys when possible
- Revoke unused keys immediately

**❌ Don't:**
- Commit keys to git repositories
- Share keys via email, Slack, or unsecured channels
- Use production keys in development
- Store keys in plaintext in shared locations
- Use the same key across multiple projects

### Key Rotation

Rotate API keys regularly to minimize security risk:

```bash
# 1. Generate new key
# Go to https://platform.openai.com/api-keys
# Click "Create new secret key"

# 2. Update environment variable
export OPENAI_API_KEY="new-key-here"

# 3. Test with Maproom
crewchief maproom scan --dry-run

# 4. If successful, revoke old key
# Go to https://platform.openai.com/api-keys
# Click trash icon next to old key
```

**Recommended rotation schedule:**
- Development keys: Every 90 days
- Production keys: Every 30-60 days
- Compromised keys: Immediately

### Monitoring & Alerts

**Set up monitoring** to detect anomalous usage:

1. **Usage alerts**:
   - Go to [Usage Limits](https://platform.openai.com/account/limits)
   - Set soft limit (email notification)
   - Set hard limit (stop requests)

2. **Billing alerts**:
   - Go to [Billing Settings](https://platform.openai.com/account/billing)
   - Enable email notifications for:
     - Daily usage reports
     - Spending threshold alerts
     - Failed payment attempts

3. **Review usage regularly**:
   - Check [Usage Dashboard](https://platform.openai.com/usage) weekly
   - Look for unexpected spikes
   - Verify usage matches expected patterns

### Data Privacy

**What data is sent to OpenAI:**
- Code chunks (text content only)
- No file paths, commit history, or repository metadata

**OpenAI's data policies:**
- **API data is NOT used for training** (as of March 2023 policy update)
- Data is encrypted in transit (HTTPS) and at rest
- Data retention: 30 days, then deleted
- See [OpenAI Privacy Policy](https://openai.com/policies/privacy-policy)

**For sensitive codebases:**
- Consider **Ollama** (fully local, no data leaves your machine)
- Use **Google Vertex AI** with data residency controls
- Review OpenAI's enterprise agreements for additional guarantees

---

## Cost Management

### Understanding Pricing

**OpenAI Embeddings Pricing (as of October 2025):**
- `text-embedding-3-small`: **$0.00002 per 1,000 tokens**
- `text-embedding-3-large`: **$0.00013 per 1,000 tokens** (higher dimensions, more expensive)

**Token estimation:**
- ~1 token = 4 characters of English text
- ~1,000 tokens = 750 words or ~500 lines of code
- Average code chunk: ~500 characters = ~125 tokens

**Cost calculator:**
```
Cost = (Total characters / 1000) × $0.00003

Example:
- 10,000 chunks × 500 chars = 5M characters
- Cost = (5,000,000 / 1000) × $0.00003 = $0.15
```

### Cost Estimation Before Indexing

```bash
# Dry run shows estimated cost
crewchief maproom scan --generate-embeddings --dry-run

# Output:
# Would index 1,234 chunks
# Estimated cost: ~$0.18
```

### Cost Optimization Strategies

1. **Use incremental indexing**:
   ```bash
   # Only re-index changed files
   crewchief maproom scan --incremental --generate-embeddings
   ```
   **Savings**: 90-95% after initial index

2. **Exclude unnecessary files**:
   ```
   # .maproomignore
   node_modules/
   dist/
   *.test.ts
   *.spec.ts
   coverage/
   ```
   **Savings**: 20-50% depending on project structure

3. **Cache embeddings** (automatic):
   - Maproom caches embeddings in database
   - Unchanged files are never re-embedded
   **Savings**: ~95% on subsequent scans

4. **Use batch processing** (automatic):
   - Maproom batches API requests
   - Reduces overhead and API calls
   **Savings**: 10-20% on request overhead

5. **Develop with Ollama, deploy with OpenAI**:
   ```bash
   # Development (free)
   export EMBEDDING_PROVIDER=ollama
   crewchief maproom scan --generate-embeddings

   # Production (paid)
   export EMBEDDING_PROVIDER=openai
   crewchief maproom scan --generate-embeddings
   ```
   **Savings**: 100% on development costs

### Monitor Usage

Track spending to avoid surprises:

```bash
# Check current usage
open https://platform.openai.com/usage

# View billing details
open https://platform.openai.com/account/billing/overview
```

**Set up alerts:**
- Daily usage emails
- Spending threshold notifications (e.g., alert at $5, $10, $20)
- Hard spending limits to prevent overages

---

## Advanced Configuration

### Using Different Models

OpenAI offers multiple embedding models:

```bash
# Default: text-embedding-3-small (1536 dimensions, $0.00002/1K tokens)
export OPENAI_MODEL=text-embedding-3-small

# Larger: text-embedding-3-large (3072 dimensions, $0.00013/1K tokens)
export OPENAI_MODEL=text-embedding-3-large
```

**Model comparison:**

| Model | Dimensions | Cost per 1K tokens | Use Case |
|-------|-----------|-------------------|----------|
| `text-embedding-3-small` | 1536 | $0.00002 | General purpose (default) |
| `text-embedding-3-large` | 3072 | $0.00013 | Maximum quality (6.5x more expensive) |
| `text-embedding-ada-002` | 1536 | $0.00010 | Legacy model (not recommended) |

**Recommendation**: Use `text-embedding-3-small` for most use cases. Quality is excellent and cost is minimal.

### Custom Batch Sizes

Control API request batching:

```bash
# Larger batches = fewer API calls but longer individual requests
export OPENAI_BATCH_SIZE=100  # Default: 50-100

# Smaller batches = more API calls but faster feedback
export OPENAI_BATCH_SIZE=20
```

**Tradeoffs:**
- **Larger batches**: Fewer API calls, lower overhead, but less granular progress
- **Smaller batches**: More API calls, higher overhead, but better progress visibility

### Timeout Configuration

Adjust timeouts for slow networks:

```bash
# Increase timeout for slow networks (seconds)
export OPENAI_TIMEOUT=60  # Default: 30

# Increase retry attempts
export OPENAI_MAX_RETRIES=5  # Default: 3
```

---

## Comparison with Other Providers

### OpenAI vs Google Vertex AI

| Feature | OpenAI | Google Vertex AI |
|---------|--------|------------------|
| **Cost** | ~$0.00003 / 1K chars | ~$0.00025 / 1K chars |
| **Dimensions** | 1536 | 768 |
| **Setup** | Easy (5 min) | Medium (15 min) |
| **Billing** | Credit card | GCP account |
| **SLA** | ✅ Yes | ✅ Yes |
| **Data residency** | Limited | ✅ Multi-region |
| **Compliance** | SOC 2, ISO | SOC 2, ISO, FedRAMP |

**When to choose OpenAI over Google:**
- Simpler setup (no GCP projects)
- Lower cost (~8x cheaper)
- Higher dimensional embeddings (1536D vs 768D)
- Already using OpenAI services

### OpenAI vs Ollama

| Feature | OpenAI | Ollama |
|---------|--------|--------|
| **Cost** | ~$0.00003 / 1K chars | Free |
| **Dimensions** | 1536 | 768 |
| **Setup** | Easy (5 min) | Easy (5 min) |
| **Privacy** | Cloud | ✅ Local only |
| **Offline** | ❌ No | ✅ Yes |
| **Speed** | Fast (API) | Fast (GPU) / Slow (CPU) |
| **SLA** | ✅ Yes | ❌ No |

**When to choose OpenAI over Ollama:**
- No local hardware for running models
- Need production SLA
- Want higher dimensional embeddings
- Don't want to manage infrastructure

**See detailed comparison**: [Provider Comparison Guide](./comparison.md)

---

## Quick Reference

### Common Commands

```bash
# Configuration
export OPENAI_API_KEY="sk-proj-abc123...xyz789"
export EMBEDDING_PROVIDER="openai"

# Verify setup
echo $OPENAI_API_KEY
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Indexing
crewchief maproom scan --generate-embeddings --dry-run   # Test
crewchief maproom scan --generate-embeddings              # Full index
crewchief maproom scan --incremental --generate-embeddings # Changed files only

# Search
crewchief maproom search "authentication flow"

# Cost monitoring
open https://platform.openai.com/usage
```

### Environment Variables

```bash
# Required
OPENAI_API_KEY=sk-proj-abc123...xyz789
EMBEDDING_PROVIDER=openai

# Optional
OPENAI_MODEL=text-embedding-3-small    # Default model
OPENAI_BATCH_SIZE=100                  # Batch size for API requests
OPENAI_TIMEOUT=30                      # API timeout (seconds)
OPENAI_MAX_RETRIES=3                   # Retry attempts on failure
```

---

## Additional Resources

### Official Documentation

- **[OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)** - Complete embeddings documentation
- **[OpenAI API Reference](https://platform.openai.com/docs/api-reference/embeddings)** - API endpoint details
- **[Rate Limits](https://platform.openai.com/docs/guides/rate-limits)** - Rate limit tiers and policies
- **[Pricing](https://openai.com/api/pricing/)** - Current pricing information

### Maproom Documentation

- **[Provider Comparison Guide](./comparison.md)** - Compare all providers
- **[Migration Guide](./migration-guide.md)** - Switch between providers
- **[Configuration Guide](../../crates/maproom/docs/configuration_guide.md)** - Full Maproom configuration
- **[Performance Tuning](../../crates/maproom/docs/PERFORMANCE_TUNING.md)** - Optimize search performance

### Community

- **[OpenAI Community Forum](https://community.openai.com/)** - Official community
- **[Maproom Discussions](https://github.com/yourusername/maproom/discussions)** - Ask questions, share tips
- **[GitHub Issues](https://github.com/yourusername/maproom/issues)** - Report bugs

---

## Need Help?

**API key issues?** Check the [Troubleshooting section](#troubleshooting)

**Rate limits?** See [Rate limit troubleshooting](#rate-limit-exceeded)

**Cost concerns?** Review [Cost Management](#cost-management) strategies

**Switching providers?** Read the [Migration Guide](./migration-guide.md)

**Have feedback?** Open an issue or discussion on GitHub

---

**Last Updated**: October 2025
