# Ticket: TESTFIX-1001: Add missing compute_git_blob_sha database function

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create the missing PostgreSQL function `maproom.compute_git_blob_sha()` that computes SHA-256 hashes of file content in git blob format. This function is required by the test suite but currently missing from the database schema.

## Background
The Test workflow is failing because `packages/maproom-mcp/tests/run-blob-sha-tests.cjs` expects a PostgreSQL function `maproom.compute_git_blob_sha()` that doesn't exist in the database schema at `packages/maproom-mcp/config/init.sql`.

The test file validates that the SQL implementation matches the Rust implementation's behavior for:
1. Function existence
2. Known hash values matching Rust implementation
3. Determinism (same input produces same output)
4. Unicode handling
5. Large content handling
6. Newline handling (LF vs CRLF)

This is a Phase 1 ticket addressing a current test failure that blocks the stability of the test workflow.

## Acceptance Criteria
- [ ] Function `maproom.compute_git_blob_sha(TEXT)` exists in init.sql
- [ ] Function returns correct hash for empty string: `473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813`
- [ ] Function returns correct hash for "test" string: `aa19560d465e7d43915547490a1f6b73eb55702e3d12cb82fb577df60bad4928`
- [ ] All tests in run-blob-sha-tests.cjs pass
- [ ] Function is deterministic (same input = same output)
- [ ] Function handles Unicode correctly
- [ ] Schema applies without errors

## Technical Requirements
- **File to Modify**: `packages/maproom-mcp/config/init.sql`
- Function must accept TEXT input (file content)
- Function must return TEXT output (SHA-256 hash as hex string)
- Must compute SHA-256 hash of git blob format: `blob {size}\0{content}`
- Must match known hashes from Rust test suite
- Function should be marked IMMUTABLE for PostgreSQL optimization
- Must use PostgreSQL's built-in `sha256()` function from pgcrypto extension (already enabled)

## Implementation Notes

**Function Signature**:
```sql
CREATE OR REPLACE FUNCTION maproom.compute_git_blob_sha(content TEXT)
RETURNS TEXT AS $$
  -- Implementation here
$$ LANGUAGE plpgsql IMMUTABLE;
```

**Expected Algorithm**:
1. Get byte length of content
2. Construct git blob header: `blob {length}\0`
3. Concatenate header + content
4. Compute SHA-256 hash
5. Return as lowercase hex string (64 characters)

**Git Blob Format Specification**:
The git blob format is `blob {size}\0{content}` where:
- `{size}` is the byte length of the content
- `\0` is a null byte separator
- `{content}` is the actual file content

**Testing Strategy**:
```bash
# Apply schema
psql -d maproom_test -f packages/maproom-mcp/config/init.sql

# Test function with empty string
psql -d maproom_test -c "SELECT maproom.compute_git_blob_sha('');"
# Expected: 473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813

# Test function with "test" string
psql -d maproom_test -c "SELECT maproom.compute_git_blob_sha('test');"
# Expected: aa19560d465e7d43915547490a1f6b73eb55702e3d12cb82fb577df60bad4928

# Run full test suite
cd packages/maproom-mcp
node tests/run-blob-sha-tests.cjs
```

## Dependencies
- **Requires**: PostgreSQL with pgcrypto extension (already enabled in schema)
- **Blocks**: None
- **Blocked By**: None

## Risk Assessment
- **Risk**: Function implementation doesn't match Rust behavior exactly
  - **Mitigation**: Use git blob format specification, test against known hashes from Rust test suite
- **Risk**: Performance issues with large content
  - **Mitigation**: Function is IMMUTABLE, PostgreSQL can optimize and cache results
- **Risk**: Encoding issues with Unicode characters
  - **Mitigation**: Use proper text encoding, validate with Unicode test cases in test suite
- **Risk**: Newline handling differences between platforms (LF vs CRLF)
  - **Mitigation**: Test suite includes newline handling tests to validate behavior

## Files/Packages Affected
- `packages/maproom-mcp/config/init.sql` - Add compute_git_blob_sha function to maproom schema

## Planning References
- Test file: `packages/maproom-mcp/tests/run-blob-sha-tests.cjs`
- Git blob format: `blob {size}\0{content}`
- Known hash values from Rust test suite included in test file
