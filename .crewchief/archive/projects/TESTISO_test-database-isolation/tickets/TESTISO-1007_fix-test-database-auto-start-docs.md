# Ticket: TESTISO-1007: Fix Incorrect Documentation About Test Database Auto-Starting

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-implementation
- verify-ticket
- commit-ticket

## Summary
Fix misleading documentation that incorrectly states both dev and test databases start automatically when running `setup`. Only the dev database auto-starts; the test database must be started manually to maintain its opt-in nature.

## Background
The documentation created in TESTISO-1006 contains a critical error: it states that "Both databases start automatically when you run `setup`" (README.md line 216) and "start automatically" (TEST_DATABASE_SETUP.md line 31). This is incorrect and misleading.

**Actual behavior:**
- The `setup` command only starts the dev database automatically via maproom-mcp's `depends_on` in docker-compose.yml
- The test database (`postgres-test` service) is a separate service that must be started manually
- This opt-in design is intentional: regular maproom users don't need the test database running

**Impact:**
Developers following the documentation will expect the test database to be running after `setup`, which leads to confusion when tests fail with connection errors.

This ticket implements a documentation-only fix as identified in the post-implementation review.

## Acceptance Criteria
- [x] README.md "Database Setup" section clarifies that only dev database auto-starts
- [x] README.md explicitly documents how to manually start test database
- [x] TEST_DATABASE_SETUP.md "Overview" section corrects auto-start language
- [x] TEST_DATABASE_SETUP.md "Common Workflows" section clarifies manual test database startup
- [x] All language implying "both databases start together" is removed
- [x] Opt-in nature of test database is clearly stated (for developers/CI who need test isolation)
- [x] Documentation maintains consistency between README.md and TEST_DATABASE_SETUP.md

## Technical Requirements

### Files to Update

**1. packages/maproom-mcp/README.md**

**Lines 208-227 (Database Setup section):**
- Replace "Both databases start automatically" (line 216) with clear statement that only dev database auto-starts
- Add explicit note that test database is opt-in and must be started manually
- Document manual test database startup command: `cd packages/maproom-mcp/config && docker compose up -d postgres-test`
- Keep existing "Running Tests" section (lines 229-238) as it's already correct

**Example correction:**
```markdown
### Starting Databases

The `setup` command starts the **development database only** (via automatic `depends_on` in docker-compose.yml):

```bash
npx @crewchief/maproom-mcp setup --provider=openai
```

**For developers/CI needing test isolation**, the test database must be started manually (opt-in):

```bash
cd packages/maproom-mcp/config  # or ~/.maproom-mcp
docker compose up -d postgres-test
```

Regular maproom users don't need the test database running.
```

**2. docs/development/TEST_DATABASE_SETUP.md**

**Lines 21-35 (Overview section):**
- Update table or text to clarify dev database auto-starts, test database is manual
- Emphasize opt-in design for test database

**Lines 103-167 (Common Workflows section):**
- Before "Running Tests" subsection, add "Starting Test Database" subsection
- Document that test database must be started manually before running tests
- Clarify that `pnpm test` assumes test database is already running
- Keep existing workflows intact, just add prerequisite documentation

**Example addition:**
```markdown
### Starting Test Database

The test database is **opt-in** and must be started manually before running tests:

```bash
cd packages/maproom-mcp/config  # or ~/.maproom-mcp
docker compose up -d postgres-test

# Wait for healthy status
docker compose ps | grep postgres-test

# Initialize schema (first time only)
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < config/init.sql
```

This design is intentional: regular maproom users don't need the test database running.

### Running Tests
...
```

## Implementation Notes

### Key Messaging Points

1. **Clarity about auto-start scope**: `setup` only starts dev database via `depends_on`
2. **Opt-in nature**: Test database is for developers/CI who need test isolation
3. **Manual startup required**: Explicit command to start test database
4. **Regular users don't need it**: Emphasize test database is optional for typical usage

### Consistency Requirements

- Both documents must present the same information
- Avoid contradictory statements between files
- Use consistent terminology ("opt-in", "manual", "auto-start")
- Maintain parallel structure in both docs

### Validation

After changes, verify:
1. No language suggesting "both databases start together"
2. Manual startup command is present and correct in both files
3. Opt-in nature is clearly stated
4. Development workflow (setup → scan → search) doesn't imply test database is needed

## Dependencies

None - this is a documentation-only fix.

## Risk Assessment

- **Risk**: Minimal - documentation-only change, no code changes
  - **Mitigation**: Review for clarity and consistency before committing

- **Risk**: Developers may not read updated docs and still expect auto-start
  - **Mitigation**: Clear messaging in both README and guide, emphasize "opt-in" nature

## Files/Packages Affected

- `packages/maproom-mcp/README.md` (Database Setup section, lines 208-260)
- `docs/development/TEST_DATABASE_SETUP.md` (Overview and Common Workflows sections)
