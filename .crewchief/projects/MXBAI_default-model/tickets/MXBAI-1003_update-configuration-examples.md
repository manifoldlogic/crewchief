# Ticket: [MXBAI-1003]: Update Configuration Examples

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
- documentation-writer
- verify-ticket
- commit-ticket

## Summary
Update .env.example file to reflect new default model and dimension values for mxbai-embed-large.

## Background
This ticket implements part of Decision 7 from architecture.md. The .env.example file serves as the primary reference for users configuring the system. It must show the current defaults to avoid confusion.

Reference: plan.md Phase 1, Deliverable 3 "Update Configuration Examples"

## Acceptance Criteria
- [ ] MAPROOM_EMBEDDING_MODEL example value changed from nomic-embed-text to mxbai-embed-large
- [ ] MAPROOM_EMBEDDING_DIMENSION example value changed from 768 to 1024
- [ ] Comment added explaining backward compatibility
- [ ] File remains valid .env format

## Technical Requirements
- Modify `crates/maproom/.env.example` line 38:
  - Change: `MAPROOM_EMBEDDING_MODEL=nomic-embed-text`
  - To: `MAPROOM_EMBEDDING_MODEL=mxbai-embed-large`

- Modify `crates/maproom/.env.example` line 44:
  - Change: `MAPROOM_EMBEDDING_DIMENSION=768`
  - To: `MAPROOM_EMBEDDING_DIMENSION=1024`

- Add comment above MAPROOM_EMBEDDING_MODEL explaining backward compatibility:
  ```bash
  # Default model (mxbai-embed-large provides better quality than nomic-embed-text)
  # To use nomic-embed-text, set: MAPROOM_EMBEDDING_MODEL=nomic-embed-text and MAPROOM_EMBEDDING_DIMENSION=768
  MAPROOM_EMBEDDING_MODEL=mxbai-embed-large
  ```

## Implementation Notes
**Purpose of .env.example**:
- Shows users recommended/default configuration
- Serves as template for creating .env file
- Documents available environment variables

**Pattern to follow**:
- Keep file format consistent (.env syntax)
- Add helpful comments
- Show defaults that match code constants

**What NOT to change**:
- Other environment variable examples
- File structure or organization
- Variable names

## Dependencies
- **Logical dependency**: MXBAI-1001, MXBAI-1002 (code defaults should be updated first)
- **External dependency**: None

## Risk Assessment
- **Risk**: Invalid .env syntax
  - **Mitigation**: Review file format, ensure = signs and no quotes where not needed

- **Risk**: Confusing documentation
  - **Mitigation**: Add clear comment explaining backward compatibility option

## Files/Packages Affected
- `/workspace/crates/maproom/.env.example` (lines 38, 44, and new comment)

## Verification Notes
Tests pass: N/A (configuration example file, not executable code)

verify-ticket agent should check:
- [ ] Both example values updated correctly
- [ ] Backward compatibility comment added and clear
- [ ] File maintains valid .env format
- [ ] No other unintended changes to .env.example
- [ ] Comments are helpful and accurate
