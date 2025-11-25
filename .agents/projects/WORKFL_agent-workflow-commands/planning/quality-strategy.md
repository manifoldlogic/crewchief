# Quality Strategy: Workflow Commands

## 1. Testing
- **Unit Tests**: Verify the scaffolding logic creates the correct files and content.
- **Integration Tests**: Run `crewchief project init` and verify file system state.

## 2. Validation
- The CLI should strictly validate SLUG formats (uppercase, max 8 chars).
- It should prevent overwriting existing projects without force flag.

## 3. Acceptance Criteria
- [ ] `crewchief project init` creates all 5 planning docs.
- [ ] `crewchief project list` detects the new project.
- [ ] Directories follow the naming convention strictly.

