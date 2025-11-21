# Security Review: Core Vector Search Exposure

## Security Assessment
- **Input Injection**: The search query is passed to `pgvector`. Ensure that `VectorExecutor` uses parameterized queries (standard in Rust SQLx/Diesel) to prevent SQL injection.
- **Data Exposure**: Ensure that the search results do not expose sensitive database fields that shouldn't be public (though this is a local tool, so risk is low).

## Gaps & Risks
- **Database Credentials**: The CLI needs access to DB credentials. Ensure these are handled securely (env vars, not hardcoded).

## Mitigations
- **Parameterized Queries**: Verify `VectorExecutor` implementation.
- **Credential Management**: Use standard environment variable patterns for DB connection strings.
