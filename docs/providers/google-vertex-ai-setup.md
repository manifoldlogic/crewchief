# Google Vertex AI Setup Guide

**Last Updated**: October 2025

## Overview

This guide walks you through setting up Google Vertex AI embeddings for Maproom's semantic code search. Vertex AI provides high-quality, production-grade embeddings through Google's `text-embedding-gecko@003` model.

### What is Vertex AI?

[Google Vertex AI](https://cloud.google.com/vertex-ai) is Google Cloud's unified machine learning platform. For Maproom, we use its text embedding models to generate 768-dimensional vector embeddings for semantic code search.

### When to Use Google Vertex AI

**Choose Google Vertex AI when you need:**
- **Production-grade reliability**: SLA-backed uptime and performance
- **Enterprise compliance**: Data residency, audit logging, SOC 2/ISO compliance
- **High-quality embeddings**: Google's latest embedding models (768 dimensions)
- **Global scalability**: Multi-region deployment options
- **Cost predictability**: Pay-per-use pricing with quotas

**Consider alternatives if:**
- **Ollama**: You want free, local embeddings without API costs
- **OpenAI**: You prefer OpenAI's embedding models (1536 dimensions with text-embedding-3-small)
- **Cohere**: You need multilingual support or specific Cohere features

### Cost Implications

**Pricing**: Approximately **$0.00025 per 1,000 characters** (text-embedding-gecko@003)

**Example costs for embedding a codebase:**
- **Small codebase** (10K chunks, avg 500 chars): ~$1.25
- **Medium codebase** (100K chunks, avg 500 chars): ~$12.50
- **Large codebase** (1M chunks, avg 500 chars): ~$125.00

**Free tier**: Check [current Vertex AI free tier limits](https://cloud.google.com/vertex-ai/pricing#free-tier) - typically includes limited free requests per month.

**Cost optimization tips:**
- Cache embeddings to avoid re-generating for unchanged code
- Use batch processing (Maproom does this automatically)
- Monitor usage with GCP billing alerts

---

## Prerequisites

Before starting, ensure you have:

- ✅ **Google Cloud Platform account** - [Create one here](https://cloud.google.com/free)
- ✅ **Active GCP project with billing enabled** - [Create a project](https://console.cloud.google.com/projectcreate)
- ✅ **gcloud CLI installed** (optional but recommended) - [Install guide](https://cloud.google.com/sdk/docs/install)

---

## Step 1: Enable Vertex AI API

The Vertex AI API must be enabled for your GCP project before you can use it.

### Using GCP Console

1. Navigate to the [Vertex AI page](https://console.cloud.google.com/vertex-ai) in GCP Console
2. Select your project from the dropdown at the top
3. Click **"Enable API"** button
4. Wait for activation (typically 1-2 minutes)
5. You'll see the Vertex AI dashboard once enabled

**Navigation path**: `GCP Console > More Products > Artificial Intelligence > Vertex AI`

### Using gcloud CLI

```bash
# Replace YOUR_PROJECT_ID with your actual GCP project ID
gcloud services enable aiplatform.googleapis.com --project=YOUR_PROJECT_ID

# Verify the API is enabled
gcloud services list --enabled --project=YOUR_PROJECT_ID | grep aiplatform
```

**Expected output**: `aiplatform.googleapis.com  Vertex AI API`

---

## Step 2: Create Service Account

A service account is a special type of Google account that represents an application rather than a person. Maproom uses this to authenticate with Vertex AI.

### Using GCP Console

1. Navigate to **IAM & Admin > Service Accounts**
   - **Navigation path**: `GCP Console > IAM & Admin > Service Accounts`
   - Direct link: [Service Accounts Console](https://console.cloud.google.com/iam-admin/serviceaccounts)

2. Click **"+ CREATE SERVICE ACCOUNT"** at the top

3. **Service account details**:
   - **Service account name**: `maproom-embeddings`
   - **Service account ID**: `maproom-embeddings` (auto-filled)
   - **Description**: `Service account for Maproom code embedding generation`

4. Click **"CREATE AND CONTINUE"**

5. **Skip** the "Grant this service account access to project" step for now (we'll do this in Step 3)

6. **Skip** the "Grant users access to this service account" step

7. Click **"DONE"**

### Using gcloud CLI

```bash
# Create the service account
gcloud iam service-accounts create maproom-embeddings \
  --display-name="Maproom Embeddings" \
  --description="Service account for Maproom code embedding generation" \
  --project=YOUR_PROJECT_ID

# Verify creation
gcloud iam service-accounts list --project=YOUR_PROJECT_ID | grep maproom-embeddings
```

**Expected output**: Shows the service account email `maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com`

---

## Step 3: Grant IAM Permissions

Following the **principle of least privilege**, we grant only the minimum permissions needed for Maproom to function.

### Recommended Role: `roles/aiplatform.user`

This role provides:
- ✅ Permission to call Vertex AI prediction endpoints
- ✅ Read access to AI Platform models
- ❌ **No** permission to modify or delete resources
- ❌ **No** permission to manage service accounts
- ❌ **No** billing or project admin rights

**Why not broader roles?**
- `roles/editor` or `roles/owner` grant far more permissions than needed
- Compromised credentials would have limited blast radius
- Follows security best practices for service accounts

### Using GCP Console

1. Navigate to **IAM & Admin > IAM**
   - **Navigation path**: `GCP Console > IAM & Admin > IAM`
   - Direct link: [IAM Console](https://console.cloud.google.com/iam-admin/iam)

2. Click **"+ GRANT ACCESS"** at the top

3. **Add principals**:
   - **New principals**: `maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com`
   - **Select a role**: Type "Vertex AI User" and select `Vertex AI User`

4. Click **"SAVE"**

5. Verify the role appears in the IAM list for your service account

### Using gcloud CLI

```bash
# Grant the Vertex AI User role to your service account
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/aiplatform.user"

# Verify the role assignment
gcloud projects get-iam-policy YOUR_PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com"
```

**Expected output**: Shows the role binding with `role: roles/aiplatform.user`

---

## Step 4: Create and Download Service Account Key

⚠️ **Security Warning**: Service account keys are powerful credentials. Anyone with this file can access your GCP resources as this service account. Store securely and never commit to version control.

### Using GCP Console

1. Navigate to **IAM & Admin > Service Accounts**
   - Direct link: [Service Accounts Console](https://console.cloud.google.com/iam-admin/serviceaccounts)

2. Click on the **`maproom-embeddings`** service account in the list

3. Go to the **"KEYS"** tab

4. Click **"ADD KEY" > "Create new key"**

5. **Key type**: Select **"JSON"**

6. Click **"CREATE"**

7. The JSON key file will automatically download to your computer

8. **Secure the file**:
   ```bash
   # Create a secure directory for GCP credentials
   mkdir -p ~/.config/gcp

   # Move the downloaded key (adjust path as needed)
   mv ~/Downloads/YOUR_PROJECT_ID-*.json ~/.config/gcp/maproom-sa-key.json

   # Set secure permissions (owner read/write only)
   chmod 600 ~/.config/gcp/maproom-sa-key.json

   # Verify permissions
   ls -la ~/.config/gcp/maproom-sa-key.json
   ```

   **Expected output**: `-rw------- 1 youruser youruser 2345 Oct 28 10:00 maproom-sa-key.json`

### Using gcloud CLI

```bash
# Create directory for credentials
mkdir -p ~/.config/gcp

# Generate and download the key
gcloud iam service-accounts keys create ~/.config/gcp/maproom-sa-key.json \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --project=YOUR_PROJECT_ID

# Set secure permissions
chmod 600 ~/.config/gcp/maproom-sa-key.json

# Verify the key was created
ls -la ~/.config/gcp/maproom-sa-key.json
```

### .gitignore Protection

**CRITICAL**: Ensure service account keys are never committed to version control.

Add to your `.gitignore`:
```gitignore
# Google Cloud service account keys
*.json
*-key.json
google-credentials.json
maproom-sa-key.json

# GCP config directories
.config/gcp/
```

---

## Step 5: Configure Environment Variables

Maproom uses environment variables to locate and authenticate with Google Vertex AI.

### Required Environment Variables

```bash
# Required: Your GCP project ID
export GOOGLE_PROJECT_ID="your-project-id"

# Required: Path to service account key JSON file
export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key.json"

# Required: Set Google as the embedding provider
export MAPROOM_EMBEDDING_PROVIDER="google"

# Optional: Regional endpoint (default: us-central1)
export GOOGLE_VERTEX_REGION="us-central1"
```

### Persistent Configuration

Add these to your shell profile to persist across sessions (example shows Bash `~/.bashrc`; adjust if your shell uses a different startup file):
```bash
echo 'export GOOGLE_PROJECT_ID="your-project-id"' >> ~/.bashrc
echo 'export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key.json"' >> ~/.bashrc
echo 'export MAPROOM_EMBEDDING_PROVIDER="google"' >> ~/.bashrc
echo 'export GOOGLE_VERTEX_REGION="us-central1"' >> ~/.bashrc

# Reload shell configuration
source ~/.bashrc
```

### Project-Specific Configuration

For project-specific configuration, create a `.env` file in your project root:

```bash
# .env file
GOOGLE_PROJECT_ID=your-project-id
GOOGLE_APPLICATION_CREDENTIALS=/home/youruser/.config/gcp/maproom-sa-key.json
MAPROOM_EMBEDDING_PROVIDER=google
GOOGLE_VERTEX_REGION=us-central1
```

**Load with direnv** (recommended):
```bash
# Install direnv: https://direnv.net/
# Allow direnv for this project
direnv allow .

# Variables will automatically load when you cd into the project
```

**OR load manually**:
```bash
# Load environment variables from .env
export $(grep -v '^#' .env | xargs)
```

### Docker/Container Configuration

For containerized deployments:

```dockerfile
# Dockerfile
ENV GOOGLE_PROJECT_ID=your-project-id
ENV GOOGLE_APPLICATION_CREDENTIALS=/config/gcp/maproom-sa-key.json
ENV MAPROOM_EMBEDDING_PROVIDER=google
ENV GOOGLE_VERTEX_REGION=us-central1

# Mount the key file as a secret volume
# docker run -v ~/.config/gcp/maproom-sa-key.json:/config/gcp/maproom-sa-key.json:ro
```

**Docker Compose**:
```yaml
# docker-compose.yml
services:
  maproom:
    environment:
      - GOOGLE_PROJECT_ID=your-project-id
      - GOOGLE_APPLICATION_CREDENTIALS=/config/gcp/maproom-sa-key.json
      - MAPROOM_EMBEDDING_PROVIDER=google
      - GOOGLE_VERTEX_REGION=us-central1
    volumes:
      - ~/.config/gcp/maproom-sa-key.json:/config/gcp/maproom-sa-key.json:ro
```

---

## Step 6: Verify Setup

Test your configuration to ensure everything is working correctly.

### Quick Verification

```bash
# Verify environment variables are set
env | grep GOOGLE

# Expected output:
# GOOGLE_PROJECT_ID=your-project-id
# GOOGLE_APPLICATION_CREDENTIALS=/home/youruser/.config/gcp/maproom-sa-key.json
# GOOGLE_VERTEX_REGION=us-central1

# Verify the service account key file exists and has correct permissions
ls -la $GOOGLE_APPLICATION_CREDENTIALS

# Expected output:
# -rw------- 1 youruser youruser 2345 Oct 28 10:00 /home/youruser/.config/gcp/maproom-sa-key.json
```

### Test with Maproom

```bash
# Test configuration with a small scan
crewchief maproom scan --generate-embeddings

# Expected output:
# ✓ Using embedding provider: google (768 dimensions)
# ✓ Authenticated to GCP project: your-project-id
# ✓ Region: us-central1
# Scanning files...
```

### Validate Credentials with gcloud

```bash
# Activate the service account (for testing only)
gcloud auth activate-service-account \
  --key-file=$GOOGLE_APPLICATION_CREDENTIALS

# Test API access
gcloud ai models list \
  --region=us-central1 \
  --project=$GOOGLE_PROJECT_ID

# Expected output: List of available Vertex AI models
```

---

## Regional Endpoints

Vertex AI operates in multiple global regions. Choose the region closest to your location or that meets your data residency requirements.

### Available Regions

| Region | Location | Model Availability | Latency (US) | Data Residency |
|--------|----------|-------------------|--------------|----------------|
| `us-central1` | Iowa, USA | ✅ All models | ~20ms (US West) | USA |
| `us-west1` | Oregon, USA | ✅ All models | ~10ms (US West) | USA |
| `us-east4` | Virginia, USA | ✅ All models | ~15ms (US East) | USA |
| `europe-west1` | Belgium | ✅ All models | ~100ms (US) | EU |
| `europe-west4` | Netherlands | ✅ All models | ~110ms (US) | EU |
| `asia-southeast1` | Singapore | ✅ All models | ~180ms (US) | APAC |
| `asia-northeast1` | Tokyo, Japan | ✅ Most models | ~120ms (US) | APAC |

**Note**: Model availability varies by region. `text-embedding-gecko@003` is available in all major regions. Check [current availability](https://cloud.google.com/vertex-ai/docs/general/locations).

### Choosing a Region

**Choose based on:**

1. **Latency**: Select the region geographically closest to your deployment
   - US deployments: `us-central1`, `us-west1`, or `us-east4`
   - European deployments: `europe-west1` or `europe-west4`
   - Asia-Pacific deployments: `asia-southeast1` or `asia-northeast1`

2. **Data Residency**: Comply with regulations (GDPR, data sovereignty)
   - EU data must stay in EU: Use `europe-west*` regions
   - APAC data requirements: Use `asia-*` regions

3. **Model Availability**: Ensure your desired model is available
   - Check [Vertex AI locations](https://cloud.google.com/vertex-ai/docs/general/locations#available-regions)

4. **Cost**: Pricing may vary slightly by region
   - Check [regional pricing](https://cloud.google.com/vertex-ai/pricing#region)

### Configuring Region

**Environment Variable** (recommended):
```bash
export GOOGLE_VERTEX_REGION="europe-west1"  # For EU deployments
```

**Application Default**: If not set, Maproom defaults to `us-central1`

### Multi-Region Deployment

For global deployments with low latency everywhere:

```bash
# Deploy separate Maproom instances per region
# US instance
export GOOGLE_VERTEX_REGION="us-central1"

# EU instance
export GOOGLE_VERTEX_REGION="europe-west1"

# APAC instance
export GOOGLE_VERTEX_REGION="asia-southeast1"
```

---

## Cost Estimation and Budgeting

### Pricing Model

Vertex AI embeddings use **pay-per-use** pricing:

- **Model**: `text-embedding-gecko@003`
- **Price**: Approximately **$0.00025 per 1,000 characters** (as of October 2025)
- **Billing**: Per character, not per request (batch-friendly)

### Example Cost Calculations

**Scenario 1: Small TypeScript project**
- Files: 500 TypeScript files
- Average chunk size: 300 characters
- Total chunks: ~2,000
- Total characters: 600,000
- **Estimated cost**: $0.15

**Scenario 2: Medium monorepo**
- Files: 5,000 files (TypeScript, Rust, Go)
- Average chunk size: 500 characters
- Total chunks: ~50,000
- Total characters: 25,000,000
- **Estimated cost**: $6.25

**Scenario 3: Large enterprise codebase**
- Files: 100,000 files
- Average chunk size: 500 characters
- Total chunks: 1,000,000
- Total characters: 500,000,000
- **Estimated cost**: $125

**Scenario 4: Daily development (re-indexing)**
- Changed files per day: 50 files
- Average chunk size: 500 characters
- Chunks per day: ~500
- Total characters: 250,000
- **Daily cost**: $0.06
- **Monthly cost (22 working days)**: ~$1.32

### Cost Optimization Strategies

1. **Enable embedding caching** (Maproom does this by default)
   - Only re-generate embeddings for changed chunks
   - Reduces costs by 90%+ in steady-state

2. **Batch processing** (Maproom does this automatically)
   - Multiple instances per request reduces overhead
   - No additional cost, but more efficient

3. **Selective indexing**
   - Only index relevant file types (exclude test fixtures, generated code)
   - Use `.maproomignore` to exclude large generated files

4. **Monitor usage**
   - Set up billing alerts in GCP Console
   - Review usage in Vertex AI dashboards

### Setting Budget Alerts

Protect against unexpected costs:

1. Navigate to **Billing > Budgets & alerts** in GCP Console
2. Click **"CREATE BUDGET"**
3. **Budget name**: "Maproom Vertex AI Monthly Budget"
4. **Budget amount**: Set threshold (e.g., $50/month)
5. **Alert thresholds**: 50%, 90%, 100%
6. **Email notifications**: Add your email
7. Click **"FINISH"**

**gcloud CLI**:
```bash
# Create a budget alert
gcloud billing budgets create \
  --billing-account=YOUR_BILLING_ACCOUNT_ID \
  --display-name="Maproom Vertex AI Budget" \
  --budget-amount=50 \
  --threshold-rule=percent=50 \
  --threshold-rule=percent=90 \
  --threshold-rule=percent=100
```

---

## Quota Management

Google Cloud enforces quotas to prevent abuse and ensure fair resource distribution.

### Default Quotas

For Vertex AI prediction endpoints:
- **Requests per minute (RPM)**: 60
- **Requests per day**: 86,400
- **Online prediction requests per minute per model**: 60

### Checking Current Quotas

**GCP Console**:
1. Navigate to **IAM & Admin > Quotas**
   - Direct link: [Quotas Console](https://console.cloud.google.com/iam-admin/quotas)
2. **Filter**: "Vertex AI API"
3. View current usage and limits

**gcloud CLI**:
```bash
# List Vertex AI quotas
gcloud compute project-info describe --project=YOUR_PROJECT_ID \
  | grep -A 10 "aiplatform"
```

### Requesting Quota Increases

If you hit quota limits during large indexing operations:

**GCP Console**:
1. Navigate to **IAM & Admin > Quotas**
2. **Filter**: "Vertex AI API"
3. Select the quota to increase (e.g., "Requests per minute")
4. Click **"EDIT QUOTAS"**
5. Click **"EDIT QUOTA"** for the selected quota
6. **New limit**: Enter desired limit (e.g., 300 RPM)
7. **Request description**: "Maproom code indexing requires higher throughput for large codebase"
8. Click **"SUBMIT REQUEST"**

**Processing time**: Quota increase requests typically take 24-48 hours. Google may contact you for justification.

### Handling Rate Limits in Maproom

Maproom automatically handles rate limiting with:
- **Exponential backoff**: Retries with increasing delays
- **Batch processing**: Multiple chunks per request
- **Concurrent request limiting**: Respects quota limits

If you see rate limit errors:
```bash
# Reduce concurrency (slower but respects quotas)
export MAPROOM_MAX_CONCURRENT_REQUESTS=5

# Increase retry delays
export MAPROOM_RETRY_DELAY_MS=2000
```

---

## Security Best Practices

### Service Account Key Security

**Key File Permissions**:
```bash
# ALWAYS set restrictive permissions
chmod 600 ~/.config/gcp/maproom-sa-key.json

# Verify no one else can read
ls -la ~/.config/gcp/maproom-sa-key.json
# Should show: -rw------- (owner read/write only)
```

**Never Commit Keys to Git**:
```gitignore
# Add to .gitignore
*.json
*-key.json
google-credentials.json
.config/gcp/
```

**Audit Key Usage**:
```bash
# List all keys for a service account
gcloud iam service-accounts keys list \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Shows key IDs, creation dates, and expiry
```

### Key Rotation

Rotate service account keys every **90 days** (industry best practice):

**Rotation Process**:
```bash
# Step 1: Create new key
gcloud iam service-accounts keys create ~/.config/gcp/maproom-sa-key-new.json \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --project=YOUR_PROJECT_ID

# Step 2: Update environment variable
export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key-new.json"

# Step 3: Test new key
crewchief maproom scan --dry-run

# Step 4: If successful, delete old key
# First, get the old key ID
gcloud iam service-accounts keys list \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Delete old key (replace KEY_ID with actual ID from list)
gcloud iam service-accounts keys delete KEY_ID \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Step 5: Securely delete old key file
shred -vfz ~/.config/gcp/maproom-sa-key.json
mv ~/.config/gcp/maproom-sa-key-new.json ~/.config/gcp/maproom-sa-key.json
```

**Automate with Calendar Reminder**: Set a recurring reminder every 90 days to rotate keys.

### Workload Identity (Production Recommended)

For production deployments on **Google Kubernetes Engine (GKE)** or **Cloud Run**, use **Workload Identity** instead of service account keys. This eliminates the need to manage JSON key files.

**Benefits**:
- ✅ No service account keys to manage or rotate
- ✅ Automatic credential rotation
- ✅ Better audit logging
- ✅ Reduced security risk

**Setup for GKE**:
```bash
# 1. Enable Workload Identity on your GKE cluster
gcloud container clusters update YOUR_CLUSTER_NAME \
  --workload-pool=YOUR_PROJECT_ID.svc.id.goog

# 2. Create Kubernetes service account
kubectl create serviceaccount maproom-ksa --namespace=default

# 3. Bind Kubernetes service account to Google service account
gcloud iam service-accounts add-iam-policy-binding maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --role roles/iam.workloadIdentityUser \
  --member "serviceAccount:YOUR_PROJECT_ID.svc.id.goog[default/maproom-ksa]"

# 4. Annotate Kubernetes service account
kubectl annotate serviceaccount maproom-ksa \
  --namespace=default \
  iam.gke.io/gcp-service-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com

# 5. Deploy Maproom with this service account
# In your deployment.yaml:
# spec:
#   serviceAccountName: maproom-ksa
```

**No `GOOGLE_APPLICATION_CREDENTIALS` needed** - Workload Identity handles authentication automatically.

**References**:
- [Workload Identity Setup Guide](https://cloud.google.com/kubernetes-engine/docs/how-to/workload-identity)
- [Cloud Run Workload Identity](https://cloud.google.com/run/docs/securing/service-identity)

### Audit Logging

Enable audit logs to track service account usage:

**GCP Console**:
1. Navigate to **IAM & Admin > Audit Logs**
2. Find "Vertex AI API"
3. Enable:
   - ✅ Admin Read
   - ✅ Data Read
   - ✅ Data Write

**Review logs**:
```bash
# View recent Vertex AI API calls
gcloud logging read "resource.type=aiplatform.googleapis.com/Endpoint" \
  --limit=50 \
  --format=json
```

### Least Privilege Review

Periodically review IAM permissions:

```bash
# List all roles for service account
gcloud projects get-iam-policy YOUR_PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com"

# Should ONLY show: roles/aiplatform.user
```

**If you see additional roles**:
```bash
# Remove unnecessary roles
gcloud projects remove-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/UNNECESSARY_ROLE"
```

---

## Troubleshooting

### Error: "GOOGLE_PROJECT_ID environment variable not set"

**Symptoms**:
```
Error: GOOGLE_PROJECT_ID environment variable not set
```

**Cause**: The `GOOGLE_PROJECT_ID` environment variable is missing.

**Solution**:
```bash
# Set the environment variable
export GOOGLE_PROJECT_ID="your-project-id"

# Verify it's set
echo $GOOGLE_PROJECT_ID

# Add to shell profile for persistence
echo 'export GOOGLE_PROJECT_ID="your-project-id"' >> ~/.bashrc
source ~/.bashrc
```

### Error: "Service account credentials file not found"

**Symptoms**:
```
Error: Could not find service account key file at /path/to/key.json
```

**Cause**: The `GOOGLE_APPLICATION_CREDENTIALS` path is incorrect or file doesn't exist.

**Solution**:
```bash
# Verify the file exists
ls -la $GOOGLE_APPLICATION_CREDENTIALS

# If file doesn't exist, check the path
echo $GOOGLE_APPLICATION_CREDENTIALS

# Update to correct path
export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key.json"

# Verify file has correct permissions
chmod 600 $GOOGLE_APPLICATION_CREDENTIALS
```

### Error: "403 Forbidden" or "Permission denied"

**Symptoms**:
```
Error: 403 Forbidden: Permission denied on resource project YOUR_PROJECT_ID
```

**Cause**: Service account lacks the required IAM permissions.

**Solution**:
```bash
# Verify the service account has the correct role
gcloud projects get-iam-policy YOUR_PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com"

# If roles/aiplatform.user is missing, grant it
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/aiplatform.user"

# Wait 30-60 seconds for IAM propagation
sleep 60

# Test again
crewchief maproom scan --dry-run
```

**If still failing**, check:
- Service account is from the correct project
- Billing is enabled on the project
- Vertex AI API is enabled

### Error: "API has not been enabled"

**Symptoms**:
```
Error: Vertex AI API has not been enabled for project YOUR_PROJECT_ID
```

**Cause**: Vertex AI API is not enabled for your GCP project.

**Solution**:
```bash
# Enable the Vertex AI API
gcloud services enable aiplatform.googleapis.com --project=YOUR_PROJECT_ID

# Verify it's enabled
gcloud services list --enabled --project=YOUR_PROJECT_ID | grep aiplatform

# Wait 1-2 minutes for propagation
sleep 120

# Test again
crewchief maproom scan --dry-run
```

### Error: "429 Too Many Requests" or "Quota exceeded"

**Symptoms**:
```
Error: 429 Resource exhausted: Quota exceeded for quota metric 'Prediction requests'
```

**Cause**: You've exceeded Vertex AI quota limits (default: 60 requests/minute).

**Solution 1: Reduce concurrency** (immediate fix):
```bash
# Reduce concurrent requests
export MAPROOM_MAX_CONCURRENT_REQUESTS=5

# Increase delay between requests
export MAPROOM_RETRY_DELAY_MS=2000

# Retry the operation
crewchief maproom scan
```

**Solution 2: Request quota increase** (long-term fix):
1. Navigate to [IAM & Admin > Quotas](https://console.cloud.google.com/iam-admin/quotas)
2. Filter by "Vertex AI API"
3. Select "Prediction requests per minute"
4. Click "EDIT QUOTAS"
5. Request higher limit (e.g., 300 RPM)
6. Provide justification: "Maproom code indexing for large codebase"
7. Submit and wait 24-48 hours

**Solution 3: Batch processing** (Maproom does this automatically):
```bash
# Verify batch size is optimized
# Maproom batches up to 5 instances per request by default
```

### Error: "Invalid JWT signature"

**Symptoms**:
```
Error: Invalid JWT signature
```

**Cause**: Service account key file is corrupted, tampered with, or from wrong project.

**Solution**:
```bash
# Delete the corrupted key file
rm $GOOGLE_APPLICATION_CREDENTIALS

# Create a new key
gcloud iam service-accounts keys create ~/.config/gcp/maproom-sa-key.json \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --project=YOUR_PROJECT_ID

# Set correct permissions
chmod 600 ~/.config/gcp/maproom-sa-key.json

# Test
crewchief maproom scan --dry-run
```

### Error: "Model not found in region"

**Symptoms**:
```
Error: Model 'text-embedding-gecko@003' not found in region 'asia-east1'
```

**Cause**: The specified model is not available in your selected region.

**Solution**:
```bash
# Use a region where the model is available
export GOOGLE_VERTEX_REGION="us-central1"  # Most stable, all models available

# Or check model availability
gcloud ai models list \
  --region=us-central1 \
  --project=YOUR_PROJECT_ID \
  | grep text-embedding-gecko

# Retry
crewchief maproom scan
```

**Available regions for `text-embedding-gecko@003`**:
- `us-central1` ✅ (most stable)
- `us-west1` ✅
- `us-east4` ✅
- `europe-west1` ✅
- `europe-west4` ✅
- `asia-southeast1` ✅

### Error: "Billing not enabled"

**Symptoms**:
```
Error: Project YOUR_PROJECT_ID is not linked to a billing account
```

**Cause**: GCP project does not have billing enabled.

**Solution**:
1. Navigate to [Billing](https://console.cloud.google.com/billing)
2. Click "LINK A BILLING ACCOUNT"
3. Select or create a billing account
4. Link to your project
5. Wait 5 minutes for activation
6. Retry

### Debugging Tips

**Enable verbose logging**:
```bash
# Set Rust log level for detailed output
export RUST_LOG=debug

# Run Maproom
crewchief maproom scan

# Look for GCP authentication details
# Should show: "Authenticated to GCP project: your-project-id"
```

**Test authentication independently**:
```bash
# Activate service account
gcloud auth activate-service-account \
  --key-file=$GOOGLE_APPLICATION_CREDENTIALS

# List available models (tests API access)
gcloud ai models list \
  --region=$GOOGLE_VERTEX_REGION \
  --project=$GOOGLE_PROJECT_ID

# If this works, authentication is correct
```

**Verify JSON key file format**:
```bash
# Check if valid JSON
jq . $GOOGLE_APPLICATION_CREDENTIALS

# Should show parsed JSON with fields:
# - type: "service_account"
# - project_id: "your-project-id"
# - private_key: "-----BEGIN PRIVATE KEY-----..."
# - client_email: "maproom-embeddings@..."
```

---

## Additional Resources

### Official Google Documentation

- **[Vertex AI Documentation](https://cloud.google.com/vertex-ai/docs)** - Complete Vertex AI reference
- **[Vertex AI Embeddings Guide](https://cloud.google.com/vertex-ai/docs/generative-ai/embeddings/get-text-embeddings)** - Text embeddings API reference
- **[Vertex AI Pricing](https://cloud.google.com/vertex-ai/pricing)** - Current pricing for all regions
- **[Service Account Best Practices](https://cloud.google.com/iam/docs/best-practices-service-accounts)** - IAM security guidance
- **[Workload Identity Setup](https://cloud.google.com/kubernetes-engine/docs/how-to/workload-identity)** - Production authentication
- **[Vertex AI Locations](https://cloud.google.com/vertex-ai/docs/general/locations)** - Regional availability
- **[Quota Management](https://cloud.google.com/docs/quota)** - Understanding and managing quotas

### Maproom Documentation

- **[Provider Comparison Guide](./README.md)** - Compare Google vs Ollama vs OpenAI
- **[Configuration Guide](../../crates/maproom/docs/configuration_guide.md)** - Full configuration reference
- **[Migration Guide](./migration-guide.md)** - Switch between embedding providers

### Community and Support

- **[Maproom GitHub Issues](https://github.com/yourusername/maproom/issues)** - Report bugs or request features
- **[GCP Community Support](https://cloud.google.com/support/community)** - Google Cloud community forums
- **[Stack Overflow - Vertex AI](https://stackoverflow.com/questions/tagged/google-vertex-ai)** - Q&A for Vertex AI

---

## Quick Reference

### Essential Commands

```bash
# Enable Vertex AI API
gcloud services enable aiplatform.googleapis.com --project=YOUR_PROJECT_ID

# Create service account
gcloud iam service-accounts create maproom-embeddings \
  --display-name="Maproom Embeddings" \
  --project=YOUR_PROJECT_ID

# Grant IAM role
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/aiplatform.user"

# Create service account key
gcloud iam service-accounts keys create ~/.config/gcp/maproom-sa-key.json \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Set environment variables
export GOOGLE_PROJECT_ID="your-project-id"
export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key.json"
export MAPROOM_EMBEDDING_PROVIDER="google"
export GOOGLE_VERTEX_REGION="us-central1"

# Test setup
crewchief maproom scan --dry-run
```

### Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GOOGLE_PROJECT_ID` | ✅ Yes | None | Your GCP project ID |
| `GOOGLE_APPLICATION_CREDENTIALS` | ✅ Yes | None | Path to service account JSON key |
| `MAPROOM_EMBEDDING_PROVIDER` | ✅ Yes | None | Must be set to `google` |
| `GOOGLE_VERTEX_REGION` | ❌ No | `us-central1` | Vertex AI region endpoint |

### IAM Roles Reference

| Role | Purpose | Permissions |
|------|---------|-------------|
| `roles/aiplatform.user` | **Recommended** | Call prediction endpoints, read models |
| `roles/aiplatform.admin` | ❌ Too broad | Full Vertex AI admin (unnecessary) |
| `roles/editor` | ❌ Too broad | Project-wide edit (dangerous) |
| `roles/owner` | ❌ Too broad | Full project access (never use) |

---

**Need help?** Open an issue on [GitHub](https://github.com/yourusername/maproom/issues) or check the [troubleshooting section](#troubleshooting) above.
