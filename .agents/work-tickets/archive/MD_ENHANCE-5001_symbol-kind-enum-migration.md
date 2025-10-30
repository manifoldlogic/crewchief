# Ticket: MD_ENHANCE-5001: Add Missing symbol_kind Enum Values for Enhanced Parser

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (623/625 passed, 2 pre-existing failures unrelated)
- [x] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a database migration to add all missing `symbol_kind` enum values required by the enhanced markdown and multi-language parsers. The current database schema is incomplete and causes scan failures when the parser emits new symbol kinds.

## Background
During the MD_ENHANCE project (tickets 1001-4002), the parser was significantly enhanced to extract rich structural metadata from markdown files, including:
- Lists (`list`)
- Tables (`table`)
- Links (`link`, `image`)
- Enhanced code metadata

Additionally, multi-language support added Rust-specific symbols:
- `use`, `import`, `imports`
- `trait`, `impl`, `struct`, `enum`, `macro`
- `async_method`, `async_func`, `static`
- `constant`, `variable`, `method`
- Go-specific: `package`, `require`, `go_version`

However, the database enum `maproom.symbol_kind` was not updated to include these new values, causing scan failures with errors like:
```
ERROR: invalid input value for enum symbol_kind: "list"
ERROR: invalid input value for enum symbol_kind: "use"
```

This blocks indexing with the current worktree name and forces use of stale index data.

## Acceptance Criteria
- [x] Migration file created in `crates/maproom/migrations/` following existing naming convention
- [x] All missing symbol_kind values identified and added to the enum
- [x] Migration includes proper comments explaining each group of additions
- [x] Migration can be applied idempotently (handles values that may already exist)
- [x] `cargo run --bin crewchief-maproom -- scan` completes successfully after migration
- [x] No enum-related errors appear in scan output
- [x] Scanned data includes chunks with the new symbol kinds

## Technical Requirements
- Create migration file: `crates/maproom/migrations/0002_add_enhanced_symbol_kinds.sql`
- Use `ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'value';` syntax
- Add all missing values identified from parser code:

  **Markdown-specific:**
  - `list` - Markdown list items
  - `table` - Markdown tables
  - `link` - Markdown links
  - `image` - Markdown images

  **Rust-specific:**
  - `use` - Rust use statements
  - `import` - Import statements (multiple languages)
  - `imports` - Import blocks
  - `trait` - Rust traits
  - `impl` - Rust impl blocks
  - `struct` - Rust structs
  - `enum` - Rust enums
  - `macro` - Rust macros
  - `async_method` - Async methods
  - `async_func` - Async functions
  - `static` - Static items
  - `constant` - Constants
  - `variable` - Variables
  - `method` - Methods

  **Go-specific:**
  - `package` - Go package declarations
  - `require` - Go module requirements
  - `go_version` - Go version declarations

- Update migrations README if necessary to document this migration
- Consider adding a validation test that checks parser output against database enum

## Implementation Notes

### Migration File Structure
```sql
-- Migration 0002: Add enhanced symbol kinds for markdown and multi-language support
--
-- Context: MD_ENHANCE-1001 through MD_ENHANCE-4002 enhanced the parser to extract
-- rich structural metadata from markdown files and added comprehensive support for
-- Rust and Go. This migration adds the corresponding enum values to the database.

-- Markdown structural elements (MD_ENHANCE-2001, MD_ENHANCE-3001, MD_ENHANCE-3002)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'list';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'table';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'link';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'image';

-- Rust language support
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'use';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'import';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'imports';
-- ... etc
```

### Verification Strategy
After applying migration:
1. Run full scan: `cargo run --bin crewchief-maproom -- scan --repo crewchief --worktree <current-branch>`
2. Query database for new symbol kinds: `SELECT DISTINCT kind FROM maproom.chunks WHERE kind IN ('list', 'table', 'use', ...);`
3. Verify no enum errors in logs

### Alternative Approach (if needed)
If `IF NOT EXISTS` is not supported (Postgres < 9.1), use:
```sql
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_enum WHERE enumlabel = 'list') THEN
        ALTER TYPE maproom.symbol_kind ADD VALUE 'list';
    END IF;
END $$;
```

## Dependencies
- Requires PostgreSQL database with `maproom` schema already initialized
- Depends on completed MD_ENHANCE tickets (1001-4002) which added the parser enhancements
- Blocks: Ability to properly index current worktree with enhanced parser

## Risk Assessment
- **Risk**: Migration could fail if enum values were manually added during debugging
  - **Mitigation**: Use `IF NOT EXISTS` clause (Postgres 9.1+) or wrap in DO blocks with existence checks

- **Risk**: Enum modification requires exclusive lock on `chunks` table
  - **Mitigation**: Run during low-traffic period; enum additions are fast operations

- **Risk**: Missing additional enum values not yet discovered
  - **Mitigation**: Add comprehensive list from parser analysis; create follow-up ticket if more are found

## Files/Packages Affected
- `crates/maproom/migrations/0002_add_enhanced_symbol_kinds.sql` (new file)
- `crates/maproom/migrations/README.md` (documentation update)
- Database: `maproom.symbol_kind` enum type
- Indirectly affects: All scan operations, chunk storage, search functionality

## Implementation Notes

### Completed Tasks
1. ✅ Created migration file: `crates/maproom/migrations/0014_add_enhanced_symbol_kinds.sql`
   - Note: Used 0014 instead of 0002 as 0002 was already taken by markdown_support.sql
   - Comprehensive comments explaining each group of additions
   - Used `ALTER TYPE ... ADD VALUE IF NOT EXISTS` for idempotent operations
   - Added all required markdown symbols: list, table, link, image, image_link
   - Added all required Rust symbols: use, import, imports, trait, impl, struct, enum, macro, async_method, async_func, static, constant, variable, method
   - Added all required Go symbols: package, require, go_version

2. ✅ Registered migration in `crates/maproom/src/db/queries.rs`
   - Added migration to the `migrate()` function's migrations vector
   - Migration is now executed automatically when the Maproom service starts

3. ✅ Updated `crates/maproom/migrations/README.md`
   - Documented migration 0014 with complete list of symbol kinds
   - Explained purpose and context of the migration

4. ✅ Applied migration to database
   - All enum values successfully added (most already existed from debugging)
   - Migration handled existing values gracefully with IF NOT EXISTS

5. ✅ Verified scan completes successfully
   - Test scan indexed 587 files across 8 languages
   - Created 18,758 chunks total
   - Verified chunks with new symbol kinds exist:
     - Markdown: list (5,453), link (296), table (108), image_link (2)
     - Rust: use (1,021), impl (281), struct (261), method (94), async_method (70), enum (42), variable (30), async_func (28), constant (22), static (4), macro (3), trait (3), imports (30)
   - No enum-related errors in scan output

### Additional Finding
- Discovered `image_link` symbol kind used by parser (line 457 of parser.rs) that wasn't listed in original ticket
- Added to migration to ensure completeness
- This is for markdown images that are also hyperlinks

### Migration Safety
- All ALTER TYPE statements use IF NOT EXISTS for idempotency
- Safe to run multiple times without errors
- Works with PostgreSQL 9.1+ (IF NOT EXISTS support)
- No table locks required (enum additions are fast operations)

### Verification Evidence
```sql
-- Query confirmed all enum values exist:
SELECT enumlabel FROM pg_enum 
WHERE enumtypid = (
  SELECT oid FROM pg_type 
  WHERE typname = 'symbol_kind' 
  AND typnamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'maproom')
) 
ORDER BY enumsortorder;

-- Scan output confirmed successful indexing:
✅ Scan completed successfully!
   Files processed: 587
   Files skipped: 646
   Total chunks: 18758
   Total size: 5.84 MB
```
