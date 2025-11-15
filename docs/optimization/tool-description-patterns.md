# Tool Description Patterns for AI Agents

A practical guide for writing effective tool descriptions based on genetic optimization research.

**Last Updated**: 2025-11-15
**Based On**: 10-generation genetic optimization experiment with 58 variants
**Performance Improvement**: +10.7% agent success rate (17.7% → 19.6%)

---

## Table of Contents

1. [Overview](#overview)
2. [Winning Patterns](#winning-patterns)
3. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
4. [How-To Guide](#how-to-guide)
5. [Before/After Examples](#beforeafter-examples)
6. [Decision Framework](#decision-framework)
7. [References](#references)

---

## Overview

### What Makes Tool Descriptions Effective for AI Agents

AI agents don't read tool descriptions the same way humans do. Research shows agents respond best to:

1. **Procedural transformation workflows** - Step-by-step instructions for converting user intent into tool inputs
2. **Before→After examples** - Visual transformations showing INPUT → OUTPUT
3. **Imperative commands** - "Extract...", "Remove...", "Try..." instead of "Best for..." or "Think about..."
4. **Complete lifecycle coverage** - From transformation through execution to recovery
5. **Focused scope** - 90% teaching THIS tool, 10% mentioning alternatives

### Key Research Finding

The single biggest differentiator between high-performing (19.6%) and low-performing (17.7%) tool descriptions was the presence of a **systematic transformation workflow** teaching agents HOW to convert natural language questions into optimal tool inputs.

**Impact**: +1.9 percentage points absolute improvement (+10.7% relative)

---

## Winning Patterns

### Pattern 1: Transformation Workflow Template

**What It Is**: A numbered, step-by-step process teaching agents how to transform user questions into tool inputs.

**Copy-Paste Template**:

```markdown
🤖 AI AGENT [INPUT TYPE] → [OUTPUT TYPE] TRANSFORMATION:

Transform [input description] into optimal [output description]:

TRANSFORMATION PATTERNS:
1. Extract [what to extract]
2. Remove: [list of words/patterns to remove]
3. Prefer [guidance on selection/prioritization]
4. [Optional 4th step if needed]

EXAMPLES:
  "[Example input 1]" → "[Example output 1]"
  "[Example input 2]" → "[Example output 2]"
  "[Example input 3]" → "[Example output 3]"
  "[Example input 4]" → "[Example output 4]"
```

**Concrete Example** (Semantic Search Tool):

```markdown
🤖 AI AGENT QUERY FORMULATION:

Transform natural language questions into optimal search queries:

TRANSFORMATION PATTERNS:
1. Extract 2-3 core technical terms
2. Remove: how, what, where, when, why, does, is, are, the, a, an
3. Prefer code-like terminology

EXAMPLES:
  "How does authentication work?" → "authentication"
  "What handles errors?" → "error handler"
  "Find auth logic" → "authentication"
  "Where is WebSocket disconnect?" → "WebSocket disconnect"
```

**Why It Works**:
- Provides explicit, repeatable procedure
- Shows both INPUT type and OUTPUT type
- Removes ambiguity about transformation process
- Uses imperative commands (Extract, Remove, Prefer)
- Visual arrows (→) create clear before/after mapping

**When to Use**: Any tool where user intent must be transformed into specific API inputs (search queries, file paths, filter criteria, etc.)

---

### Pattern 2: AI Agent Section Template

**What It Is**: Using the 🤖 emoji to mark sections specifically for AI agents, creating audience targeting.

**Copy-Paste Template**:

```markdown
🤖 AI AGENT [SPECIFIC GUIDANCE TYPE]:

[Procedural instructions here]
```

**Concrete Examples**:

```markdown
🤖 AI AGENT QUERY FORMULATION:
[transformation workflow]

🤖 AI AGENT RETRY STRATEGY:
[multi-query fallback procedures]

🤖 AI AGENT RESULT INTERPRETATION:
[how to identify correct results]
```

**Why It Works**:
- Creates clear audience targeting
- Agents quickly identify relevant sections
- Signals "this is for you, pay attention"
- Reduces time parsing general documentation

**When to Use**: Always use for agent-specific procedural guidance. Don't overuse (2-3 sections max).

---

### Pattern 3: Multi-Query Strategy Template

**What It Is**: A recovery workflow teaching agents what to do when first attempts fail.

**Copy-Paste Template**:

```markdown
MULTI-[ACTION] STRATEGY:
If first [action] returns [failure condition], try variations:
  [Action] 1: "[first attempt]"
  → [failure condition]?
  [Action] 2: "[second attempt]"
  → [failure condition]?
  [Action] 3: "[third attempt]"
```

**Concrete Example** (Search Tool):

```markdown
MULTI-QUERY STRATEGY:
If first query returns <3 results, try variations:
  Query 1: "error handling"
  → <3 results?
  Query 2: "exception handler"
  → <3 results?
  Query 3: "try catch error"
```

**Concrete Example** (File Finding Tool):

```markdown
MULTI-PATH STRATEGY:
If first pattern finds no files, try broader patterns:
  Pattern 1: "src/auth/login.ts"
  → 0 files?
  Pattern 2: "src/**/login.ts"
  → 0 files?
  Pattern 3: "**/*login*.ts"
```

**Why It Works**:
- Provides fallback strategies
- Reduces task abandonment when results are poor
- Teaches iterative refinement
- Shows concrete variation examples

**When to Use**: Any tool where first attempts can fail or return insufficient results (search, file finding, API calls with fallbacks)

**Impact**: +1.7% performance improvement when included

---

### Pattern 4: Imperative Command Structure

**What It Is**: Using command-style instructions instead of descriptive advice.

**Copy-Paste Templates**:

| Instead of (Descriptive) | Use This (Imperative) |
|--------------------------|----------------------|
| "Best for finding X" | "Extract X from Y" |
| "Use concepts not keywords" | "Remove keywords: [list]" |
| "Think about X" | "Identify X by checking Y" |
| "Try to avoid X" | "Never use X. Use Y instead." |
| "Consider using X when Y" | "If Y occurs, execute X" |

**Concrete Before/After**:

❌ **DESCRIPTIVE (Lower Performance)**:
```markdown
Best for finding functions/classes by concept
Use concepts: "auth" not "authentication_service_implementation_v2"
Think "what does this do" not "what is it called"
```

✅ **IMPERATIVE (Higher Performance)**:
```markdown
TRANSFORMATION PATTERNS:
1. Extract 2-3 core technical terms
2. Remove implementation details and underscores
3. Prefer domain concepts over code identifiers
```

**Why It Works**:
- Imperative = executable procedure
- Descriptive = general principle requiring inference
- Agents execute procedures better than internalize principles
- Reduces interpretation ambiguity

**When to Use**: Always. Replace all descriptive advice with imperative commands.

**Impact**: 100% of top performers used imperative commands

---

### Pattern 5: Complete Lifecycle Coverage

**What It Is**: End-to-end guidance covering transformation → execution → recovery → boundaries.

**Copy-Paste Template**:

```markdown
🤖 AI AGENT [TOOL NAME] WORKFLOW:

1. TRANSFORMATION: [How to convert user input to tool input]
2. EXECUTION: [How to use the tool with those inputs]
3. RECOVERY: [What to do if results are insufficient]
4. BOUNDARIES: [What NOT to use this tool for]
```

**Concrete Example** (Search Tool):

```markdown
🤖 AI AGENT SEARCH WORKFLOW:

1. TRANSFORMATION: Convert question to 2-3 keyword query
   [transformation patterns here]

2. EXECUTION: Use hybrid mode (default) or:
   - fts: When you know exact technical terms
   - vector: When searching by concept

3. RECOVERY: If <3 results, try query variations
   [multi-query strategy here]

4. BOUNDARIES: Not for exact strings (use Grep), file paths (use Glob)
```

**Why It Works**:
- Agents learn complete process
- Covers normal case AND edge cases
- Provides recovery strategies
- Sets clear boundaries

**When to Use**: Complex tools with multiple usage patterns and failure modes

**Impact**: 100% of top-3 performers included all four components

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Static Examples Without Transformation

**What It Looks Like**:

```markdown
EXAMPLES: "authentication flow", "error handling", "database connection"

TIPS:
- Keep it simple: 1-3 words works best
- Use concepts not implementation names
```

**Why It Fails**:
- Shows only OUTPUTS (search queries)
- No INPUT examples (user questions)
- Agents must infer transformation logic
- No systematic procedure to follow

**How to Fix**:

```markdown
🤖 AI AGENT QUERY FORMULATION:

TRANSFORMATION PATTERNS:
1. Extract core technical terms
2. Remove question words
3. Keep nouns and action verbs

EXAMPLES:
  "How does authentication work?" → "authentication"
  "What handles database errors?" → "database error handler"
  "Where is checkout logic?" → "checkout"
```

**Impact**: -1.9% penalty for static examples only

---

### Anti-Pattern 2: Alternative Tool Over-Documentation

**What It Looks Like**:

```markdown
⚠️ NOT FOR:
- Exact string matching (use Grep instead)
- File paths (use Glob instead)
- Special characters

✅ USE GREP WHEN:
- You know the exact text to search for
- Searching for literal patterns, comments, or markers
- Finding special characters (emojis, symbols, punctuation)
- Need regex pattern matching
- Performance is critical for simple searches
[...5 more bullets]

✅ USE GLOB WHEN:
- Finding files by name pattern: "*.test.ts", "components/**/*.tsx"
- Discovering files in specific directories
- File extension or path-based searches
[...3 more bullets]
```

**Token Analysis**:
- Example above: ~150 tokens on alternative tools (43% of description)
- Winners: ~15 tokens on alternatives (3% of description)

**Why It Fails**:
- Dilutes core message
- Creates decision fatigue
- Agents already understand basic tool purposes
- Wastes limited token budget

**How to Fix**:

```markdown
⚠️ NOT FOR:
- Exact text matching (use Grep)
- File name patterns (use Glob)
```

That's it. 15 tokens, not 150.

**Impact**: -1.5% penalty for >30% alternative tool content

---

### Anti-Pattern 3: Missing Systematic Transformation

**What It Looks Like**:

```markdown
Semantic code search finds code by concept.

BEST FOR: Finding functions/classes, exploring codebases
USE WHEN: Searching for functionality rather than exact text

QUERY TIPS:
- Use 1-3 words: "error handling", "auth", "database"
- Concepts work best
```

**Why It Fails**:
- No numbered transformation rules
- No before→after examples
- General principles instead of procedures
- No step-by-step workflow

**How to Fix**:

Add systematic transformation workflow (see Pattern 1).

**Impact**: -0.9% to -1.9% penalty

---

### Anti-Pattern 4: Excessive Brevity

**What It Looks Like**:

A 220-token description that omits:
- Transformation workflow
- Multi-query retry strategy
- Numbered rules
- Sufficient examples

**Why It Fails**:
- Underpowered for complex tasks
- No guidance for edge cases
- No recovery strategies
- Too minimal to teach the skill

**Token Analysis**:
- 220 tokens (too brief): 18.7% performance
- 450 tokens (detailed): 19.6% performance
- 350 tokens (unfocused): 17.7% performance

**Key Insight**: Token count alone doesn't predict performance. A 350-token description with poor structure underperforms a 450-token description with good structure.

**How to Fix**:

Aim for 400-500 tokens with complete lifecycle coverage. Don't sacrifice critical components for brevity.

**Impact**: -0.9% penalty vs detailed variants

---

### Anti-Pattern 5: Descriptive vs Imperative Language

**What It Looks Like**:

```markdown
This tool is best for finding code by concept.

You should think about what the code does, not what it's called.

Try to use simple terms instead of complex implementation names.

Consider using 2-3 words for better results.
```

**Why It Fails**:
- Passive, advisory tone
- Requires agent to interpret guidance
- No executable steps
- Ambiguous ("should", "try", "consider")

**How to Fix**:

```markdown
TRANSFORMATION PATTERNS:
1. Extract core functionality concept
2. Remove implementation details
3. Use 2-3 technical terms maximum
```

**Impact**: 100% of bottom-3 performers used descriptive language. 100% of top-3 used imperative.

---

## How-To Guide

### Step-by-Step: Writing a New Tool Description

Follow this 5-step process to write high-performing tool descriptions.

---

#### Step 1: Identify Transformation Workflow

**Goal**: Define how user intent transforms into tool inputs.

**Process**:

1. **List example user questions/tasks** for this tool:
   ```
   "Find authentication code"
   "Where is error handling?"
   "Show me checkout logic"
   ```

2. **List corresponding tool inputs**:
   ```
   query: "authentication"
   query: "error handler"
   query: "checkout"
   ```

3. **Identify transformation pattern**:
   ```
   Pattern: Extract core technical nouns and action verbs
   Remove: question words, articles, prepositions
   ```

4. **Formalize into numbered rules** (3-4 steps):
   ```
   1. Extract 2-3 core technical terms
   2. Remove: where, is, the, show, me
   3. Prefer domain concepts over code names
   ```

**Output**: A clear transformation procedure

---

#### Step 2: Create Numbered Rules

**Goal**: Convert transformation pattern into actionable, imperative steps.

**Template**:

```markdown
TRANSFORMATION PATTERNS:
1. [Action verb] [what to extract/identify]
2. [Action verb] [what to remove/filter]
3. [Action verb] [how to prioritize/select]
4. [Optional: edge case handling]
```

**Action Verbs to Use**:
- Extract
- Remove
- Prefer
- Identify
- Convert
- Filter
- Select
- Combine
- Split
- Replace

**Example** (File Search Tool):

```markdown
PATH TRANSFORMATION PATTERNS:
1. Extract file name or extension from user question
2. Remove: "find", "show", "where is", "the"
3. Convert to glob pattern: exact name → **/*name*, extension → **/*.ext
4. If multiple components mentioned, combine with slashes: "auth config" → "auth/**/config*"
```

**Quality Check**:
- ✅ Each rule is imperative (starts with action verb)
- ✅ Each rule is concrete (no ambiguous terms like "should" or "consider")
- ✅ 3-4 rules total (not too few, not too many)
- ✅ Rules are ordered logically (extraction → filtering → transformation)

---

#### Step 3: Add Before→After Examples

**Goal**: Show concrete transformations with visual arrows.

**Template**:

```markdown
EXAMPLES:
  "[user input 1]" → "[tool input 1]"
  "[user input 2]" → "[tool input 2]"
  "[user input 3]" → "[tool input 3]"
  "[user input 4]" → "[tool input 4]"
```

**Example Quality Guidelines**:

1. **Show variety**: Different question structures, different domains
2. **Align with rules**: Each example should demonstrate 1+ transformation rules
3. **Use realistic inputs**: Actual user questions, not artificial examples
4. **Keep outputs valid**: Ensure tool inputs would actually work

**Good Examples** (Search Tool):

```markdown
EXAMPLES:
  "How does authentication work?" → "authentication"              # Rule 1+2
  "What handles database errors?" → "database error handler"      # Rule 1+3
  "Find checkout validation" → "checkout validation"              # Rule 2+3
  "Where is WebSocket disconnect?" → "WebSocket disconnect"       # Rule 3
```

**Bad Examples**:

```markdown
EXAMPLES:
  "authentication flow"        # Missing INPUT (no transformation shown)
  "error handling"             # Missing INPUT
  "database connection"        # Missing INPUT
```

**Quality Check**:
- ✅ 4-5 examples minimum
- ✅ Each shows INPUT → OUTPUT transformation
- ✅ Visual arrow (→) separates input and output
- ✅ Examples align with transformation rules
- ✅ Variety in input structures

---

#### Step 4: Include Recovery Strategies

**Goal**: Teach agents what to do when first attempts fail.

**Template**:

```markdown
MULTI-[ACTION] STRATEGY:
If first [action] returns [failure condition], try variations:
  [Action] 1: "[first attempt]"
  → [failure condition]?
  [Action] 2: "[variation 1]"
  → [failure condition]?
  [Action] 3: "[variation 2]"
```

**Example** (API Tool):

```markdown
MULTI-REQUEST STRATEGY:
If first request returns 404 or 0 results, try variations:
  Request 1: GET /api/users/{exact_name}
  → 404 error?
  Request 2: GET /api/users?search={partial_name}
  → 0 results?
  Request 3: GET /api/users?filter=active&search={partial_name}
```

**Quality Check**:
- ✅ Shows concrete failure condition
- ✅ Provides 2-3 variation attempts
- ✅ Visual flow with → arrows
- ✅ Variations are meaningfully different (not just minor tweaks)

---

#### Step 5: Test with AI Agents

**Goal**: Validate that agents understand and follow your description.

**Testing Process**:

1. **Create test tasks** (5-10 realistic user requests):
   ```
   "Find where user authentication happens"
   "Show me error handling for database connections"
   "Where is the checkout validation logic?"
   ```

2. **Run agents with ONLY your tool description**:
   - Don't provide additional context
   - Observe which transformation rules agents follow
   - Track success rate

3. **Identify failure patterns**:
   - Where do agents misunderstand transformation?
   - Which edge cases aren't covered?
   - What ambiguities exist in your rules?

4. **Iterate**:
   - Add examples for failure cases
   - Refine ambiguous rules
   - Add recovery strategies for common failures

**Success Criteria**:
- ✅ Agents successfully transform user questions >80% of the time
- ✅ Agents follow numbered rules systematically
- ✅ Agents use recovery strategies when first attempts fail
- ✅ No misinterpretation of imperative commands

---

## Before/After Examples

### Example 1: Semantic Search Tool

#### ❌ BEFORE (17.7% Performance)

```markdown
Semantic code search - BEST FOR: finding functions/classes by concept,
understanding code relationships, exploring unfamiliar codebases.
FASTER THAN: Grep for conceptual searches. USE WHEN: searching for
functionality rather than exact text matches.

⚠️ NOT FOR:
- Exact string matching: "TODO", "FIXME", "⚠️", "console.log"
- Special characters or symbols in the query
- File paths or file names (use Glob instead)
- Very long queries (>4 words) or implementation-specific names

✅ USE GREP WHEN:
- You know the exact text to search for
- Searching for literal patterns, comments, or markers
- Finding special characters (emojis, symbols, punctuation)
- Need regex pattern matching
- Performance is critical for simple searches

✅ USE GLOB WHEN:
- Finding files by name pattern: "*.test.ts", "components/**/*.tsx"
- Discovering files in specific directories
- File extension or path-based searches

EXAMPLES: "authentication flow", "error handling", "database connection"

QUERY BEST PRACTICES:
- Keep it simple: 1-3 words works best
- Use concepts: "auth" not "authentication_service_implementation_v2"
- Think "what does this do" not "what is it called"
```

**Problems**:
1. ❌ No transformation workflow
2. ❌ 150 tokens on alternative tools (43% of content)
3. ❌ Static examples without INPUT → OUTPUT
4. ❌ Descriptive language ("best for", "think about")
5. ❌ No recovery strategy

---

#### ✅ AFTER (19.6% Performance)

```markdown
Semantic code search - BEST FOR: finding functions/classes by concept,
understanding code relationships, exploring unfamiliar codebases.
FASTER THAN: Grep for conceptual searches. USE WHEN: searching for
functionality rather than exact text matches.

🤖 AI AGENT QUERY FORMULATION:

Transform natural language questions into optimal search queries:

TRANSFORMATION PATTERNS:
1. Extract 2-3 core technical terms
2. Remove: how, what, where, when, why, does, is, are, the, a, an
3. Prefer code-like terminology

EXAMPLES:
  "How does authentication work?" → "authentication"
  "What handles errors?" → "error handler"
  "Find auth logic" → "authentication"
  "Where is WebSocket disconnect?" → "WebSocket disconnect"

QUERY BEST PRACTICES:
✅ Good: 2-3 words, concepts, code terms
  - "error handling"
  - "cart validation"
  - "WebSocket disconnect"

❌ Avoid: Full sentences, questions, special chars
  - "How do I handle errors in the checkout?"
  - "src/cart/checkout.ts"

MULTI-QUERY STRATEGY:
If first query returns <3 results, try variations:
  Query 1: "error handling"
  → <3 results?
  Query 2: "exception handler"
  → <3 results?
  Query 3: "try catch error"

SEARCH MODES:
- "hybrid" (default): Combines FTS + vector for optimal results
- "fts": Best for exact keyword matches, identifiers
- "vector": Best for conceptual queries, similar code

⚠️ NOT FOR: Exact strings (use Grep), file paths (use Glob)
```

**Improvements**:
1. ✅ Numbered transformation workflow with 🤖 marker
2. ✅ Only 15 tokens on alternatives (3% of content)
3. ✅ Before→After examples with → arrows
4. ✅ Imperative language ("Extract", "Remove", "Prefer")
5. ✅ Multi-query recovery strategy
6. ✅ Complete lifecycle coverage

**Performance Impact**: +1.9 percentage points (+10.7% relative improvement)

---

### Example 2: File Finding Tool

#### ❌ BEFORE (Low Performance)

```markdown
Find files by name pattern. Supports glob patterns.

EXAMPLES:
- "*.ts" finds all TypeScript files
- "**/*.test.ts" finds all test files
- "src/**/*.tsx" finds all React components in src

Use this when you need to find files by name or extension.
For content search, use Grep instead.
```

**Problems**:
1. ❌ No transformation from user question to glob pattern
2. ❌ Static examples (outputs only)
3. ❌ Descriptive language ("use this when")
4. ❌ No recovery strategy for failed searches
5. ❌ No systematic guidance

---

#### ✅ AFTER (High Performance)

```markdown
Find files by name pattern. Supports glob patterns.

🤖 AI AGENT PATTERN FORMULATION:

Transform user questions into glob patterns:

TRANSFORMATION PATTERNS:
1. Extract file name, extension, or path component from question
2. Remove: "find", "show", "where is", "the", "all", "any"
3. Convert to glob pattern:
   - Exact name known → **/*{name}*
   - Extension known → **/*.{ext}
   - Path component known → {path}/**/*
   - Multiple criteria → combine with slashes

EXAMPLES:
  "Find all TypeScript files" → "**/*.ts"
  "Where are the test files?" → "**/*.test.ts"
  "Show me React components in src" → "src/**/*.tsx"
  "Find authentication config" → "auth/**/config*"

MULTI-PATTERN STRATEGY:
If first pattern finds 0 files, try broader patterns:
  Pattern 1: "src/auth/login.ts" (exact)
  → 0 files?
  Pattern 2: "src/**/login.ts" (any subdirectory)
  → 0 files?
  Pattern 3: "**/*login*.ts" (anywhere, partial match)

PATTERN SYNTAX:
- * : Matches any characters in filename
- ** : Matches any directory depth
- {a,b} : Matches either a or b
- [0-9] : Matches any digit

⚠️ NOT FOR: File content search (use Grep)
```

**Improvements**:
1. ✅ Clear question → glob pattern transformation
2. ✅ Numbered imperative rules
3. ✅ Before→After examples with realistic questions
4. ✅ Multi-pattern recovery strategy
5. ✅ Systematic guidance with examples

---

### Example 3: API Request Tool

#### ❌ BEFORE (Low Performance)

```markdown
Make HTTP requests to REST APIs. Supports GET, POST, PUT, DELETE.

Parameters:
- url: The API endpoint
- method: HTTP method (default: GET)
- headers: Optional headers object
- body: Optional request body (for POST/PUT)

Returns: Response data or error

Use this for API interactions. Always check response status codes.
```

**Problems**:
1. ❌ No guidance on constructing URLs from user intent
2. ❌ Technical parameter list (not transformation workflow)
3. ❌ Descriptive advice ("always check")
4. ❌ No examples showing user task → API call
5. ❌ No error recovery strategy

---

#### ✅ AFTER (High Performance)

```markdown
Make HTTP requests to REST APIs. Supports GET, POST, PUT, DELETE.

🤖 AI AGENT REQUEST FORMULATION:

Transform user goals into API requests:

TRANSFORMATION PATTERNS:
1. Identify action type:
   - "Get/fetch/retrieve" → GET
   - "Create/add/new" → POST
   - "Update/modify/change" → PUT
   - "Delete/remove" → DELETE

2. Extract resource identifier:
   - "user with ID 123" → /users/123
   - "all active users" → /users?status=active
   - "products in category X" → /products?category=X

3. Build URL: {base_url}/{resource}/{id}?{params}

EXAMPLES:
  "Get user profile for user 123" → GET /api/users/123
  "Fetch all active products" → GET /api/products?status=active
  "Create a new order for cart abc" → POST /api/orders {"cart_id": "abc"}
  "Update product 456 price to $99" → PUT /api/products/456 {"price": 99}

MULTI-REQUEST STRATEGY:
If first request returns error, try fallbacks:
  Request 1: GET /api/users/john_smith (exact match)
  → 404 error?
  Request 2: GET /api/users?name=john_smith (search)
  → 0 results?
  Request 3: GET /api/users?search=john (partial match)

ERROR HANDLING:
- 404: Resource not found → Try search endpoint or broader query
- 401: Unauthorized → Check headers include authentication
- 400: Bad request → Verify request body matches API schema
- 500: Server error → Retry with exponential backoff

⚠️ NOT FOR: GraphQL APIs (use GraphQL tool)
```

**Improvements**:
1. ✅ Task goal → API request transformation
2. ✅ Action verb → HTTP method mapping
3. ✅ Before→After examples with realistic user goals
4. ✅ Multi-request recovery strategy
5. ✅ Concrete error handling procedures
6. ✅ Imperative language throughout

---

## Decision Framework

### When to Use Which Patterns

#### Pattern Selection Matrix

| Tool Characteristics | Recommended Patterns | Token Budget | Example Tools |
|---------------------|---------------------|--------------|---------------|
| **User input needs transformation** | Pattern 1 (Transformation) + Pattern 2 (🤖) + Pattern 3 (Multi-query) | 400-500 | Search, API calls, file finding |
| **Simple, direct mapping** | Pattern 4 (Imperative) only | 200-300 | Read file, list directory |
| **High failure rate** | Pattern 3 (Multi-query) + Pattern 5 (Lifecycle) | 350-450 | Network requests, external APIs |
| **Complex multi-step workflow** | Pattern 5 (Complete Lifecycle) | 450-550 | Multi-stage processing, pipelines |
| **Straightforward, low-ambiguity** | Pattern 4 (Imperative) | 150-250 | Write file, delete file |

---

### Tool Complexity Assessment

Use this decision tree to determine pattern combination:

```
START
│
├─ Does user input need transformation to tool input?
│  YES → Include Pattern 1 (Transformation Workflow)
│  NO → Skip Pattern 1
│
├─ Can first attempts fail or return poor results?
│  YES → Include Pattern 3 (Multi-Query Strategy)
│  NO → Skip Pattern 3
│
├─ Is the workflow multi-step with edge cases?
│  YES → Include Pattern 5 (Complete Lifecycle)
│  NO → Use simplified structure
│
└─ Always use:
   - Pattern 2 (🤖 AI Agent sections) for transformation guidance
   - Pattern 4 (Imperative commands) for all instructions
```

---

### Token Budget Guidelines

**Recommended token allocation** for 400-500 token descriptions:

| Section | Tokens | Percentage | Priority |
|---------|--------|------------|----------|
| Transformation Workflow | 150-200 | 35-40% | CRITICAL |
| Examples (Before→After) | 100-120 | 20-25% | CRITICAL |
| Recovery Strategy | 60-80 | 15-18% | HIGH |
| Execution Guidance | 50-70 | 12-15% | HIGH |
| Boundaries/Anti-patterns | 30-40 | 7-10% | MEDIUM |
| Alternative tool mentions | 10-20 | 3-5% | LOW |

**Critical Rule**: Never sacrifice transformation workflow to add alternative tool documentation.

---

### Complexity Trade-offs

#### When to Choose Comprehensive (450-500 tokens)

**Choose comprehensive when**:
- Tool has high ambiguity in input formulation
- Failure modes are common (>20% first-attempt failure rate)
- User questions vary significantly in structure
- Domain is technical or specialized

**Examples**: Semantic search, regex building, complex queries, API construction

**Validation**: Does the tool require teaching a "skill" vs documenting an "action"?

---

#### When to Choose Minimal (200-300 tokens)

**Choose minimal when**:
- Input mapping is direct (1:1 correspondence)
- Failure modes are rare (<5% failure rate)
- User requests are uniform in structure
- Domain is familiar to agents

**Examples**: Read file, write file, list directory, simple calculations

**Validation**: Can an agent use this tool correctly with zero examples?

---

#### When to Choose Middle Ground (350-400 tokens)

**Choose middle ground when**:
- Some input transformation needed but pattern is simple
- Moderate failure rate (5-20%)
- Some variation in user requests but limited
- Domain is semi-technical

**Examples**: File pattern matching, simple filtering, basic transformations

**Validation**: Do agents need 2-3 examples to understand the pattern?

---

### Pattern Combination Recommendations

#### High-Complexity Tools (Search, APIs, Complex Queries)

**Use all 5 patterns**:
1. ✅ Transformation Workflow (Pattern 1)
2. ✅ AI Agent Sections (Pattern 2)
3. ✅ Multi-Query Strategy (Pattern 3)
4. ✅ Imperative Commands (Pattern 4)
5. ✅ Complete Lifecycle (Pattern 5)

**Token budget**: 450-550 tokens
**Expected performance**: High (19%+ on complex tasks)

---

#### Medium-Complexity Tools (File Operations, Filtering)

**Use patterns 1, 2, 4**:
1. ✅ Transformation Workflow (Pattern 1)
2. ✅ AI Agent Sections (Pattern 2)
3. ❌ Multi-Query Strategy (skip if failure is rare)
4. ✅ Imperative Commands (Pattern 4)
5. ❌ Complete Lifecycle (simplified version)

**Token budget**: 300-400 tokens
**Expected performance**: Medium-High

---

#### Low-Complexity Tools (Direct Actions)

**Use pattern 4 only**:
1. ❌ Transformation Workflow (not needed)
2. ❌ AI Agent Sections (not needed)
3. ❌ Multi-Query Strategy (not needed)
4. ✅ Imperative Commands (Pattern 4)
5. ❌ Complete Lifecycle (not needed)

**Token budget**: 150-250 tokens
**Expected performance**: Adequate for simple tools

---

## References

### Research Foundation

**Primary Source**: Genetic Optimization Experiment
- **Duration**: 10 generations, 58 variants tested
- **Benchmark**: "Find where git worktrees are created in the crewchief CLI and explain how it works"
- **Method**: Parallel Claude Code agent execution with performance scoring
- **Performance Range**: 17.7% (baseline) to 20.4% (peak), 19.6% (consistent winner)

**Key Documents**:
1. `/workspace/docs/optimization/genetic-optimization-results.md` - Full experimental results and analysis
2. `/workspace/.agents/projects/TOOLOPT_maproom-search-tool-optimization/planning/analysis.md` - Pattern analysis and implications
3. `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/` - Raw experimental data

---

### Winning Variant Analysis

**Consistent Winner**: variant-a-detailed (19.6%)

**Key Characteristics**:
- 450 tokens
- Complete transformation workflow
- 5 before→after examples
- Multi-query retry strategy
- Focused scope (90% this tool, 10% alternatives)
- Imperative command structure
- 🤖 AI agent section markers

**Location**: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/variant-a-detailed.json`

---

### Performance Correlations

| Feature | Correlation | Impact |
|---------|-------------|---------|
| Transformation workflow present | **STRONG** (+0.7) | +1.9% |
| Before→After examples count | **STRONG** (+0.6) | +0.4% per example (up to 5) |
| Numbered rules (3-4) | **STRONG** (+0.7) | +1.5% |
| Multi-query strategy | **MODERATE** (+0.6) | +1.7% |
| Alternative tool tokens | **INVERSE** (-0.6) | -0.5% per 50 tokens |
| Imperative vs descriptive | **STRONG** (+0.8) | +1.2% |
| Token count | **WEAK** (+0.3) | Minimal correlation |

---

### Application Examples

**Tools Using These Patterns** (in this codebase):

1. **Maproom Search** (`packages/maproom-mcp/src/tools/search.ts`)
   - Currently using baseline variant (17.7%)
   - Upgrade to winner variant available (+1.9% improvement)

**Tools That Would Benefit** (hypothetical):
- API request builders
- Query construction tools
- Path/pattern generators
- Any tool requiring user intent → structured input transformation

---

### Future Research

**Identified Gaps** (opportunities for >20% performance):

1. **Task-to-Strategy Mapping**: Teaching agents how to map high-level user goals to search strategies
2. **Result Interpretation**: How to identify correct files from search results (implementation vs test vs config)
3. **Iterative Refinement**: Beyond simple query variations, teaching systematic refinement
4. **Context-Aware Search**: When to search vs when information is already available

**Hypothesis**: Addressing task-to-strategy gap could push performance beyond 20% plateau.

---

### Related Documentation

- **Genetic Optimization Results**: `/workspace/docs/optimization/genetic-optimization-results.md`
- **Project Planning**: `/workspace/.agents/projects/TOOLOPT_maproom-search-tool-optimization/`
- **Experimental Data**: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/`

---

## Appendix: Quick Reference Card

### One-Page Cheat Sheet

**✅ DO THIS**:
- Write numbered transformation rules (3-4 steps)
- Show before→after examples with → arrows
- Use imperative commands (Extract, Remove, Prefer)
- Include multi-query recovery strategy
- Mark AI agent sections with 🤖
- Focus 90% on THIS tool

**❌ AVOID THIS**:
- Static examples without transformation
- >30% content on alternative tools
- Descriptive language (should, consider, think)
- Excessive brevity (<300 tokens for complex tools)
- Missing recovery strategies

**Template (Copy-Paste)**:

```markdown
[Tool summary in 1 sentence]

🤖 AI AGENT [INPUT] → [OUTPUT] TRANSFORMATION:

TRANSFORMATION PATTERNS:
1. Extract [what]
2. Remove: [list]
3. Prefer [guidance]

EXAMPLES:
  "[input 1]" → "[output 1]"
  "[input 2]" → "[output 2]"
  "[input 3]" → "[output 3]"
  "[input 4]" → "[output 4]"

MULTI-[ACTION] STRATEGY:
If first [action] returns [failure condition], try variations:
  [Action] 1: "[attempt]"
  → [failure]?
  [Action] 2: "[variation]"

⚠️ NOT FOR: [boundaries] (use [alternative] instead)
```

**Performance Target**: 19%+ on complex transformation tasks

---

**Document Version**: 1.0
**Last Updated**: 2025-11-15
**Maintained By**: CrewChief Optimization Team
**Based On**: 10-generation genetic optimization with 58 variants
