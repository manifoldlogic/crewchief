// filename based on SLUG-0001_ticket-name.md where SLUG is the project slug
// Ticket numbers are prefaced with their phase number
// e.g., phase 2 starts with SLUG-2001_ticket-name.md

# Ticket: [TICKET-ID]: [Title]

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
- [primary-task-agent-name]
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
[Brief description of what needs to be done]

## Background
[Context and why this work is needed]
[Reference the specific section/feature from <SLUG>_PLAN.md this ticket implements]

## Acceptance Criteria
- [ ] [Specific measurable outcome 1]
- [ ] [Specific measurable outcome 2]
- [ ] [Specific measurable outcome 3]

## Technical Requirements
- [Requirement 1]
- [Requirement 2]
- [Requirement 3]

## Implementation Notes
[Technical details, approach, considerations]

## Dependencies
- [List any prerequisite tickets or work]
- [External dependencies]

## Risk Assessment
- **Risk**: [Description]
  - **Mitigation**: [How to handle]

## Files/Packages Affected
- [List of files or packages that will be modified]