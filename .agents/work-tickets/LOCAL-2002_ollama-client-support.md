# Ticket: LOCAL-2002: Modify OpenAIClient for Ollama Support

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- embeddings-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify the OpenAIClient in `crates/maproom/src/embedding/client.rs` to support Ollama's API format alongside the existing OpenAI implementation. This enables Maproom to use local LLM embeddings from Ollama without requiring API keys.

## Background
Ollama provides a local embedding service with an API similar to OpenAI, but with critical differences in authentication, request format, and response structure. To enable zero-configuration local embeddings, the OpenAIClient needs to handle both provider formats based on the Provider enum variant.

This task is on the critical path for LOCAL project success. Without Ollama API compatibility, the local embedding service cannot function, blocking Phase 2 and all subsequent work.

**Key Differences**:
- **Authentication**: OpenAI requires Bearer token, Ollama requires no authentication
- **Request Format**: Different field names (`prompt` vs `input`) and structure
- **Endpoint**: OpenAI uses external API, Ollama uses local server (http://ollama:11434)

## Acceptance Criteria
- [ ] OpenAIClient::try_embed_batch handles both OpenAI and Ollama request formats
- [ ] No Authorization header is sent to Ollama provider
- [ ] Ollama requests use correct JSON format: `{"model": model, "prompt": texts}`
- [ ] OpenAI requests continue to use existing format: `{"input": texts, "model": model, "dimensions": dimension}`
- [ ] Successful embedding generation from Ollama endpoint returns Vec<Vector>
- [ ] Error messages distinguish between OpenAI and Ollama failures
- [ ] Existing OpenAI functionality unchanged (zero regressions)
- [ ] Code compiles without warnings

## Technical Requirements

### 1. Conditional Header Logic
- Check `self.config.provider` to determine which headers to include
- OpenAI: Include `Authorization: Bearer {api_key}`
- Ollama: Omit Authorization header entirely
- Both: Include `Content-Type: application/json`

### 2. Request Body Formatting
- OpenAI format (existing):
  ```rust
  serde_json::json!({
      "input": texts,
      "model": self.config.model,
      "dimensions": self.config.dimension,
  })
  ```
- Ollama format (new):
  ```rust
  serde_json::json!({
      "model": self.config.model,
      "prompt": texts,
  })
  ```

### 3. API Endpoint Configuration
- Use `self.config.api_endpoint_url()` to get correct endpoint
- OpenAI: https://api.openai.com/v1/embeddings
- Ollama: http://ollama:11434/api/embeddings (from docker-compose network)

### 4. Error Handling
- Distinguish provider in error messages
- Handle Ollama-specific error responses
- Provide clear troubleshooting context

## Implementation Notes

### Reference Architecture (LOCAL_ARCHITECTURE.md lines 732-804)

The implementation should follow this pattern:

```rust
async fn try_embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>, EmbeddingError> {
    let api_key = self.config.api_key.as_ref();

    // Build request based on provider
    let request = match self.config.provider {
        Provider::OpenAI => {
            let key = api_key.ok_or_else(|| {
                EmbeddingError::Config(ConfigError::MissingConfig("API key".to_string()))
            })?;

            self.client
                .post(&self.config.api_endpoint_url())
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
        },
        Provider::Ollama => {
            // Ollama doesn't require API key
            self.client
                .post(&self.config.api_endpoint_url())
                .header("Content-Type", "application/json")
        },
    };

    // Ollama uses different request format
    let body = if self.config.provider == Provider::Ollama {
        serde_json::json!({
            "model": self.config.model,
            "prompt": texts,
        })
    } else {
        serde_json::json!({
            "input": texts,
            "model": self.config.model,
            "dimensions": self.config.dimension,
        })
    };

    let response = request.json(&body).send().await?;

    // Handle response parsing...
    // (continue with existing response handling logic)
}
```

### Key Considerations

1. **Backward Compatibility**: OpenAI functionality must remain unchanged
2. **Response Parsing**: Both providers return similar response structures (verify this during implementation)
3. **Model Names**: Ollama uses model names like "nomic-embed-text", OpenAI uses "text-embedding-3-small"
4. **Batch Sizes**: Ollama may have different batch size limits than OpenAI
5. **Timeout Handling**: Local Ollama may have different performance characteristics

### Testing Strategy

Testing will be handled in LOCAL-2005, but consider:
- Unit tests for request formatting logic
- Mock tests for both provider types
- Integration tests will validate actual Ollama connectivity

## Dependencies

**Prerequisite Tickets**:
- **LOCAL-2001** (CRITICAL): Provider enum must have Ollama variant before this implementation
- **LOCAL-1003** (CRITICAL): docker-compose.yml must define Ollama service for integration testing

**Blocks**:
- LOCAL-2004: Ollama-specific request formatting refinements
- LOCAL-2005: Integration tests for Ollama provider
- LOCAL-2006: Batch embedding tests with nomic-embed-text

## Risk Assessment

- **Risk**: Ollama response format differs from OpenAI in unexpected ways
  - **Mitigation**: Test early with real Ollama endpoint, review Ollama API docs thoroughly

- **Risk**: Breaking existing OpenAI functionality
  - **Mitigation**: Comprehensive regression testing, ensure all OpenAI tests still pass

- **Risk**: Error handling doesn't distinguish between provider failures
  - **Mitigation**: Include provider type in all error messages, add specific error variants

- **Risk**: Batch size differences cause failures
  - **Mitigation**: Research Ollama batch limits, implement conservative defaults

## Files/Packages Affected

### Primary File
- `crates/maproom/src/embedding/client.rs` - Main implementation file

### Related Files (context)
- `crates/maproom/src/embedding/config.rs` - Provider enum and configuration
- `crates/maproom/src/embedding/mod.rs` - Module exports
- `crates/maproom/src/embedding/error.rs` - Error types (may need updates)

### Test Files (LOCAL-2005)
- `crates/maproom/tests/embedding_integration.rs` - Integration tests
- `crates/maproom/src/embedding/client.rs` - Unit tests (inline)

## References

**External Documentation**:
- Ollama Embeddings API: https://github.com/ollama/ollama/blob/main/docs/api.md#generate-embeddings
- OpenAI Embeddings API: https://platform.openai.com/docs/api-reference/embeddings
- reqwest HTTP Client: https://docs.rs/reqwest/latest/reqwest/

**Planning Documents**:
- LOCAL_PLAN.md: Phase 2 Task LOCAL-2002 (line 84)
- LOCAL_ARCHITECTURE.md: Section 5.2.1 "Client Modifications" (lines 732-804)
- LOCAL_ANALYSIS.md: Ollama API compatibility analysis

**Related Tickets**:
- LOCAL-2001: Add Ollama variant to Provider enum
- LOCAL-2003: Update EmbeddingConfig validation for Ollama
- LOCAL-2004: Implement Ollama-specific request formatting
- LOCAL-2005: Add integration tests for Ollama provider
