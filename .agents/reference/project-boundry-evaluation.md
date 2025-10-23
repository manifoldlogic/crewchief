
# Project Boundary Framework for Agent-Based Development

## The Stable Context Triangle

At the core of effective agent-based development is defining the right project boundaries. A well-bounded project creates a **stable context** where AI agents can operate effectively without confusion or inconsistency.

```
        🔒 Interface Stability
              ╱     ╲
             ╱       ╲
            ╱         ╲
     📦 Context   🎯 Testable
      Coherence     Completion
            ╲         ╱
             ╲       ╱
              ╲     ╱
         ✅ Valid Project
```

## The Three Core Criteria

### 1. Interface Stability 🔒
**The Golden Rule:** All external interfaces must remain stable throughout the project lifecycle.

**What This Means:**
- APIs your project calls won't change mid-development
- Data formats and schemas are locked before work begins
- Integration contracts with other systems are finalized
- No renegotiation of boundaries during implementation

**Why It's Critical:**
AI agents cannot adapt to changing interfaces the way humans can. When an interface changes, every ticket becomes invalid, context becomes confused, and agents start producing inconsistent code. This is the #1 cause of agent-driven development failure.

**How to Verify:**
- Document all external dependencies
- Get commitment from interface owners
- Create interface mocks/stubs upfront
- If interfaces might change, they must be wrapped in an abstraction layer

### 2. Context Coherence 📦
**The Memory Rule:** The entire project must fit within an agent's working memory.

**What This Means:**
- The complete project can be explained in 2-3 paragraphs
- There are fewer than 20 key domain concepts
- All work references the same 5-10 core modules/files
- Agents can understand the whole when working on any part

**Why It's Critical:**
Agents need the full context to make consistent decisions. Too broad, and they lose track of the overall design. Too narrow, and they lack context for good architectural choices.

**How to Verify:**
- Write a project summary in <500 words
- List all domain entities (should be <20)
- Map file dependencies (should cluster tightly)
- Test: Could a new developer understand the project from your docs?

### 3. Testable Completion 🎯
**The Verification Rule:** Success must be programmatically verifiable.

**What This Means:**
- Clear, measurable acceptance criteria
- Automated tests can determine completion
- No subjective "feel good" requirements
- Binary pass/fail for the entire project

**Why It's Critical:**
Your validation agents need to programmatically determine if work is complete. Fuzzy requirements break the entire verification chain that prevents false completion claims.

**How to Verify:**
- Write acceptance tests before starting
- Each criterion has a specific test
- Success is deterministic, not subjective
- A script could determine project completion

---

## Secondary Criteria

While the three core criteria are essential, these additional factors significantly improve agent effectiveness:

### Architectural Cohesion
Changes should cluster in one area of the system architecture. While not mandatory, scattered changes make it harder for agents to maintain consistency.

### Domain Unity
The project should operate within a single domain language. Mixing multiple domains (e.g., "billing" + "user management" + "reporting") creates context confusion.

### Independent Value
The project should deliver standalone value, not require other projects to be useful. This ensures clear scoping and prevents dependency cascades.

---

## Project Boundary Patterns

### Pattern 1: Feature Journey
**Structure:** Complete user workflow from entry to completion

**Good Examples:**
- Customer checkout process
- User onboarding flow
- Report generation system

**Boundaries:**
- Entry point: Where user starts
- Exit point: Where user completes goal
- Scope: All steps in between

### Pattern 2: Service Module
**Structure:** Autonomous service with clear API boundaries

**Good Examples:**
- Authentication service
- Payment processing module
- Notification engine

**Boundaries:**
- Inbound: API endpoints
- Outbound: External service calls
- Scope: All internal logic

### Pattern 3: Capability Layer
**Structure:** Horizontal functionality across system

**Good Examples:**
- Caching implementation
- Audit logging system
- Error handling framework

**Boundaries:**
- Touch points: Where it integrates
- Abstraction: How it's accessed
- Scope: Full implementation

---

## Quick Decision Tests

### The New Agent Test
*"If I gave this project to a fresh agent halfway through, could it understand what's being built?"*
- ✅ Yes → Good boundary
- ❌ No → Too broad or poorly defined

### The Interface Churn Test
*"Will I need to rewrite tickets if external systems change?"*
- ❌ Yes → Bad boundary, stabilize interfaces first
- ✅ No → Good boundary

### The Context Overflow Test
*"Can I fit the essential project context in a single conversation?"*
- ✅ Yes → Good boundary
- ❌ No → Too large, split it

### The Script Completion Test
*"Could a script determine if this project is complete?"*
- ✅ Yes → Good boundary
- ❌ No → Success criteria too vague

---

## Boundary Evaluation Checklist

```yaml
## Core Requirements (All Required)
interface_stability:
  ☐ All external APIs documented
  ☐ Data formats finalized
  ☐ Integration points stable
  ☐ No expected interface changes

context_coherence:
  ☐ Project explainable in <500 words
  ☐ Less than 20 domain concepts
  ☐ Tightly clustered codebase
  ☐ Single area of architecture

testable_completion:
  ☐ Measurable success criteria
  ☐ Automated test suite possible
  ☐ Binary pass/fail determination
  ☐ No subjective requirements

## Secondary Factors (Nice to Have)
architectural_fit:
  ☐ Single service/module
  ☐ Clear component boundaries
  ☐ Minimal cross-cutting concerns

practical_sizing:
  ☐ 2-6 week implementation
  ☐ 10-50 tickets
  ☐ Single team ownership
  ☐ Independent deployment
```

---

## Common Anti-Patterns

### ❌ The Time Box
**Example:** "Q1 2025 Features"  
**Problem:** No architectural coherence, no unified context  
**Fix:** Group by capability, not calendar

### ❌ The Kitchen Sink
**Example:** "Customer Portal Improvements"  
**Problem:** Too broad, interfaces keep changing  
**Fix:** Break into specific workflows

### ❌ The Fragment
**Example:** "Add search button"  
**Problem:** Insufficient context for good decisions  
**Fix:** Bundle into meaningful feature set

### ❌ The Scatter Shot
**Example:** "Performance optimizations"  
**Problem:** No unified architecture or testing  
**Fix:** Focus on specific subsystem performance

---

## Project Definition Template

```markdown
# Project: [NAME]

## Project Summary
[2-3 paragraph description that any developer could understand]

## Core Criteria Assessment

### Interface Stability
**External Interfaces:**
- API: [Name, version, status]
- Database: [Schema, locked?]
- Services: [Dependencies, contracts]

**Stability Commitment:** ✅/❌
**Risk Areas:** [Any interfaces that might change]

### Context Coherence
**Domain Concepts:** [Count: X]
- Concept 1
- Concept 2
- [List all]

**Core Modules:**
- Module 1: [Purpose]
- Module 2: [Purpose]
- [Should be <10]

**Context Size:** [Estimated tokens/words]

### Testable Completion
**Success Criteria:**
- [ ] Criterion 1 [How to test]
- [ ] Criterion 2 [How to test]
- [ ] Criterion 3 [How to test]

**Verification Method:** [How to determine success]

## Scope Definition

### In Scope
- Specific features/capabilities
- Architectural components
- User workflows

### Out of Scope
- Explicitly excluded items
- Future phase work
- External dependencies

### Edge Cases
- Boundary condition 1: [Decision]
- Boundary condition 2: [Decision]

## Risk Assessment
| Risk | Impact on Agents | Mitigation |
|------|-----------------|------------|
| Interface might change | High - invalidates tickets | Create abstraction layer |
| Context too large | Medium - agent confusion | Split into sub-projects |
| Vague requirements | High - can't verify | Define specific tests |
```

---

## Examples Evaluated

### ✅ Good: Payment Processing System
- **Interface Stability:** Payment gateway APIs documented and versioned
- **Context Coherence:** 12 domain concepts, all payment-related
- **Testable Completion:** Can process payments, handle failures, reconcile

### ❌ Bad: Improve Application Performance  
- **Interface Stability:** Touches everything, interfaces undefined
- **Context Coherence:** Scattered across entire codebase
- **Testable Completion:** "Better" is not measurable

### ✅ Good: User Authentication Service
- **Interface Stability:** Standard OAuth/JWT, stable protocols
- **Context Coherence:** 8 concepts, all auth-related
- **Testable Completion:** Login works, tokens refresh, logout succeeds

---

## Implementation Guide

### Step 1: Define Interfaces
Before anything else, lock down every external interface:
1. Document all APIs (versions, endpoints, schemas)
2. Define data formats (JSON schemas, DB tables)
3. Get commitment on stability from owners
4. Create mocks/stubs for development

### Step 2: Bound the Context
Ensure agents can hold the full picture:
1. Write project summary (<500 words)
2. List and count domain concepts (<20)
3. Map module dependencies
4. Verify tight clustering

### Step 3: Define Success
Make completion binary and measurable:
1. Write specific acceptance criteria
2. Create test plan for each criterion
3. Automate verification where possible
4. Ensure no subjective measures

### Step 4: Validate the Boundary
Run through the decision tests:
- New Agent Test ✓
- Interface Churn Test ✓
- Context Overflow Test ✓
- Script Completion Test ✓

If any test fails, adjust boundaries before proceeding.

---

## Adapting Existing Work

### If Your Project Is Too Large
**Symptoms:** Agents getting confused, inconsistent implementations

**Solutions:**
1. Find natural phase boundaries
2. Split along architectural seams
3. Create stable interfaces between phases
4. Each phase becomes its own project

### If Your Project Is Too Small
**Symptoms:** Excessive overhead, agents lack context

**Solutions:**
1. Bundle related features
2. Expand to complete user journey
3. Include full vertical slice
4. Group by deployment unit

### If Interfaces Aren't Stable
**Symptoms:** Constant ticket rewrites, agent confusion

**Solutions:**
1. Create abstraction layer
2. Delay project until interfaces stabilize
3. Mock unstable interfaces
4. Reduce scope to stable portions only

---

## Key Insights

1. **Interface stability is non-negotiable.** It's better to delay a project than start with unstable interfaces.

2. **Context coherence determines agent effectiveness.** If agents can't understand the whole, they can't make good decisions on parts.

3. **Testable completion prevents drift.** Without clear verification, projects never truly complete.

4. **These boundaries are different from human-driven development.** What works for human teams may fail for agent orchestration.

5. **Start strict, then relax.** It's easier to expand boundaries than to contract them mid-project.