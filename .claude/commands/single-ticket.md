---
argument-hint: [TICKET_ID]
description: Complete, verify, and commit a ticket following the full implementation workflow
---

# Context

Ticket ID: $ARGUMENTS
Location: `.agents/projects/{SLUG}_*/tickets/`

# Task

Execute complete ticket workflow: implementation → verification → commit

## Phase 1: Locate Ticket

1. Search for ticket ID across all project folders in `.agents/projects/`
2. Identify project SLUG and full ticket path
3. Read ticket file to understand:
   - Acceptance criteria
   - Technical requirements
   - Implementation notes
   - Assigned agents
   - Dependencies
   - Files/packages affected

## Phase 2: Implementation

Use agents specified in ticket's "Agent Assignments" section:

**Primary agent:** Implements core functionality according to technical requirements

**Supporting agents:** Handle specialized aspects (database, integration, etc.)

**Implementation checklist:**
- ✓ Review all acceptance criteria before starting
- ✓ Check dependencies are completed
- ✓ Follow implementation notes and architecture guidance
- ✓ Create/modify files listed in ticket
- ✓ Write code that meets technical requirements
- ✓ Add inline documentation for complex logic
- ✓ Consider edge cases and error handling

**Working principles:**
- Focus on ticket scope only (no feature creep)
- Follow project conventions from CLAUDE.md
- Implement cleanly for future maintainability
- Ask clarifying questions if requirements are ambiguous

## Phase 3: Verification

Delegate to `verify-ticket` agent with ticket ID and implementation summary:

**Verify-ticket will check:**
- All acceptance criteria are met
- Technical requirements are satisfied
- Tests pass (if applicable)
- Code quality standards maintained
- No unintended side effects
- Documentation is adequate

**Verification outcomes:**
- **Pass:** Proceed to commit phase
- **Fail:** Address issues, then re-verify

## Phase 4: Commit

Delegate to `commit-ticket` agent with ticket ID:

**Commit-ticket will:**
- Stage all changed files
- Generate descriptive commit message referencing ticket ID
- Commit changes with proper attribution

## Quality Gates

Before moving to next phase:

**After Implementation:**
- Code compiles/runs without errors
- Manual testing shows expected behavior
- All files from ticket are addressed

**After Verification:**
- All acceptance criteria explicitly confirmed
- No failing tests
- verify-ticket agent provides clear pass

**After Commit:**
- Changes are committed with proper message
- Ticket status updated
- No uncommitted changes remaining

## Error Handling

If issues arise at any phase:
1. Document the specific problem
2. Determine root cause
3. Address issue or clearly indicate if blocked
4. Re-run verification after fixes
5. Do not commit until verification passes

## Output

Provide summary at completion:
- Ticket ID and title
- Implementation approach used
- Verification results
- Commit SHA and message
- Any notes or follow-up items

Work systematically through all phases to ensure complete, verified, and committed work.