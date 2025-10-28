# Ticket: MPEMBED-3002: Add Google provider to factory

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- provider-abstraction-architect
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Add "google" case to the provider factory, validate GOOGLE_PROJECT_ID and GOOGLE_APPLICATION_CREDENTIALS environment variables, and implement graceful error handling with helpful messages for authentication and configuration issues.

## Background
This ticket extends the provider factory created in Phase 2 (MPEMBED-2004) to support Google Vertex AI as a third embedding provider option. The factory must validate GCP-specific configuration, instantiate the GoogleProvider, and provide clear error messages to guide users through setup issues.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-3-google-vertex-ai.md

## Acceptance Criteria
- [ ] Factory supports EMBEDDING_PROVIDER=google environment variable
- [ ] GOOGLE_PROJECT_ID validation (required when provider=google)
- [ ] GOOGLE_APPLICATION_CREDENTIALS path validation (file exists, readable)
- [ ] Service account credentials validation (valid JSON structure)
- [ ] Helpful error messages for missing/invalid configuration
- [ ] Factory returns Arc<dyn EmbeddingProvider> with GoogleProvider
- [ ] Unit tests for google provider instantiation
- [ ] Unit tests for configuration validation errors

## Technical Requirements
- Extend create_provider() function with "google" match arm
- Read GOOGLE_PROJECT_ID from environment (required)
- Read GOOGLE_APPLICATION_CREDENTIALS from environment (required)
- Validate credentials file exists and is readable before instantiation
- Parse service account JSON to validate structure (project_id, private_key, client_email)
- Return descriptive errors for each validation failure
- Maintain consistent error types across all providers
- Add google provider to factory unit tests

## Implementation Notes
**Factory Extension:**
```rust
// crates/maproom/src/embedding/factory.rs
pub fn create_provider(provider_name: &str) -> Result<Arc<dyn EmbeddingProvider>> {
    match provider_name {
        "openai" => { /* existing */ },
        "ollama" => { /* existing */ },
        "google" => {
            let project_id = std::env::var("GOOGLE_PROJECT_ID")
                .context("GOOGLE_PROJECT_ID environment variable not set. Required for Google provider.")?;

            let creds_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .context("GOOGLE_APPLICATION_CREDENTIALS environment variable not set. Set it to path of service account JSON key.")?;

            let creds_path = PathBuf::from(creds_path);
            if !creds_path.exists() {
                bail!("Service account credentials file not found at: {}", creds_path.display());
            }

            // Validate JSON structure early
            validate_service_account_json(&creds_path)?;

            Ok(Arc::new(GoogleProvider::new(project_id, creds_path)?))
        },
        _ => bail!("Unknown provider: {}. Supported: openai, ollama, google", provider_name),
    }
}

fn validate_service_account_json(path: &Path) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read service account JSON file")?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .context("Service account file is not valid JSON")?;

    // Validate required fields
    let required = ["project_id", "private_key", "client_email", "type"];
    for field in required {
        if !json.get(field).is_some() {
            bail!("Service account JSON missing required field: {}", field);
        }
    }

    if json["type"] != "service_account" {
        bail!("Service account JSON has wrong type: expected 'service_account', got '{}'", json["type"]);
    }

    Ok(())
}
```

**Error Message Examples:**
- Missing project ID: "GOOGLE_PROJECT_ID environment variable not set. Get your project ID from GCP Console."
- Missing credentials: "GOOGLE_APPLICATION_CREDENTIALS environment variable not set. Set it to the path of your service account JSON key file."
- File not found: "Service account credentials file not found at: /path/to/key.json. Verify the path is correct."
- Invalid JSON: "Service account file is not valid JSON. Download a new key from GCP Console."
- Wrong type: "Service account JSON has wrong type: expected 'service_account', got 'authorized_user'."

## Dependencies
- MPEMBED-3001 (GoogleProvider implementation must exist)
- MPEMBED-2004 (Provider factory must exist)

## Risk Assessment
- **Risk**: Validation may not catch all GCP authentication issues
  - **Mitigation**: Validate early but defer actual API authentication to provider instantiation, document common issues
- **Risk**: Users may confuse GOOGLE_PROJECT_ID with project number or name
  - **Mitigation**: Error messages clarify format, provide link to GCP Console
- **Risk**: Service account JSON validation may be too strict
  - **Mitigation**: Only validate essential fields, allow GCP SDK to handle schema evolution

## Files/Packages Affected
- crates/maproom/src/embedding/factory.rs (modify - add google case)
- crates/maproom/tests/unit/factory_test.rs (modify - add google validation tests)
