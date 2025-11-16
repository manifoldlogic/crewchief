## Codebase Maintainability & Hardening Review

Execute a pragmatic maintainability assessment focusing on actionable improvements that AI agents can autonomously implement. Generate concrete tickets following VG-{phase}-{ticket}.md pattern.

### Phase 1: Architecture Assessment (ANALYZE)
**Output: MAINTAINABILITY_ANALYSIS.md**

1. **Structural Health Check**
   - Module boundaries and coupling patterns
   - Knowledge graph coherence for AI agent navigation
   - CLAUDE.md coverage and context scoping effectiveness
   - Monorepo organization and dependency management

2. **AI Agent Workability Score**
   - Code predictability and consistency patterns
   - Self-documenting code structures
   - Clear input/output contracts
   - Testability without human intervention

### Phase 2: Critical Path Analysis (PRIORITIZE)
**Output: MAINTAINABILITY_ARCHITECTURE.md**

3. **Technical Debt Triage**
   - High-risk code sections (complex + frequently modified)
   - Missing test coverage on critical paths
   - Fragile integrations requiring human oversight
   - Version drift and deprecated dependencies

4. **Refactoring Opportunities**
   - Extract reusable patterns for AI agent templates
   - Consolidate duplicate logic
   - Simplify overly complex functions (cyclomatic complexity > 10)
   - Standardize error handling and logging

### Phase 3: Actionable Improvements (IMPLEMENT)
**Output: MAINTAINABILITY_PLAN.md with VG-MAINT-NNN tickets**

5. **Automated Quality Gates**
   - Pre-commit hooks for AI agent work validation
   - Test coverage thresholds per module
   - Linting rules tuned for AI-generated code
   - Documentation generation from code structure

6. **AI Agent Enablement**
   - Create/update SKILL.md files for domain-specific operations
   - Establish clear ticket completion criteria
   - Define automated verification steps
   - Document edge cases and gotchas in agent-readable format

### Deliverables Format

For each identified issue, create a ticket with:
- **Impact**: Business/technical risk if unaddressed
- **Effort**: S/M/L for AI agent implementation
- **Dependencies**: Blocking relationships
- **Verification**: Automated test criteria
- **Implementation hint**: One-liner for AI agent context

### Constraints

- Focus on changes that can be implemented autonomously by AI agents
- Prioritize improvements that reduce future human intervention
- Skip ceremonial documentation in favor of executable specifications
- Maximum 10 high-priority tickets in initial batch
- Each ticket must be completable in under 4 hours of AI agent time

### Skip Unless Critical

- Minor style inconsistencies
- Comprehensive documentation (beyond AI agent needs)
- Performance optimizations without measured impact
- Security theater (focus on actual vulnerabilities)
- Process documentation

Generate the three deliverables and initial ticket batch ready for immediate AI agent assignment.

