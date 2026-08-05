# AWS Bedrock Setup Guide

Amazon Bedrock embeddings for maproom, using the standard AWS credential chain.

**Best for:**

- Organizations already on AWS, where adding a second vendor means a new
  procurement and security review
- Deployments that must keep code inside an AWS account and VPC
- Environments with no long-lived secrets — instance roles, EKS service
  accounts, and IAM Identity Center all work without a maproom-specific key

**Pricing**: ~$0.00002 per 1,000 tokens (Titan Text Embeddings v2, on-demand,
`us-east-1`) — the cheapest of maproom's cloud providers.

**Setup time**: ~5 minutes if the AWS CLI already works on the machine.

---

## Why there is no maproom AWS key

Bedrock does not use an API key. maproom signs each request with AWS Signature
Version 4 using whatever credentials the machine already has, resolved through
the same chain the AWS CLI and SDKs use. If `aws sts get-caller-identity`
works, maproom can resolve credentials the same way.

That covers authentication only. Model access and the `bedrock:InvokeModel`
permission are separate gates — see [Prerequisites](#prerequisites).

This is the point: no new secret is created, stored, rotated, or leaked.

---

## Prerequisites

1. **An AWS account with Bedrock available** in your chosen region.
   Model availability differs per region — see
   [Model support by region](https://docs.aws.amazon.com/bedrock/latest/userguide/models-regions.html).

2. **Model access enabled.** In the AWS console, go to
   **Bedrock → Model access** and enable the embedding model you intend to use.
   This is a one-time, per-account, per-region action. Without it every request
   fails with `AccessDeniedException` even when IAM is correct.

3. **IAM permission to invoke the model:**

   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Sid": "MaproomBedrockEmbeddings",
         "Effect": "Allow",
         "Action": "bedrock:InvokeModel",
         "Resource": [
           "arn:aws:bedrock:*::foundation-model/amazon.titan-embed-text-v2:0"
         ]
       }
     ]
   }
   ```

   That is the entire permission set. maproom calls `InvokeModel` and nothing
   else — it never lists, creates, or deletes Bedrock resources.

---

## Quick start

```bash
# 1. Authenticate however your organization does. Any of these work:
aws sso login --profile my-profile        # IAM Identity Center
export AWS_PROFILE=my-profile
# ...or static keys, or nothing at all on an EC2/EKS host with a role.

# 2. Point maproom at Bedrock.
export MAPROOM_EMBEDDING_PROVIDER=bedrock

# 3. Index with embeddings.
maproom scan --path /path/to/repo --generate-embeddings

# 4. Search.
maproom search --repo myrepo --query "how does auth work" --mode vector
```

Verify credentials independently at any point:

```bash
aws sts get-caller-identity
```

---

## Models

| Model id | Dimensions | Texts per request | Notes |
|----------|-----------:|------------------:|-------|
| `amazon.titan-embed-text-v2:0` | 1024 | 1 | Default. Cheapest, widest regional availability. |
| `amazon.titan-embed-text-v1` | 1536 | 1 | Previous generation. |
| `cohere.embed-english-v3` | 1024 | 96 | Batches natively — far fewer requests per scan. |
| `cohere.embed-multilingual-v3` | 1024 | 96 | Same, for non-English identifiers and comments. |

Select one with:

```bash
export MAPROOM_EMBEDDING_MODEL=cohere.embed-english-v3
```

### Choosing between Titan and Cohere

The practical difference is **request count**, not quality. Bedrock's
`InvokeModel` API takes a single document for Titan, so indexing 50,000 chunks
means 50,000 requests. Cohere accepts 96 texts per call — the same scan is
~520 requests. On a large repository, or under a tight requests-per-minute
quota, Cohere finishes substantially sooner.

Titan v2 is the cheaper of the two per token.

Note that although Titan v2 can emit 512- and 256-dimensional vectors, maproom
stores embeddings in per-dimension tables and has storage only for 768, 1024,
and 1536. Requesting 512 or 256 is rejected at startup, with an error naming
the widths that do work — rather than embedding an entire repository and then
failing on the first database write.

```bash
# Rejected at startup: Titan v2 supports it, maproom cannot store it.
export MAPROOM_EMBEDDING_DIMENSION=512
```

### Cross-region inference profiles and provisioned throughput

Inference-profile ids (`us.amazon.titan-embed-text-v2:0`) and full ARNs are
accepted; maproom resolves the underlying model to infer dimensions. For a
provisioned-throughput ARN whose model maproom cannot identify, set the
dimension explicitly:

```bash
export MAPROOM_EMBEDDING_MODEL=arn:aws:bedrock:us-east-1:123456789012:provisioned-model/abc123
export MAPROOM_EMBEDDING_DIMENSION=1024
```

---

## Credential resolution

maproom tries these sources in order and stops at the first that yields
credentials:

| # | Source | Typical environment |
|---|--------|---------------------|
| 1 | `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` (+ `AWS_SESSION_TOKEN`) | CI, local scripts |
| 2 | A named profile — `MAPROOM_AWS_PROFILE` or `AWS_PROFILE` | Developer laptops |
| 3 | `AWS_WEB_IDENTITY_TOKEN_FILE` + `AWS_ROLE_ARN` | EKS IRSA, GitHub Actions OIDC |
| 4 | The `default` profile | Developer laptops |
| 5 | Container credentials endpoint | ECS task roles, EKS Pod Identity |
| 6 | EC2 instance metadata (IMDSv2) | EC2 build hosts |

A profile itself may resolve through static keys, `credential_process`, IAM
Identity Center (SSO), or `role_arn` + `source_profile` role chaining.

**A named profile that fails is a hard error.** If you set `AWS_PROFILE` and it
is missing or broken, maproom stops rather than falling through to an instance
role — silently using a different AWS account is worse than failing.

Temporary credentials are refreshed automatically five minutes before they
expire, so a long-running `maproom watch` survives rotation without a restart.

### Example: EKS with IRSA

Nothing to configure in maproom. Annotate the service account:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: maproom
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::123456789012:role/maproom-bedrock
```

EKS projects the token and sets `AWS_WEB_IDENTITY_TOKEN_FILE` and
`AWS_ROLE_ARN`; maproom picks them up as source 3.

### Example: role chaining in `~/.aws/config`

```ini
[profile base]
region = us-east-1

[profile bedrock]
role_arn = arn:aws:iam::123456789012:role/BedrockInvoker
source_profile = base
external_id = my-external-id
```

```bash
export AWS_PROFILE=bedrock
```

---

## Region

Resolved in this order:

1. `MAPROOM_BEDROCK_REGION`
2. `AWS_REGION`
3. `AWS_DEFAULT_REGION`
4. `region` in the selected profile
5. `us-east-1`

`MAPROOM_BEDROCK_REGION` exists because Bedrock model availability is narrower
than AWS's region list. It lets you embed in a region that has the model while
the rest of your tooling stays pointed at your primary region:

```bash
export AWS_REGION=eu-west-2                 # everything else
export MAPROOM_BEDROCK_REGION=eu-central-1  # where the model lives
```

---

## Private networking

### VPC endpoints (PrivateLink)

To keep embedding traffic off the public internet, create an interface VPC
endpoint for `com.amazonaws.<region>.bedrock-runtime` and point maproom at it:

```bash
export MAPROOM_BEDROCK_ENDPOINT_URL=https://vpce-0abc123-xyz.bedrock-runtime.us-east-1.vpce.amazonaws.com
```

The SDK-standard `AWS_ENDPOINT_URL_BEDROCK_RUNTIME` and `AWS_ENDPOINT_URL` are
also honored, in that order of precedence after the maproom-specific variable.

Note that maproom deliberately ignores the generic
`MAPROOM_EMBEDDING_API_ENDPOINT` for Bedrock. That variable is frequently set
to an Ollama URL by container tooling, and silently redirecting signed AWS
traffic to it would be a security problem.

### FIPS endpoints

```bash
export MAPROOM_BEDROCK_USE_FIPS=true
```

Uses `bedrock-runtime-fips.<region>.amazonaws.com`. Available in US and Canada
regions only.

---

## Throughput and quotas

Bedrock enforces a requests-per-minute quota per model per region. Because
Titan issues one request per text, a large scan is request-bound.

```bash
# Concurrent in-flight requests (default 12)
export MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=12

# Texts per request — Cohere only; Titan is clamped to 1 (default 96)
export MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=96
```

Throttling (HTTP 429) is retried automatically with exponential backoff, so
occasional 429s are invisible. If a scan is slow and `RUST_LOG=debug` shows
sustained retries, either:

- lower `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY`, or
- request a higher **InvokeModel requests per minute** quota for your model in
  the Service Quotas console, or
- switch to a Cohere model, which needs ~96× fewer requests.

---

## Cost estimation

Titan Text Embeddings v2 at ~$0.00002 per 1,000 tokens, assuming ~200 tokens
per chunk:

| Repository size | Chunks | Approx. tokens | Approx. cost |
|-----------------|-------:|---------------:|-------------:|
| Small | 5,000 | 1M | $0.02 |
| Medium | 50,000 | 10M | $0.20 |
| Large | 500,000 | 100M | $2.00 |

Re-indexing only embeds changed chunks, so ongoing cost is far lower than the
initial scan. maproom tracks actual token usage and reports an estimate through
provider metrics.

Regional and provisioned-throughput pricing differ; see the
[Bedrock pricing page](https://aws.amazon.com/bedrock/pricing/) for current
numbers.

---

## Troubleshooting

### `No AWS credentials found for the Bedrock embedding provider`

maproom lists every source it tried and why each failed. Work down that list.
Confirm the machine can authenticate at all:

```bash
aws sts get-caller-identity
```

Exit code 2 — this is a configuration error and will not succeed on retry.

### `AccessDeniedException` / "not authorized to invoke this model"

Two independent gates, and both must be open:

1. **Model access** — enable the model under **Bedrock → Model access** in the
   console, for this account *and* this region.
2. **IAM** — the caller needs `bedrock:InvokeModel` on the model ARN.

Check which identity is actually being used: maproom logs the credential source
and access key prefix at startup with `RUST_LOG=info`.

### `Model '…' was not found in region …`

Bedrock model availability varies by region. Either switch regions:

```bash
export MAPROOM_BEDROCK_REGION=us-east-1
```

or list what is actually offered where you are:

```bash
# Resolve the region maproom will actually use, following the same precedence
# as above. Querying $AWS_REGION alone can inspect a different region than the
# one maproom embeds in, which is exactly the confusion this is meant to settle.
region="${MAPROOM_BEDROCK_REGION:-${AWS_REGION:-${AWS_DEFAULT_REGION:-$(aws configure get region)}}}"

aws bedrock list-foundation-models --region "${region:-us-east-1}" \
  --query "modelSummaries[?outputModalities[0]=='EMBEDDING'].modelId"
```

### `Cannot infer the embedding dimension for Bedrock model '…'`

The model id is not one maproom recognizes — usually a provisioned-throughput
ARN or a newly released model. Set the width explicitly:

```bash
export MAPROOM_EMBEDDING_DIMENSION=1024
```

maproom refuses to guess here on purpose: a wrong dimension produces an index
that builds successfully and then returns nothing.

### `AWS profile '…' not found`

The error lists the profiles maproom did find in `~/.aws/config` and
`~/.aws/credentials`. Remember that `config` writes non-default profiles as
`[profile name]` while `credentials` writes them as `[name]`; maproom
normalizes both, so either spelling is fine.

### Sustained throttling

See [Throughput and quotas](#throughput-and-quotas) above.

### Requests appear to hang on a non-AWS host

If the machine has unusual routing, IMDS probing can be slow. Skip it:

```bash
export AWS_EC2_METADATA_DISABLED=true
```

---

## Security notes

- **No new secret.** maproom stores no AWS credential; it reads whatever the
  host already provides and holds resolved credentials in memory only.
- **Secrets never reach logs.** Credential and signing structs redact secret
  keys and session tokens in their `Debug` output.
- **IMDSv2 only.** maproom never falls back to IMDSv1, which lacks the
  session-token protection against SSRF.
- **`credential_process` is not run through a shell.** The configured command is
  executed directly with parsed arguments, so a hostile `~/.aws/config` cannot
  inject pipes or substitutions.
- **Least privilege.** `bedrock:InvokeModel` on the specific model ARN is
  sufficient; no broader Bedrock permission is needed.

---

## See also

- [Provider comparison](./comparison.md)
- [Ollama setup](./ollama-setup.md) — free, local, no cloud round trip
- [Vector search configuration](../../crates/maproom/docs/VECTOR_SEARCH_CONFIGURATION.md)
- [AWS Bedrock documentation](https://docs.aws.amazon.com/bedrock/)
