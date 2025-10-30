# MPEMBED: Multi-Provider Embedding Support - Security Review

## Security Philosophy

**MVP pragmatism:**
- Address **obvious pitfalls** (API key leakage, injection attacks)
- Cover **bases pragmatically** (TLS for API calls, input validation)
- Avoid **elite security theater** (perfect key rotation, HSM integration)
- Document **known limitations** for enterprise users to address

**Enterprise expectations:**
- Authentication credentials must not be logged or exposed
- Code sent to external APIs has privacy implications (document clearly)
- Input validation prevents injection attacks
- TLS protects data in transit

## Threat Model

### Assets to Protect

**High value:**
1. **API keys and credentials** (OpenAI, Google Cloud service accounts)
2. **Source code content** (intellectual property sent for embedding)
3. **Database access** (embeddings and code metadata)

**Medium value:**
4. **Search queries** (reveal what developers are looking for)
5. **Embedding vectors** (reverse-engineering possible, but difficult)

**Low value:**
6. **Provider selection** (not sensitive, public preference)
7. **Performance metrics** (useful but not confidential)

### Threat Actors

**Internal threats (primary concern):**
- **Accidental exposure**: Developers committing API keys to git
- **Log leakage**: Credentials appearing in logs, error messages, or CI output
- **Configuration mistakes**: Wrong permissions on credential files

**External threats (secondary):**
- **Network eavesdropping**: MITM attacks on API calls
- **SQL injection**: Malicious input in search queries
- **API key theft**: Stolen keys used for unauthorized embedding generation

**Out of scope for MVP:**
- Nation-state actors with database access
- Zero-day vulnerabilities in PostgreSQL/Rust
- Physical access to servers
- Supply chain attacks on dependencies

## Security by Component

### 1. API Key Management

#### Risk: API keys exposed in logs or environment

**Current exposure points:**
- Environment variables (`OPENAI_API_KEY`, `GOOGLE_APPLICATION_CREDENTIALS`)
- Log output (tracing, error messages)
- Process list (`ps aux` shows env vars)
- Core dumps (env vars in memory)

**Mitigations (pragmatic):**

```rust
// DO: Redact sensitive fields in debug output
#[derive(Debug)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    #[debug(skip)]  // Never log this field
    api_key: Option<String>,
}

// DO: Scrub credentials from error messages
impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmbeddingError::Authentication(msg) => {
                // Strip API key from error message
                let sanitized = msg.replace(|c: char| c.is_alphanumeric(), "*");
                write!(f, "Authentication failed: {}", sanitized)
            }
            // ...
        }
    }
}

// DON'T: Log full config objects
tracing::info!("Using config: {:?}", config); // ❌ Might expose API key

// DO: Log only safe fields
tracing::info!(
    provider = %config.provider,
    model = %config.model,
    "Embedding service initialized"
); // ✅ No credentials logged
```

**Documentation guidance:**
```markdown
## Security Best Practices

**API Key Storage:**
- ✅ Store in `.env` file (git-ignored)
- ✅ Use environment variables in production
- ✅ Use secret managers in enterprise (AWS Secrets Manager, GCP Secret Manager)
- ❌ Never commit API keys to git
- ❌ Never pass API keys as command-line arguments (visible in `ps`)
```

#### Risk: Google service account JSON files readable by other users

**Mitigations:**

```bash
# Ensure service account files have restrictive permissions
chmod 600 /path/to/service-account.json

# Check permissions at runtime
if let Some(creds_path) = env::var("GOOGLE_APPLICATION_CREDENTIALS").ok() {
    let metadata = fs::metadata(&creds_path)?;
    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        tracing::warn!(
            "Service account file {} has overly permissive permissions ({}). \
             Recommend: chmod 600 {}",
            creds_path, mode, creds_path
        );
    }
}
```

### 2. Data Privacy

#### Risk: Source code sent to external APIs (OpenAI, Google)

**Current behavior:**
- Code chunks (functions, classes) sent to embedding APIs as plaintext
- API providers may log requests for debugging/training (per their ToS)
- Network traffic visible to ISP/enterprise proxy

**Mitigations (transparency):**

**Documentation (critical):**
```markdown
## Privacy Considerations

When using cloud embedding providers (OpenAI, Google Vertex AI), your **source code is sent to external APIs** for embedding generation.

### What is sent:
- Function signatures and bodies
- Class definitions
- Comments and docstrings
- File paths (relative to repository root)

### What is NOT sent:
- Full file contents (only extracted symbols)
- Git history or commit messages
- Environment variables or secrets

### Provider policies:
- **OpenAI**: [Data Usage Policy](https://openai.com/policies/api-data-usage-policies) - API data not used for training by default
- **Google Vertex AI**: [Data Processing Terms](https://cloud.google.com/terms/data-processing-addendum) - Subject to GCP DPA

### Recommendations:
- **Proprietary code**: Use Ollama (local, nothing sent externally)
- **Open-source code**: Any provider acceptable
- **Enterprise/regulated**: Use Ollama or Google Vertex AI with private GCP
```

**Code-level mitigation (sanitization option for future):**
```rust
// Optional future enhancement: PII/secret detection before embedding
pub struct SanitizationConfig {
    pub strip_comments: bool,         // Remove comments before embedding
    pub redact_strings: bool,         // Replace string literals with <STRING>
    pub redact_api_keys: bool,        // Detect and redact API key patterns
}

// For now: Document that code is sent as-is, users choose provider accordingly
```

#### Risk: Embeddings stored in database reveal code structure

**Exposure level:**
- Embedding vectors are **not plaintext code** (encoded representations)
- Reverse-engineering embeddings to source code is **extremely difficult** (active research area)
- Embeddings **do** reveal semantic similarity (can infer "these functions are related")

**Mitigations:**
- Standard PostgreSQL access controls (role-based permissions)
- Encrypt database at rest (enterprise feature, not MVP concern)
- Document that embeddings have lower sensitivity than source code

**Acceptable for MVP:**
- No special encryption of embedding columns
- Rely on database-level security (SSL connections, user permissions)

### 3. Input Validation

#### Risk: SQL injection via search queries

**Attack vector:**
```rust
// ❌ DANGEROUS: String interpolation
let query = format!("SELECT * FROM chunks WHERE symbol_name = '{}'", user_input);
```

**Mitigation (already correct in codebase):**
```rust
// ✅ SAFE: Parameterized queries
sqlx::query_as(
    "SELECT * FROM maproom.chunks WHERE symbol_name = $1"
)
.bind(user_input)
.fetch_all(pool)
.await
```

**Verification:**
- Audit all `format!()` calls in `src/search/` and `src/db/`
- Ensure **all user input** goes through parameterized queries
- Run sqlmap or similar tool against search endpoints (future)

#### Risk: Path traversal in file operations

**Attack vector:**
```rust
// ❌ DANGEROUS: Unchecked file paths
let path = format!("{}/{}", repo_root, user_provided_relpath);
fs::read_to_string(path)?;
```

**Mitigation:**
```rust
// ✅ SAFE: Canonicalize and check bounds
let full_path = Path::new(&repo_root).join(&user_provided_relpath);
let canonical = full_path.canonicalize()?;

if !canonical.starts_with(&repo_root) {
    return Err(SecurityError::PathTraversal);
}

fs::read_to_string(canonical)?;
```

**Status:** Need to audit file operations in indexer and MCP tools.

#### Risk: Command injection via provider configuration

**Attack vector:**
```bash
# User sets malicious model name
EMBEDDING_MODEL="; rm -rf /"
```

**Mitigation:**
```rust
// ✅ Validate model names against allowlist
const ALLOWED_OLLAMA_MODELS: &[&str] = &[
    "nomic-embed-text",
    "all-minilm",
    "bge-large",
];

fn validate_model(provider: &str, model: &str) -> Result<(), ConfigError> {
    match provider {
        "ollama" => {
            if !ALLOWED_OLLAMA_MODELS.contains(&model) {
                return Err(ConfigError::InvalidModel(format!(
                    "Ollama model '{}' not in allowlist. Allowed: {:?}",
                    model, ALLOWED_OLLAMA_MODELS
                )));
            }
        }
        "google" => {
            // Google models are versioned, validate format
            let regex = Regex::new(r"^text-embedding-[a-z0-9-]+@\d{3}$")?;
            if !regex.is_match(model) {
                return Err(ConfigError::InvalidModel(format!(
                    "Invalid Google model format: {}", model
                )));
            }
        }
        "openai" => {
            // OpenAI models follow predictable naming
            if !model.starts_with("text-embedding-") {
                return Err(ConfigError::InvalidModel(format!(
                    "Invalid OpenAI model format: {}", model
                )));
            }
        }
        _ => {}
    }
    Ok(())
}
```

### 4. Network Security

#### Risk: MITM attacks on embedding API calls

**Exposure:**
- OpenAI API: `https://api.openai.com` (TLS by default)
- Google Vertex AI: `https://<region>-aiplatform.googleapis.com` (TLS by default)
- Ollama: `http://localhost:11434` (plaintext, but localhost-only)

**Mitigations:**
```rust
// DO: Enforce HTTPS for cloud providers
pub fn validate_endpoint(provider: &str, endpoint: &str) -> Result<(), ConfigError> {
    match provider {
        "openai" | "google" => {
            if !endpoint.starts_with("https://") {
                return Err(ConfigError::InsecureEndpoint(
                    "Cloud providers must use HTTPS".to_string()
                ));
            }
        }
        "ollama" => {
            // Allow HTTP for localhost
            let url = Url::parse(endpoint)?;
            if url.scheme() == "http" && !is_localhost(url.host_str().unwrap_or("")) {
                tracing::warn!(
                    "Ollama endpoint {} uses HTTP over network. Consider HTTPS tunnel.",
                    endpoint
                );
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_localhost(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}
```

**Certificate validation:**
- Rely on `reqwest` default behavior (validates TLS certificates)
- Do NOT allow `danger_accept_invalid_certs(true)` in production

### 5. Dependency Security

#### Risk: Vulnerable dependencies (supply chain attacks)

**Mitigations:**
- Run `cargo audit` in CI pipeline
- Use Dependabot or similar for automatic updates
- Pin major versions, allow patch updates

**Relevant dependencies:**
- `reqwest` - HTTP client (widely used, well-maintained)
- `sqlx` - Database driver (PostgreSQL official)
- `tokio` - Async runtime (Rust core library)
- `google-cloud-auth` - Google auth (official Google crate)

**Action items:**
- Add `cargo audit` to CI
- Enable Dependabot alerts on GitHub
- Review audit results quarterly

### 6. Google Cloud Authentication

#### Risk: Service account keys with excessive permissions

**Recommended IAM roles (least privilege):**
```yaml
# Minimal permissions for embedding generation
roles/aiplatform.user  # Can call Vertex AI predict endpoints
# OR more restrictive:
roles/aiplatform.predictionServiceUser
```

**Avoid:**
```yaml
roles/owner  # ❌ Full project access
roles/editor  # ❌ Can modify resources
```

**Documentation:**
```markdown
## Google Vertex AI Setup

1. Create service account:
   ```bash
   gcloud iam service-accounts create maproom-embeddings \
     --display-name="Maproom Embedding Service"
   ```

2. Grant minimal permissions:
   ```bash
   gcloud projects add-iam-policy-binding PROJECT_ID \
     --member="serviceAccount:maproom-embeddings@PROJECT_ID.iam.gserviceaccount.com" \
     --role="roles/aiplatform.user"
   ```

3. Create key:
   ```bash
   gcloud iam service-accounts keys create ~/maproom-sa-key.json \
     --iam-account=maproom-embeddings@PROJECT_ID.iam.gserviceaccount.com
   ```

4. Secure the key:
   ```bash
   chmod 600 ~/maproom-sa-key.json
   ```

5. Configure:
   ```bash
   export GOOGLE_APPLICATION_CREDENTIALS=~/maproom-sa-key.json
   ```
```

## Security Gaps (Acceptable for MVP)

**What we're NOT doing (but enterprise might need):**

1. **Key rotation**: API keys don't auto-rotate
   - **Impact**: Stolen keys remain valid until manually revoked
   - **Enterprise solution**: Use Google Workload Identity, AWS IAM roles
   - **MVP status**: Document manual key rotation process

2. **Rate limiting**: No protection against API abuse
   - **Impact**: Malicious user could generate unlimited embeddings
   - **Enterprise solution**: Implement rate limiting at MCP layer
   - **MVP status**: Rely on provider-level rate limits

3. **Audit logging**: No record of who generated which embeddings
   - **Impact**: Can't trace back malicious/accidental API usage
   - **Enterprise solution**: Log all embedding requests to audit table
   - **MVP status**: Standard application logs only

4. **Encryption at rest**: Database embeddings not encrypted
   - **Impact**: Direct database access reveals embeddings
   - **Enterprise solution**: PostgreSQL transparent data encryption
   - **MVP status**: Rely on database-level access controls

5. **Secret management**: Environment variables, not secret managers
   - **Impact**: Env vars visible to process inspections
   - **Enterprise solution**: AWS Secrets Manager, GCP Secret Manager, HashiCorp Vault
   - **MVP status**: Document as enterprise enhancement

## Compliance Considerations

**GDPR (EU):**
- **Risk**: Sending EU citizen code to US-based APIs (OpenAI)
- **Mitigation**: Use Ollama (local) or Google Vertex AI with EU regions
- **Documentation**: Clearly state data residency implications

**HIPAA (Healthcare):**
- **Risk**: Sending medical code to non-BAA providers
- **Mitigation**: Ollama only, or Google Cloud with BAA
- **Documentation**: State that OpenAI is not HIPAA-compliant

**SOC 2:**
- **Risk**: Lack of audit logging, key rotation
- **Mitigation**: Document as known limitation, point to enterprise solutions
- **Documentation**: Provide SOC 2 readiness checklist for enterprises

## Security Checklist

**Before merging to main:**
- [ ] All API keys use `#[debug(skip)]` or similar
- [ ] No credentials in error messages or logs
- [ ] All database queries use parameterized inputs (no string interpolation)
- [ ] File paths canonicalized and bounded to repository root
- [ ] HTTPS enforced for cloud providers (OpenAI, Google)
- [ ] Model names validated against allowlist or regex
- [ ] Google service account file permissions checked (warn if >600)
- [ ] Documentation includes privacy considerations
- [ ] Documentation includes provider data policies
- [ ] `cargo audit` passes with no high/critical vulnerabilities

**Post-launch monitoring:**
- [ ] Watch for API key exposure in GitHub search (e.g., `OPENAI_API_KEY` in public repos)
- [ ] Monitor for unusual embedding generation patterns (potential key theft)
- [ ] Track failed authentication attempts (Google, OpenAI)

## Incident Response Plan

**If API key is leaked:**

1. **Immediate (< 1 hour):**
   - Revoke compromised key via provider dashboard
   - Generate new key
   - Update production environment
   - Rotate any downstream credentials

2. **Short-term (< 24 hours):**
   - Audit logs for unauthorized usage
   - Estimate financial impact (API costs)
   - Notify affected users if applicable

3. **Long-term (< 1 week):**
   - Post-mortem: How was key leaked?
   - Implement preventive measures (pre-commit hooks, secret scanning)
   - Update documentation with lessons learned

## Security Documentation

**User-facing:**
- Privacy policy (what data is sent where)
- Provider comparison (data residency, compliance)
- Best practices (key storage, permissions)

**Developer-facing:**
- Secure coding guidelines (no credential logging)
- Dependency management (cargo audit workflow)
- Incident response runbook

## Final Assessment

**Security posture: Acceptable for MVP**

**Strengths:**
- ✅ No obvious critical vulnerabilities
- ✅ Follows Rust best practices (memory safety, type safety)
- ✅ Uses established libraries (reqwest, sqlx, tokio)
- ✅ TLS by default for cloud APIs
- ✅ Parameterized SQL queries

**Weaknesses (documented for enterprise):**
- ⚠️ No key rotation
- ⚠️ No rate limiting
- ⚠️ No audit logging
- ⚠️ Environment variables for secrets

**Recommendation:** Ship MVP with clear documentation of limitations. Enterprises can layer on secret managers, audit logging, and key rotation as needed.
