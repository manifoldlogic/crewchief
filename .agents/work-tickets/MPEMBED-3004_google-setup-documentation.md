# Ticket: MPEMBED-3004: Google Vertex AI setup documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- google-cloud-integration-engineer
- verify-ticket
- commit-ticket

## Summary
Create comprehensive setup guide for Google Vertex AI provider including service account creation, IAM role assignment, key generation, and configuration examples. Follow principle of least privilege for IAM permissions.

## Background
This ticket creates user-facing documentation for the Google Vertex AI embedding provider. The documentation must be clear enough for users unfamiliar with GCP to successfully set up and use the provider. It should follow GCP security best practices, particularly around IAM roles and service account key management.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-3-google-vertex-ai.md

## Acceptance Criteria
- [ ] Step-by-step guide for service account creation in GCP Console
- [ ] IAM role assignment with least-privilege (roles/aiplatform.user)
- [ ] Service account key generation and download instructions
- [ ] Environment variable configuration examples
- [ ] Troubleshooting section with common errors
- [ ] Security best practices section (key rotation, workload identity)
- [ ] Regional endpoint configuration guide
- [ ] Cost estimation and quota management tips
- [ ] Screenshots or GCP Console navigation paths

## Technical Requirements
- Document format: Markdown
- Include both GCP Console (UI) and gcloud CLI instructions
- Link to official GCP documentation where appropriate
- Include example commands with placeholder values clearly marked
- Test instructions on clean GCP project to verify accuracy
- Follow existing docs/ structure and style
- Include code examples for all configuration scenarios

## Implementation Notes
**Document Structure:**
```markdown
# Google Vertex AI Setup Guide

## Overview
- What is Vertex AI
- Cost implications (~$0.00025 per 1K characters)
- When to use Google vs Ollama vs OpenAI

## Prerequisites
- [ ] Google Cloud Platform account
- [ ] Active GCP project with billing enabled
- [ ] Vertex AI API enabled

## Step 1: Enable Vertex AI API
### Using GCP Console
1. Navigate to [Vertex AI](https://console.cloud.google.com/vertex-ai)
2. Click "Enable API"
3. Wait for activation (1-2 minutes)

### Using gcloud CLI
```bash
gcloud services enable aiplatform.googleapis.com --project=YOUR_PROJECT_ID
```

## Step 2: Create Service Account
### Using GCP Console
1. Navigate to IAM & Admin > Service Accounts
2. Click "+ CREATE SERVICE ACCOUNT"
3. Name: `maproom-embeddings`
4. Description: `Service account for Maproom code embedding generation`
5. Click "CREATE AND CONTINUE"

### Using gcloud CLI
```bash
gcloud iam service-accounts create maproom-embeddings \
  --display-name="Maproom Embeddings" \
  --description="Service account for Maproom code embedding generation" \
  --project=YOUR_PROJECT_ID
```

## Step 3: Grant IAM Permissions
**Recommended Role**: `roles/aiplatform.user`

This role provides:
- Permission to call Vertex AI prediction endpoints
- Read access to models
- No permission to modify or delete resources

### Using GCP Console
1. In Service Accounts list, click on `maproom-embeddings`
2. Go to "PERMISSIONS" tab
3. Click "GRANT ACCESS"
4. Add principal: `maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com`
5. Select role: "Vertex AI User"
6. Click "SAVE"

### Using gcloud CLI
```bash
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/aiplatform.user"
```

## Step 4: Create and Download Service Account Key
⚠️ **Security Note**: Service account keys are sensitive credentials. Store securely.

### Using GCP Console
1. In Service Accounts, click on `maproom-embeddings`
2. Go to "KEYS" tab
3. Click "ADD KEY" > "Create new key"
4. Select "JSON" format
5. Click "CREATE"
6. Save file securely (e.g., `~/.config/gcp/maproom-sa-key.json`)

### Using gcloud CLI
```bash
gcloud iam service-accounts keys create ~/.config/gcp/maproom-sa-key.json \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --project=YOUR_PROJECT_ID
```

## Step 5: Configure Environment Variables
```bash
# Required: Your GCP project ID
export GOOGLE_PROJECT_ID="your-project-id"

# Required: Path to service account key JSON
export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key.json"

# Required: Set Google as embedding provider
export EMBEDDING_PROVIDER="google"

# Optional: Regional endpoint (default: us-central1)
export GOOGLE_VERTEX_REGION="us-central1"
```

Add to your shell profile (~/.bashrc, ~/.zshrc):
```bash
echo 'export GOOGLE_PROJECT_ID="your-project-id"' >> ~/.zshrc
echo 'export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key.json"' >> ~/.zshrc
echo 'export EMBEDDING_PROVIDER="google"' >> ~/.zshrc
```

## Step 6: Verify Setup
```bash
# Test configuration
crewchief maproom scan --generate-embeddings

# Should see:
# ✓ Using embedding provider: google (768 dimensions)
# ✓ Authenticated to GCP project: your-project-id
```

## Regional Endpoints
Vertex AI is available in multiple regions. Choose based on:
- **Latency**: Closest to your location
- **Data residency**: Compliance requirements
- **Model availability**: Some models only in specific regions

Available regions:
- `us-central1` (Iowa) - Default, most stable
- `us-west1` (Oregon)
- `us-east4` (Virginia)
- `europe-west1` (Belgium)
- `europe-west4` (Netherlands)
- `asia-southeast1` (Singapore)

## Cost Estimation
**Pricing**: ~$0.00025 per 1,000 characters (textembedding-gecko@003)

Example costs:
- Small codebase (10K chunks, avg 500 chars): ~$1.25
- Medium codebase (100K chunks): ~$12.50
- Large codebase (1M chunks): ~$125

**Free tier**: 1,000 requests/month (check current limits)

## Quota Management
Default quotas:
- **Requests per minute**: 60
- **Requests per day**: 86,400

To increase quotas:
1. Navigate to IAM & Admin > Quotas
2. Filter: "Vertex AI API"
3. Select quota to increase
4. Click "EDIT QUOTAS"
5. Submit request with justification

## Security Best Practices
### Key Rotation
Rotate service account keys every 90 days:
```bash
# Create new key
gcloud iam service-accounts keys create ~/.config/gcp/maproom-sa-key-new.json \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Update environment variable
export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/gcp/maproom-sa-key-new.json"

# Test new key
crewchief maproom status

# Delete old key
gcloud iam service-accounts keys delete OLD_KEY_ID \
  --iam-account=maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com
```

### Workload Identity (GKE/Cloud Run)
For production deployments, use Workload Identity instead of service account keys:
```bash
# Grant Workload Identity binding
gcloud iam service-accounts add-iam-policy-binding maproom-embeddings@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --role roles/iam.workloadIdentityUser \
  --member "serviceAccount:YOUR_PROJECT_ID.svc.id.goog[NAMESPACE/KSA_NAME]"
```

## Troubleshooting
### Error: "GOOGLE_PROJECT_ID environment variable not set"
- **Cause**: Missing project ID configuration
- **Fix**: `export GOOGLE_PROJECT_ID="your-project-id"`

### Error: "Service account credentials file not found"
- **Cause**: Wrong path or file doesn't exist
- **Fix**: Verify path with `ls -la $GOOGLE_APPLICATION_CREDENTIALS`

### Error: "403 Forbidden" or "Permission denied"
- **Cause**: Service account lacks IAM permissions
- **Fix**: Grant `roles/aiplatform.user` role (see Step 3)

### Error: "API has not been enabled"
- **Cause**: Vertex AI API not enabled for project
- **Fix**: `gcloud services enable aiplatform.googleapis.com --project=YOUR_PROJECT_ID`

### Error: "429 Too Many Requests"
- **Cause**: Exceeded quota limits
- **Fix**: Implement rate limiting or request quota increase (see Quota Management)

### Error: "Invalid JWT signature"
- **Cause**: Corrupted or tampered service account key file
- **Fix**: Download new service account key (see Step 4)

### Error: "Model not found in region"
- **Cause**: textembedding-gecko not available in specified region
- **Fix**: Use `us-central1` or check model availability per region

## Additional Resources
- [Vertex AI Documentation](https://cloud.google.com/vertex-ai/docs)
- [Vertex AI Pricing](https://cloud.google.com/vertex-ai/pricing)
- [Service Account Best Practices](https://cloud.google.com/iam/docs/best-practices-service-accounts)
- [Workload Identity Setup](https://cloud.google.com/kubernetes-engine/docs/how-to/workload-identity)
```

## Dependencies
- MPEMBED-3001 (GoogleProvider implementation must be complete to test instructions)

## Risk Assessment
- **Risk**: Documentation may become outdated as GCP Console UI changes
  - **Mitigation**: Focus on gcloud CLI commands (more stable), add "last updated" date, link to official docs
- **Risk**: Users may grant overly broad IAM permissions
  - **Mitigation**: Emphasize least-privilege principle, explain what each role grants
- **Risk**: Service account key security issues
  - **Mitigation**: Include security best practices section, recommend Workload Identity for production

## Files/Packages Affected
- docs/providers/google-vertex-ai-setup.md (create)
- docs/providers/README.md (modify - add link to Google guide)
