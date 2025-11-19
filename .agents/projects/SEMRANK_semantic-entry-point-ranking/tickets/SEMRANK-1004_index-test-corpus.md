# Ticket: SEMRANK-1004: Index Test Corpus in Maproom

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Run maproom scan on test corpus, validate chunk metadata (kind, symbol_name) extracted correctly, verify relationships table populated.

## Background
The test corpus created in SEMRANK-1003 must be indexed to verify tree-sitter extraction is working correctly. This validation step confirms that:
1. Chunk kind enum values match the database schema ('func' not 'function')
2. symbol_name is extracted correctly for all functions, classes, components
3. The maproom indexer can extract semantic metadata needed for ranking

This ticket validates the test infrastructure portion of Phase 1 and ensures the test corpus is ready for baseline measurements in SEMRANK-1005.

## Acceptance Criteria
- [ ] Test corpus successfully indexed with `maproom scan` command
- [ ] All 30-50 chunks extracted and inserted into database
- [ ] Chunk metadata validated: kind values match database enum ('func', 'class', 'component', 'hook', 'module', 'var', 'type', 'other', 'heading_*')
- [ ] symbol_name extracted correctly for all functions, classes, components
- [ ] Relationships table populated (imports, calls, etc.) if available
- [ ] Query verification: `SELECT DISTINCT kind FROM maproom.chunks` returns expected values

## Technical Requirements
- **Indexing Command**: Use Rust maproom binary at `packages/cli/bin/<platform>/crewchief-maproom scan`
- **Scan Parameters**:
  - `--repo test-corpus`
  - `--worktree main`
  - `--path <corpus-path>`
  - `--commit HEAD`
- **Database Connection**: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- **Kind Enum Validation**: Verify kind enum matches `init.sql:44`: 'func','class','component','hook','module','var','type','other'
- **Markdown Headings**: Check that heading_1, heading_2, heading_3 values present for markdown files

## Implementation Notes
- Document the exact indexing command used
- Record chunk counts by kind (e.g., "15 func, 9 heading_1, 6 heading_2...")
- Verify symbol_name extraction for sample functions across all 3 languages
- Check relationships table if tree-sitter extracted them (imports, function calls)
- Save example SQL queries used for validation
- Create documentation of indexing process and results for future reference

## Dependencies
- SEMRANK-1003 (test corpus must exist)

## Risk Assessment
- **Risk**: Tree-sitter parsing failures during indexing
  - **Mitigation**: Test corpus syntax issues, fix in SEMRANK-1003
- **Risk**: Kind enum mismatch between tree-sitter and database
  - **Mitigation**: Would indicate indexer bug, escalate to Rust team
- **Risk**: Missing symbol names after extraction
  - **Mitigation**: Tree-sitter extraction issue, may need Rust indexer fixes

## Files/Packages Affected
- Database tables: `maproom.chunks`, `maproom.files`, `maproom.chunk_edges`
- Documentation file: Record indexing process and results
