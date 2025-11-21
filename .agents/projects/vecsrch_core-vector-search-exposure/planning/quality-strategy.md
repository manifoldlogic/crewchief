# Quality Strategy: Core Vector Search Exposure

## Test Strategy
- **Unit Tests**: Verify that the CLI argument parser correctly interprets the new flags.
- **Integration Tests**:
    - Run the new CLI command against a known test database/index.
    - Verify that the output JSON matches the expected schema.
    - Verify that relevant results are returned for a known query.

## Critical Paths
- **Argument Parsing**: Ensuring user input is correctly passed to `VectorExecutor`.
- **Error Handling**: Gracefully handling cases where the DB is unreachable or the index is missing.

## Risk Mitigation
- **Dry Run**: Implement a way to see the generated SQL or query plan without executing it, if possible, for debugging.
- **Schema Validation**: Ensure the output JSON schema is strictly defined to prevent breaking downstream consumers (MCP).
