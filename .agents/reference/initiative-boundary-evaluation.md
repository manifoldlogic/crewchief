# Initiative Boundary Evaluation

The Role of an Initiative in Agent-Based Development

An initiative represents a higher-order context from which one or more projects emerge.
It is the strategic synthesis layer — where discovery, research, and analysis converge into a coherent mission that can be decomposed into multiple stable, agent-executable projects.

If a project defines a stable context for execution,
an initiative defines a stable frame for meaning.

            🌍 Conceptual Stability
                  ╱        ╲
                  ╱          ╲
                ╱            ╲
      🧠 Domain Coherence   🎯 Directional Clarity
                ╲            ╱
                  ╲          ╱
                  ╲        ╱
                ✅ Valid Initiative

⸻

The Three Core Criteria

1. Conceptual Stability 🌍

The Golden Rule: The initiative must define a stable problem space, not a moving target.

What This Means:
• The core vision and value proposition are defined and will not pivot midstream.
• The “why” of the initiative remains constant even if the “how” evolves.
• Documents under the initiative reference the same conceptual frame of purpose.

Why It’s Critical:
Without conceptual stability, downstream projects fragment into incompatible worldviews. Agents lose alignment on the initiative’s underlying logic, resulting in incoherent project definitions and competing interpretations of value.

How to Verify:
• Write a concise initiative vision statement (<150 words).
• Ensure all reference materials reinforce that same vision.
• Check that differences between documents are about approaches, not definitions.

⸻

2. Domain Coherence 🧠

The Scope Rule: All projects derived from an initiative should live in a single conceptual domain.

What This Means:
• The initiative operates within a shared ontology — one domain language.
• The problems addressed are tightly related and reference the same entities.
• Projects differ by implementation boundary, not by world.

Why It’s Critical:
An initiative that spans multiple unrelated domains (“payment infra,” “user growth,” “documentation platform”) creates incoherent project decomposition.
Agents cannot cluster related documents effectively or build a unified model of value.

How to Verify:
• Identify the core domain concepts (<30 total).
• Test: Can they all be described in one system diagram?
• If multiple ontologies appear, split the initiative before proceeding.

⸻

3. Directional Clarity 🎯

The Navigation Rule: The initiative defines where we’re going, not how to get there.

What This Means:
• There is a clear desired end state or transformation.
• The path to reach it is open for exploration and decomposition.
• Each project should move the system measurably closer to this end state.

Why It’s Critical:
Agents need a directional compass when proposing or ordering projects. Without clarity of direction, they cannot rationally prioritize or derive dependency order.

How to Verify:
• Write a single measurable outcome statement (“When this initiative succeeds, X will be true.”)
• List 3–5 success signals observable across projects.
• Ensure none of these require defining implementation details yet.

⸻

Secondary Criteria

Temporal Elasticity

Initiatives typically span multiple weeks to months, not days. Their work should survive the lifecycle of individual projects. If your “initiative” completes in two weeks, it’s probably just a large project.

Research Completeness

An initiative should be backed by sufficient discovery materials — research, analysis, or architecture sketches. If you’re inventing the context as you go, it’s too early to formalize it as an initiative.

Cross-Project Synergy

Each project under the initiative should reinforce others, not compete with or invalidate them.
Redundancy or orthogonal goals are warning signs of poor initiative definition.

⸻

Initiative Boundary Patterns

Pattern 1: Strategic Transformation

Structure: A major shift in system architecture, workflow, or product direction
Examples:
• “Migrate to fully agent-managed CI/CD”
• “Adopt federated identity across all products”

Boundaries:
• Shared transformation goal
• Projects: platform redesigns, adapters, and migrations

⸻

Pattern 2: Capability Expansion

Structure: Introduce a new horizontal or vertical capability to the ecosystem
Examples:
• “Add AI-driven insights to all dashboards”
• “Implement self-healing infrastructure”

Boundaries:
• One capability theme
• Multiple projects: service module, UX, observability, integration

⸻

Pattern 3: Domain Consolidation

Structure: Merge fragmented or redundant systems into a unified domain
Examples:
• “Unify notification systems”
• “Centralize data access layer”

Boundaries:
• Common conceptual core
• Projects: API consolidation, data normalization, service retirement

⸻

Quick Decision Tests

The Vision Drift Test

“Would new discoveries change the initiative’s purpose?”
• ✅ No → Good boundary
• ❌ Yes → Define narrower scope or rephrase purpose

The Domain Split Test

“Can all derived projects share one domain model?”
• ✅ Yes → Good boundary
• ❌ No → Split initiative

The Direction Ambiguity Test

“Could an agent sequence projects logically toward the goal?”
• ✅ Yes → Clear direction
• ❌ No → Goal too vague or conflicting

⸻

Initiative Evaluation Checklist

## Core Requirements (All Required)

conceptual_stability:
  ☐ Stable definition of purpose
  ☐ Core value proposition consistent across docs
  ☐ No competing problem statements

domain_coherence:
  ☐ Common ontology / domain language
  ☐ <30 domain entities total
  ☐ All subtopics fit one conceptual model

directional_clarity:
  ☐ Measurable end state
  ☐ 3–5 observable success signals
  ☐ Projects can be ordered toward outcome

## Secondary Factors (Recommended)

temporal_elasticity:
  ☐ Expected duration >1 month
  ☐ Survives multiple project cycles

research_completeness:
  ☐ Includes discovery materials
  ☐ Sufficient context for decomposition

cross_project_synergy:
  ☐ Projects reinforce shared goals
  ☐ No conflicting deliverables

⸻

Common Anti-Patterns

❌ The Wishlist

Example: “All the cool features we want next year”
Problem: No conceptual or directional unity
Fix: Group by capability or transformation

❌ The Committee Scope

Example: “Everything Marketing requested”
Problem: Multiple unrelated domains and purposes
Fix: Split into domain-specific initiatives

❌ The Moving Target

Example: “Whatever improves retention this quarter”
Problem: Goal pivots with metrics, causing project churn
Fix: Define a stable conceptual theory of improvement first

❌ The Project-in-Disguise

Example: “Build the API Gateway”
Problem: Single concrete deliverable — just a project, not an initiative
Fix: Move directly to /create-project

⸻

Initiative Definition Template

# Initiative: [NAME]

## Vision Statement

[Brief, stable description of the purpose and long-term goal]

## Conceptual Frame

[Define the problem space, context, and why this initiative exists]

## Domain Coherence

**Core Domain Concepts (≤30):**

- Concept 1
- Concept 2
- ...

## Directional Clarity

**Desired End State:**  
“When this initiative succeeds, [X] will be true.”

**Success Signals:**

- [ ] Signal 1
- [ ] Signal 2
- [ ] Signal 3

## Derived Projects

(List to be generated by `/create-initiative`)

## Risks

| Risk | Impact | Mitigation |
|------|---------|-------------|
| Concept drift | Projects lose alignment | Define fixed purpose statement |
| Domain confusion | Context overlap | Separate initiative domains |
| Vague goal | Agents can’t order work | Define measurable end state |

⸻

Key Insights

1. Initiatives create meaning; projects create outcomes.
2. The initiative boundary is philosophical, not technical — it defines coherence, not code.
3. Initiatives should be big enough to unify multiple projects, small enough to remain cognitively stable.
4. Once decomposition begins, initiative boundaries must remain fixed until completion.
5. A stable initiative is the highest leverage point in agent-based orchestration — it shapes every project downstream.
