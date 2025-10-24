# Ticket: LANG_PARSE-1007: Python Database Integration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate Python symbol extraction with the Maproom database layer. Map Python-specific symbols (functions, classes, methods, variables) to database chunk kinds, store Python-specific metadata (decorators, async flags, class bases), and implement incremental update functionality for Python files.

## Background
This ticket is part of Phase 1, Week 2, Task 3 of the LANG_PARSE project. After establishing Python symbol extraction (LANG_PARSE-1002) and import extraction (LANG_PARSE-1003), we need to persist this data into the Maproom database. This enables semantic search capabilities for Python codebases and ensures that Python symbols are indexed with appropriate metadata for accurate search and relationship tracking.

The database integration must handle Python's unique characteristics including decorators, async/await syntax, class inheritance relationships, and dynamic import patterns. This work builds upon the integration architecture established in LANG_PARSE-1005.

## Acceptance Criteria
- [ ] Symbol mapping correct - Python symbols (functions, classes, methods, variables) are correctly mapped to appropriate chunk_kind values in the database
- [ ] Python metadata stored - Python-specific metadata (decorators, async flag, class bases) is persisted in the chunks table
- [ ] Incremental updates working - When Python files change, only affected chunks are re-indexed without full repository re-scan
- [ ] Import edges created - chunk_edges table contains relationships based on Python import statements

## Technical Requirements
- Map Python symbols to chunk_kind enumeration:
  - Functions → chunk_kind::function
  - Classes → chunk_kind::class
  - Methods → chunk_kind::method (or function with class context)
  - Module-level variables → chunk_kind::variable
- Store Python-specific metadata in chunks.metadata JSONB field:
  - `decorators`: Array of decorator names/expressions
  - `is_async`: Boolean flag for async functions/methods
  - `class_bases`: Array of base class names for class definitions
  - `is_property`: Boolean flag for @property decorated methods
  - `is_classmethod`: Boolean flag for @classmethod decorated methods
  - `is_staticmethod`: Boolean flag for @staticmethod decorated methods
- Implement incremental update logic for Python files:
  - Detect file modifications via file hash or timestamp comparison
  - Delete existing chunks for modified files
  - Re-extract and insert updated chunks
  - Update chunk_edges for modified import relationships
- Create chunk_edges for Python import relationships:
  - `from module import symbol` → edge from current file to imported symbol
  - `import module` → edge from current file to module
  - Edge type should indicate import relationship

## Implementation Notes
### Database Write Integration
Update `crates/maproom/src/parser/python/extractor.rs` to include database write operations:
- Add database connection parameter to extraction functions
- After extracting symbols, insert into chunks table with appropriate chunk_kind
- Construct metadata JSON with Python-specific fields
- Handle errors gracefully with proper transaction rollback

### Incremental Update Strategy
- Store file content hash in files table
- On re-scan, compare current hash with stored hash
- If hash differs:
  1. Begin transaction
  2. Delete chunks WHERE file_id = ?
  3. Delete chunk_edges WHERE source_chunk_id IN (chunks from this file)
  4. Re-extract and insert new chunks
  5. Re-extract and insert new edges
  6. Update file hash
  7. Commit transaction

### Import Edge Creation
- For each import statement extracted in LANG_PARSE-1003:
  - Resolve imported symbol to chunk_id (if available in same repository)
  - Create edge in chunk_edges table
  - Store import type (direct import vs from import) in edge metadata

### Testing Approach
Create comprehensive integration tests in `crates/maproom/tests/integration/python_db_test.rs`:
- Test basic symbol insertion (function, class, method, variable)
- Test metadata persistence (decorators, async, bases)
- Test incremental updates (modify file, verify old chunks deleted, new chunks inserted)
- Test import edge creation (verify edges exist between importing and imported symbols)
- Test edge cases: nested classes, multiple decorators, complex inheritance

### Schema Considerations
Review existing schema in `crates/maproom/migrations/` to ensure:
- chunk_kind enum includes all necessary Python symbol types
- chunks.metadata JSONB can accommodate Python-specific fields
- chunk_edges table can represent import relationships
- If schema changes are needed, create a new migration file

## Dependencies
- **LANG_PARSE-1005** - Integration architecture must be established before Python-specific database writes can be implemented
- **LANG_PARSE-1002** - Python symbol extraction provides the symbols to be persisted
- **LANG_PARSE-1003** - Python import extraction provides the import relationships for edge creation
- Maproom database schema must support chunk_kind types and JSONB metadata

## Risk Assessment
- **Risk**: Schema changes may require migration and could affect existing data
  - **Mitigation**: Review schema carefully before implementation. If changes needed, create reversible migrations and test on sample data first

- **Risk**: Incremental update logic may miss edge cases leading to stale data
  - **Mitigation**: Comprehensive integration tests covering file modifications, deletions, and renames. Include test cases for boundary conditions

- **Risk**: Import edge resolution may fail for dynamic imports or external dependencies
  - **Mitigation**: Gracefully handle unresolved imports by logging warnings but continuing indexing. Store import strings even if target cannot be resolved

- **Risk**: Large Python files with many symbols may cause transaction timeouts
  - **Mitigation**: Use batch insertions with configurable batch size. Consider chunking very large files

## Files/Packages Affected
- `crates/maproom/src/parser/python/extractor.rs` - Add database write operations
- `crates/maproom/tests/integration/python_db_test.rs` - New integration tests (create this file)
- `crates/maproom/migrations/` - Potentially new migration file if schema changes required
- `crates/maproom/src/db/` - May need to update database layer if new query patterns needed
- `crates/maproom/src/parser/python/mod.rs` - Update module exports if needed
