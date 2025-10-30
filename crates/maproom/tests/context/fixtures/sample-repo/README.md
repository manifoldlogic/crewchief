# Sample Repository for Context Assembly Testing

This is a small, realistic codebase used for testing the context assembly system.

## Structure

- `src/lib.rs` - Main library entry point
- `src/utils.rs` - Utility functions (config loading, validation, formatting)
- `src/api.rs` - API request handling

## Relationships

- `lib::run()` calls `utils::load_config()` and `api::process_request()`
- `api::process_request()` calls `utils::validate_config()` and `utils::format_output()`
- `api::process_request()` calls internal `fetch_data()`
- Each module has tests that test the public functions

This structure provides:
- Multiple files with interdependencies
- Function call relationships
- Test coverage
- Realistic Rust code patterns
