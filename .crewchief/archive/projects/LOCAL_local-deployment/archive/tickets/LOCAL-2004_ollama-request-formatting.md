# Ticket: LOCAL-2004: Implement Ollama-specific request formatting

## Status
- [x] **Task completed** - acceptance criteria met (implemented in LOCAL-2002)
- [x] **Tests pass** - related tests pass (verified in LOCAL-2002 testing)
- [x] **Verified** - by the verify-ticket agent (included in LOCAL-2002 verification)

## Agents
- embeddings-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the specific request and response formatting logic for Ollama's embedding API, which differs from OpenAI in field names, structure, and response format. This ensures proper compatibility when generating embeddings through the Ollama provider.

## Background
CrewChief's LOCAL project aims to provide a local-first development environment with Docker-based Ollama and PostgreSQL integration. The embedding generation subsystem needs to support multiple providers (OpenAI and Ollama), each with different API formats.

Ollama's embedding API uses different field names and structure compared to OpenAI's API. Specifically:
- OpenAI uses `"input"` field; Ollama uses `"prompt"` field
- OpenAI supports `"dimensions"` parameter; Ollama does not
- Response structures may vary between providers

Without proper format translation, embedding requests to Ollama will fail. This ticket implements the provider-specific formatting layer to ensure seamless operation with Ollama while maintaining support for OpenAI.

## Acceptance Criteria
- [ ] Request body builder selects correct format based on Provider enum
- [ ] Ollama requests use "prompt" field (not "input")
- [ ] Ollama requests omit "dimensions" field
- [ ] Response parser handles both OpenAI and Ollama embedding response formats
- [ ] Batch processing works correctly for Ollama (array of prompts)
- [ ] Error responses from Ollama are parsed correctly
- [ ] Integration test validates end-to-end embedding generation with Ollama
- [ ] No code duplication between OpenAI and Ollama formatting logic (DRY principle)

## Technical Requirements

### Request Format Implementation
- Build request bodies conditionally based on `Provider` enum
- For **OpenAI** format:
  ```json
  {
    "input": ["text1", "text2"],
    "model": "text-embedding-3-small",
    "dimensions": 1536
  }
  ```
- For **Ollama** format:
  ```json
  {
    "model": "nomic-embed-text",
    "prompt": ["text1", "text2"]
  }
  ```

### Response Format Handling
- Parse Ollama responses with `response.embeddings` or `response.data` field
- Parse OpenAI responses with `response.data` field
- Handle both single and batch embedding responses
- Extract embedding vectors consistently regardless of provider

### Error Handling
- Parse Ollama error responses (format may differ from OpenAI)
- Provide clear error messages indicating provider-specific failures
- Return appropriate error types from the formatter module

### Code Organization
- Use Rust's type system to enforce format correctness at compile time
- Leverage `serde_json` for JSON serialization/deserialization
- Consider enum-based dispatch or trait-based polymorphism for format selection
- Ensure zero runtime overhead for format selection where possible

## Implementation Notes

### Recommended Approach
1. **Create format modules**: Separate modules for OpenAI and Ollama formats
   - `crates/maproom/src/embeddings/formats/openai.rs`
   - `crates/maproom/src/embeddings/formats/ollama.rs`
   - `crates/maproom/src/embeddings/formats/mod.rs` (trait definitions)

2. **Define common trait**: `EmbeddingFormat` trait with methods:
   - `build_request(&self, texts: &[String], model: &str, dimensions: Option<usize>) -> serde_json::Value`
   - `parse_response(&self, response: serde_json::Value) -> Result<Vec<Vec<f32>>>)`
   - `parse_error(&self, response: serde_json::Value) -> Result<String>`

3. **Implement trait for each provider**:
   - `OpenAIFormat` struct implementing `EmbeddingFormat`
   - `OllamaFormat` struct implementing `EmbeddingFormat`

4. **Provider dispatch**: Use `Provider` enum to select appropriate formatter:
   ```rust
   let formatter: Box<dyn EmbeddingFormat> = match provider {
       Provider::OpenAI => Box::new(OpenAIFormat),
       Provider::Ollama => Box::new(OllamaFormat),
   };
   ```

### Dependencies
- **Crate**: `LOCAL-2002` (client modifications must exist to use these formatters)
- **External dependencies**:
  - `serde_json` for JSON manipulation
  - `anyhow` for error handling
  - `thiserror` for custom error types (if needed)

### Reference Documentation
- Ollama embeddings API: https://github.com/ollama/ollama/blob/main/docs/api.md#generate-embeddings
- OpenAI embeddings API: https://platform.openai.com/docs/api-reference/embeddings
- serde_json docs: https://docs.rs/serde_json/latest/serde_json/

### Testing Strategy
- Unit tests for each format implementation (request building, response parsing)
- Error case tests (malformed responses, missing fields)
- Integration tests in LOCAL-2005 will validate end-to-end operation
- Compare output structure consistency between providers

### Design Considerations
- **DRY Principle**: Extract common logic (e.g., vector validation, normalization)
- **Type Safety**: Use strongly-typed structs for request/response bodies where possible
- **Error Context**: Provide detailed error messages mentioning the provider name
- **Future Extensibility**: Design allows adding new providers (e.g., Cohere, HuggingFace) easily

## Risk Assessment
- **Risk**: Ollama API documentation may be incomplete or change
  - **Mitigation**: Include comprehensive integration tests with real Ollama instance; monitor Ollama releases; version-pin Ollama in Docker setup

- **Risk**: Response format variations across Ollama models
  - **Mitigation**: Test with multiple Ollama embedding models (nomic-embed-text, mxbai-embed-large); handle both `embeddings` and `data` response fields

- **Risk**: Performance overhead from dynamic dispatch
  - **Mitigation**: Use trait objects only at request boundaries; consider compile-time dispatch with generics if benchmarks show issues

- **Risk**: Incomplete error handling leads to cryptic failures
  - **Mitigation**: Log raw response bodies on parse failures; provide context-rich error messages; test error scenarios explicitly

## Files/Packages Affected
- `crates/maproom/src/embeddings/formats/mod.rs` (new file - trait definitions)
- `crates/maproom/src/embeddings/formats/openai.rs` (new file - OpenAI format)
- `crates/maproom/src/embeddings/formats/ollama.rs` (new file - Ollama format)
- `crates/maproom/src/embeddings/client.rs` (modified - use formatters)
- `crates/maproom/src/embeddings/mod.rs` (modified - export formats module)
- `crates/maproom/src/config.rs` (reference - Provider enum)
- Unit test files for each format module
