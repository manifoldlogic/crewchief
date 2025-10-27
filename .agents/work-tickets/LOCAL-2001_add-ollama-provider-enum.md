# Ticket: LOCAL-2001: Add Ollama variant to Provider enum

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Extend the Provider enum in the Rust codebase to support Ollama as a new embedding provider option, alongside the existing OpenAI, Cohere, and Local providers.

## Background
As part of Phase 2 (Ollama Integration), we need to add Ollama as a supported embedding provider in the Maproom codebase. This is a foundational change that enables subsequent work on Ollama API client implementation and configuration handling. The Provider enum is the core abstraction for different embedding backends, and extending it to include Ollama is the first step toward local embedding support.

This change must maintain backward compatibility with existing OpenAI, Cohere, and Local providers while adding the new Ollama variant with proper serialization, deserialization, and parsing support.

## Acceptance Criteria
- [x] Ollama variant added to Provider enum definition
- [x] Serde serialization/deserialization works correctly for all variants including Ollama
- [x] FromStr implementation parses "ollama" string (case-insensitive)
- [x] Enum compiles without errors or warnings
- [x] Unit tests pass for Provider parsing, including new Ollama variant
- [x] No breaking changes to existing OpenAI/Cohere/Local variants
- [x] Code follows existing Rust style and conventions in the file

## Technical Requirements
- File location: `crates/maproom/src/embedding/config.rs`
- Add `Ollama` variant to the Provider enum
- Maintain `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]` attributes
- Maintain `#[serde(rename_all = "lowercase")]` attribute for consistent serialization
- Update `FromStr` implementation to handle "ollama" case (case-insensitive matching)
- Ensure proper error handling in `FromStr` for invalid provider strings
- Default API endpoint for Ollama should be: `http://localhost:11434/api/embeddings`

## Implementation Notes

### Current Enum Structure
The Provider enum currently exists with OpenAI, Cohere, and Local variants. Add Ollama to this list:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Cohere,
    Ollama,  // NEW - add this
    Local,
}
```

### FromStr Implementation
The existing `FromStr` implementation must be updated to handle the "ollama" string (case-insensitive):

```rust
impl FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Provider::OpenAI),
            "cohere" => Ok(Provider::Cohere),
            "ollama" => Ok(Provider::Ollama),  // NEW - add this case
            "local" => Ok(Provider::Local),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}
```

### Testing Requirements
Add unit tests to verify:
1. Parsing "ollama" string to Provider::Ollama
2. Parsing "Ollama" (uppercase) to Provider::Ollama (case-insensitive)
3. Serialization of Provider::Ollama to "ollama" string
4. Deserialization of "ollama" string to Provider::Ollama
5. Existing tests for OpenAI/Cohere/Local still pass

### Design Considerations
- **Serde compatibility**: The `rename_all = "lowercase"` ensures JSON/TOML config files use "ollama" (lowercase)
- **Case-insensitive parsing**: Allows users to write "Ollama" or "ollama" in config
- **Enum ordering**: Ollama placed before Local to maintain alphabetical ordering
- **Copy trait**: Maintained for performance (Provider is used frequently)

### Reference Documentation
- Rust enums: https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html
- Serde derive: https://serde.rs/derive.html
- FromStr trait: https://doc.rust-lang.org/std/str/trait.FromStr.html

## Dependencies
- LOCAL-1003 (Docker Compose orchestration) - infrastructure ready for testing
- This ticket blocks:
  - LOCAL-2002 (OllamaClient struct implementation)
  - LOCAL-2003 (Ollama API client methods)
  - All subsequent Ollama integration work

## Risk Assessment
- **Risk**: Breaking existing Provider parsing in configuration files
  - **Mitigation**: Comprehensive unit tests for all variants, including existing OpenAI/Cohere/Local
  - **Mitigation**: Manual testing of config file deserialization with all provider types

- **Risk**: Incorrect serialization format causing config compatibility issues
  - **Mitigation**: Verify `rename_all = "lowercase"` attribute is preserved
  - **Mitigation**: Test round-trip serialization/deserialization

- **Risk**: Case-sensitivity issues in FromStr implementation
  - **Mitigation**: Use `to_lowercase()` consistently for all variants
  - **Mitigation**: Add test cases for mixed-case input ("Ollama", "OLLAMA", "ollama")

## Files/Packages Affected
- `crates/maproom/src/embedding/config.rs` - Provider enum definition and FromStr implementation
- `crates/maproom/src/embedding/config.rs` (tests module) - Unit tests for Provider parsing
- Potentially: Any files that match exhaustively on Provider enum (compiler will warn if missing Ollama case)
