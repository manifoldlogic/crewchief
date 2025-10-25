# Maproom MCP Server - Example Workflows

This document provides practical, step-by-step examples for common workflows using the Maproom MCP server. Each example includes actual tool calls with parameters and expected results.

## Table of Contents

- [Workflow 1: Find Authentication Implementation](#workflow-1-find-authentication-implementation)
- [Workflow 2: Get Context for Specific Function](#workflow-2-get-context-for-specific-function)
- [Workflow 3: Update Index After Code Changes](#workflow-3-update-index-after-code-changes)
- [Workflow 4: Navigate Codebase by Concept](#workflow-4-navigate-codebase-by-concept)
- [Workflow 5: Understand Test Coverage](#workflow-5-understand-test-coverage)
- [Workflow 6: Compare Implementations Across Branches](#workflow-6-compare-implementations-across-branches)
- [Workflow 7: Refactor Function Safely](#workflow-7-refactor-function-safely)
- [Workflow 8: Explore Architecture](#workflow-8-explore-architecture)
- [Workflow 9: Debug Error Handler](#workflow-9-debug-error-handler)
- [Workflow 10: Find Configuration](#workflow-10-find-configuration)

---

## Workflow 1: Find Authentication Implementation

**Goal**: Locate and understand the authentication implementation in a codebase

**Scenario**: You're new to a project and need to understand how user authentication works.

### Step 1: Verify Repository is Indexed

**Tool**: `status`

**Parameters**:
```json
{
  "repo": "crewchief"
}
```

**Expected Output**:
```json
{
  "repositories": [
    {
      "name": "crewchief",
      "worktrees": [
        {
          "name": "main",
          "files": 234,
          "chunks": 1567,
          "last_indexed": "2024-10-25T10:30:00Z"
        }
      ]
    }
  ]
}
```

**Interpretation**: The repository is indexed with 234 files and 1567 code chunks. Ready to search.

---

### Step 2: Search for Authentication Code

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "authentication login",
  "filter": "code",
  "k": 10
}
```

**Expected Output**:
```json
{
  "results": [
    {
      "chunk_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "relpath": "packages/cli/src/auth/authenticate.ts",
      "worktree": "main",
      "start_line": 15,
      "end_line": 45,
      "score": 0.89,
      "content": "export async function authenticate(credentials: Credentials) {\n  // Validate credentials\n  const user = await validateUser(credentials);\n  ...\n}"
    },
    {
      "chunk_id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "relpath": "packages/cli/src/auth/session.ts",
      "worktree": "main",
      "start_line": 20,
      "end_line": 50,
      "score": 0.82,
      "content": "export class SessionManager {\n  async createSession(user: User) {\n    ...\n  }\n}"
    }
  ],
  "total": 10,
  "query": "authentication login"
}
```

**Interpretation**: Found 10 results. The top result is the main `authenticate` function with a score of 0.89 (highly relevant).

---

### Step 3: View the Authentication Function

**Tool**: `open`

**Parameters**:
```json
{
  "relpath": "packages/cli/src/auth/authenticate.ts",
  "worktree": "main",
  "range": {
    "start": 15,
    "end": 45
  },
  "context": 10
}
```

**Expected Output**:
```typescript
// Lines 5-14 (context before)
import { User } from '../types';
import { validateUser } from './validation';
import { SessionManager } from './session';
import { logger } from '../utils/logger';

const sessionManager = new SessionManager();

// Lines 15-45 (requested range)
export async function authenticate(credentials: Credentials): Promise<AuthResult> {
  try {
    // Validate credentials
    const user = await validateUser(credentials);

    if (!user) {
      logger.warn({ email: credentials.email }, 'Authentication failed: user not found');
      return { success: false, error: 'Invalid credentials' };
    }

    // Check if user is active
    if (!user.isActive) {
      logger.warn({ userId: user.id }, 'Authentication failed: user inactive');
      return { success: false, error: 'Account inactive' };
    }

    // Create session
    const session = await sessionManager.createSession(user);

    logger.info({ userId: user.id }, 'User authenticated successfully');
    return { success: true, user, session };
  } catch (error) {
    logger.error({ error }, 'Authentication error');
    return { success: false, error: 'Authentication failed' };
  }
}

// Lines 46-55 (context after)
export async function logout(sessionId: string): Promise<void> {
  await sessionManager.destroySession(sessionId);
  logger.info({ sessionId }, 'User logged out');
}
```

**Interpretation**: The authentication function validates credentials, checks user status, and creates a session. Context shows related imports and the logout function.

---

### Step 4: Get Related Code Context

**Tool**: `context`

**Parameters**:
```json
{
  "chunk_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "budget_tokens": 8000,
  "expand": {
    "callers": true,
    "callees": true,
    "tests": true,
    "max_depth": 2
  }
}
```

**Expected Output**:
```json
{
  "target_chunk": {
    "chunk_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "relpath": "packages/cli/src/auth/authenticate.ts",
    "content": "export async function authenticate..."
  },
  "related_chunks": [
    {
      "relationship": "calls",
      "chunk": {
        "relpath": "packages/cli/src/auth/validation.ts",
        "content": "export async function validateUser(credentials) {...}"
      }
    },
    {
      "relationship": "called_by",
      "chunk": {
        "relpath": "packages/cli/src/routes/auth.ts",
        "content": "router.post('/login', async (req, res) => {\n  const result = await authenticate(req.body);\n  ...\n})"
      }
    },
    {
      "relationship": "test",
      "chunk": {
        "relpath": "packages/cli/tests/auth/authenticate.test.ts",
        "content": "describe('authenticate', () => {\n  it('should authenticate valid user', async () => {...})\n})"
      }
    }
  ],
  "total_tokens": 6543
}
```

**Interpretation**: The context shows:
- **Callees**: `validateUser` is called by `authenticate`
- **Callers**: The `/login` route calls `authenticate`
- **Tests**: Test file shows how the function is tested

---

### Step 5: Explain the Function (Optional)

**Tool**: `explain`

**Parameters**:
```json
{
  "chunk_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

**Expected Output**:
```markdown
# Symbol: authenticate

**Type**: Function (async)
**Language**: TypeScript
**File**: packages/cli/src/auth/authenticate.ts
**Lines**: 15-45

## Description

Authenticates a user with provided credentials. Validates the credentials, checks user status, and creates a session upon successful authentication.

## Signature

```typescript
async function authenticate(credentials: Credentials): Promise<AuthResult>
```

## Parameters

- `credentials: Credentials` - User credentials containing email and password

## Returns

`Promise<AuthResult>` - Authentication result with success status, user data, and session if successful

## Relationships

**Calls**:
- `validateUser(credentials)` - Validates user credentials
- `sessionManager.createSession(user)` - Creates user session

**Called By**:
- `packages/cli/src/routes/auth.ts:12` - POST /login route handler

## Usage Example

```typescript
const result = await authenticate({
  email: 'user@example.com',
  password: 'secret123'
});

if (result.success) {
  console.log('Logged in as', result.user.email);
} else {
  console.error('Login failed:', result.error);
}
```

## Related Symbols

- `validateUser` - Credential validation
- `SessionManager` - Session management
- `logout` - Session termination
```

**Interpretation**: Comprehensive documentation including signature, relationships, and usage examples.

---

### Summary

**What We Learned**:
1. The authentication system is in `packages/cli/src/auth/`
2. Main function is `authenticate()` in `authenticate.ts`
3. It calls `validateUser()` for validation
4. Sessions are managed by `SessionManager`
5. The `/login` route uses this function
6. Tests exist in `tests/auth/authenticate.test.ts`

**Time Saved**: Instead of grepping through files or reading all auth-related code, we found exactly what we needed in 5 tool calls.

---

## Workflow 2: Get Context for Specific Function

**Goal**: Understand a complex function and its dependencies

**Scenario**: You found a function called `processPayment` and need to understand how it works and what it depends on.

### Step 1: Search for the Function

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "ecommerce",
  "query": "process payment",
  "filter": "code"
}
```

**Expected Output**:
```json
{
  "results": [
    {
      "chunk_id": "p1a2y3m4-e5n6-7890-abcd-ef1234567890",
      "relpath": "src/payments/processor.ts",
      "worktree": "main",
      "start_line": 45,
      "end_line": 120,
      "score": 0.95
    }
  ]
}
```

---

### Step 2: Get Comprehensive Context

**Tool**: `context`

**Parameters**:
```json
{
  "chunk_id": "p1a2y3m4-e5n6-7890-abcd-ef1234567890",
  "budget_tokens": 12000,
  "expand": {
    "callers": true,
    "callees": true,
    "tests": true,
    "docs": true,
    "config": true,
    "max_depth": 3
  }
}
```

**Expected Output**:
```json
{
  "target_chunk": {
    "relpath": "src/payments/processor.ts",
    "content": "async function processPayment(order: Order) {...}"
  },
  "related_chunks": [
    {
      "relationship": "calls",
      "depth": 1,
      "chunk": {
        "relpath": "src/payments/stripe.ts",
        "content": "async function chargeCard(amount, token) {...}"
      }
    },
    {
      "relationship": "calls",
      "depth": 1,
      "chunk": {
        "relpath": "src/orders/validation.ts",
        "content": "function validateOrder(order) {...}"
      }
    },
    {
      "relationship": "called_by",
      "depth": 1,
      "chunk": {
        "relpath": "src/api/checkout.ts",
        "content": "router.post('/checkout', async (req) => {\n  await processPayment(req.body.order)\n})"
      }
    },
    {
      "relationship": "test",
      "chunk": {
        "relpath": "tests/payments/processor.test.ts",
        "content": "describe('processPayment', () => {...})"
      }
    },
    {
      "relationship": "config",
      "chunk": {
        "relpath": "config/payment.json",
        "content": "{\n  \"stripe\": {\n    \"apiKey\": \"...\"\n  }\n}"
      }
    },
    {
      "relationship": "calls",
      "depth": 2,
      "chunk": {
        "relpath": "src/notifications/email.ts",
        "content": "async function sendReceipt(order) {...}"
      }
    }
  ],
  "total_tokens": 10234
}
```

**Interpretation**:
- **Direct calls (depth 1)**: `chargeCard`, `validateOrder`
- **Called by**: Checkout API route
- **Indirect calls (depth 2)**: Email notification
- **Tests**: Comprehensive test suite exists
- **Config**: Payment configuration with Stripe API key

---

### Step 3: View the Main Implementation

**Tool**: `open`

**Parameters**:
```json
{
  "relpath": "src/payments/processor.ts",
  "worktree": "main",
  "range": {
    "start": 45,
    "end": 120
  },
  "context": 15
}
```

**Interpretation**: Full implementation with context of imports and helper functions.

---

### Step 4: Check the Tests

**Tool**: `open`

**Parameters**:
```json
{
  "relpath": "tests/payments/processor.test.ts",
  "worktree": "main"
}
```

**Interpretation**: Learn how the function is tested and what edge cases are covered.

---

### Summary

**What We Found**:
- Payment processing flow: validation → Stripe charge → email receipt
- API endpoint that triggers payment: `/checkout`
- Configuration requirements: Stripe API key
- Test coverage: Comprehensive test suite
- Error handling patterns from tests

**Use Case**: Perfect for understanding critical business logic before making changes.

---

## Workflow 3: Update Index After Code Changes

**Goal**: Keep the search index current during active development

**Scenario**: You just refactored the authentication module and want the search to reflect your changes.

### Step 1: Make Code Changes

(Edit files in your editor)

---

### Step 2: Index the Changed Files

**Tool**: `upsert`

**Parameters**:
```json
{
  "paths": [
    "packages/cli/src/auth/authenticate.ts",
    "packages/cli/src/auth/validation.ts",
    "packages/cli/src/auth/session.ts"
  ],
  "commit": "HEAD",
  "repo": "crewchief",
  "worktree": "main",
  "root": "/Users/developer/projects/crewchief"
}
```

**Expected Output**:
```json
{
  "indexed_files": 3,
  "chunks_created": 24,
  "chunks_updated": 18,
  "chunks_deleted": 6,
  "duration_ms": 342,
  "statistics": {
    "total_lines": 456,
    "total_bytes": 15234,
    "languages": {
      "TypeScript": 3
    }
  }
}
```

**Interpretation**:
- 3 files indexed successfully
- 24 new code chunks created
- 18 existing chunks updated
- 6 old chunks removed (deleted code)
- Took 342ms

---

### Step 3: Verify New Code is Searchable

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "new function name you just added",
  "filter": "code"
}
```

**Expected Output**: Your newly added function appears in search results.

---

### Step 4: Check Overall Index Status

**Tool**: `status`

**Parameters**:
```json
{
  "repo": "crewchief"
}
```

**Expected Output**: Updated chunk counts and last_indexed timestamp.

---

### Best Practices for Incremental Indexing

**When to Re-index**:
- After adding new functions
- After renaming symbols
- After refactoring
- Before searching for new code
- After merging branches

**What to Index**:
- Only changed files for speed
- Related files if dependencies changed
- Test files if implementation changed
- Full repo if major refactoring

**Automation Ideas**:
- Git post-commit hook
- CI/CD pipeline step
- File watcher during development
- Pre-PR validation

---

## Workflow 4: Navigate Codebase by Concept

**Goal**: Explore a large codebase by architectural concepts

**Scenario**: You're new to a large project and want to understand the architecture.

### Step 1: Find Entry Points

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "main entry index",
  "filter": "code",
  "k": 5
}
```

**Expected Output**: Entry points like `index.ts`, `main.ts`, `app.ts`

---

### Step 2: Explore Routing Layer

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "router handler endpoint",
  "filter": "code",
  "k": 15
}
```

**Expected Output**: All route definitions and handlers

---

### Step 3: Find Service Layer

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "service business logic",
  "filter": "code"
}
```

**Expected Output**: Service classes and business logic

---

### Step 4: Locate Database Layer

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "database model repository",
  "filter": "code"
}
```

**Expected Output**: Database models, repositories, queries

---

### Step 5: Map Dependencies

For each interesting result, use the `context` tool:

**Tool**: `context`

**Parameters**:
```json
{
  "chunk_id": "chunk-id-from-search",
  "budget_tokens": 10000,
  "expand": {
    "callers": true,
    "callees": true,
    "max_depth": 2
  }
}
```

**Interpretation**: Build a mental map of how layers connect.

---

### Architectural Concepts to Search

**Common Patterns**:
- "middleware chain" - Express/Koa middleware
- "dependency injection" - DI containers
- "event bus message" - Event-driven architecture
- "cache layer" - Caching strategies
- "error handler" - Error handling
- "validation schema" - Input validation
- "authentication guard" - Auth middleware
- "logging audit" - Logging and monitoring

**By Layer**:
- Presentation: "controller handler route"
- Business: "service manager business"
- Data: "repository model query"
- Infrastructure: "config logger cache"

---

## Workflow 5: Understand Test Coverage

**Goal**: Find and understand existing tests before writing new ones

**Scenario**: You need to add a feature and want to understand the existing test patterns.

### Step 1: Search for Related Tests

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "authentication test describe",
  "filter": "code"
}
```

**Expected Output**: Test files for authentication

---

### Step 2: Get Test Context

**Tool**: `context`

**Parameters**:
```json
{
  "chunk_id": "test-chunk-id",
  "budget_tokens": 8000,
  "expand": {
    "callers": false,
    "callees": true,
    "tests": false,
    "max_depth": 1
  }
}
```

**Expected Output**: What the test actually calls (the implementation)

---

### Step 3: View Test Implementation

**Tool**: `open`

**Parameters**:
```json
{
  "relpath": "tests/auth/authenticate.test.ts",
  "worktree": "main"
}
```

**Expected Output**: Full test file with all test cases

---

### Step 4: Find Test Utilities

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "test helper fixture mock",
  "filter": "code"
}
```

**Expected Output**: Shared test utilities and fixtures

---

### What to Look For in Tests

**Test Structure**:
- How are tests organized?
- What testing framework? (Jest, Vitest, Mocha)
- Are there test utilities?
- How are mocks created?

**Coverage Patterns**:
- Happy path tests
- Error case tests
- Edge case tests
- Integration vs. unit tests

**Testing Style**:
- AAA (Arrange, Act, Assert)
- Given-When-Then
- Test data patterns
- Assertion libraries

---

## Workflow 6: Compare Implementations Across Branches

**Goal**: See how code differs between branches

**Scenario**: You want to compare the main branch implementation with a feature branch.

### Step 1: Check Available Worktrees

**Tool**: `status`

**Parameters**:
```json
{}
```

**Expected Output**:
```json
{
  "repositories": [
    {
      "name": "crewchief",
      "worktrees": [
        { "name": "main", "files": 234 },
        { "name": "feature-oauth", "files": 238 }
      ]
    }
  ]
}
```

---

### Step 2: Search in Main Branch

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "authentication",
  "worktree": "main",
  "filter": "code"
}
```

**Expected Output**: Authentication implementation in main branch

---

### Step 3: Search in Feature Branch

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "authentication",
  "worktree": "feature-oauth",
  "filter": "code"
}
```

**Expected Output**: New OAuth implementation in feature branch

---

### Step 4: Compare Implementations

**Tool**: `open` (for each version)

**Main Branch**:
```json
{
  "relpath": "packages/cli/src/auth/authenticate.ts",
  "worktree": "main"
}
```

**Feature Branch**:
```json
{
  "relpath": "packages/cli/src/auth/authenticate.ts",
  "worktree": "feature-oauth"
}
```

**Interpretation**: Side-by-side comparison of old vs. new implementation.

---

### Use Cases

- **Code Review**: Understand changes before reviewing PR
- **Migration**: Compare old vs. new patterns
- **Learning**: See how implementation evolved
- **Debugging**: Check if bug exists in other branches

---

## Workflow 7: Refactor Function Safely

**Goal**: Rename or refactor a function with full confidence

**Scenario**: You need to rename `processPayment` to `handlePayment` and update all call sites.

### Step 1: Find All Usages

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "ecommerce",
  "query": "processPayment",
  "filter": "code",
  "k": 20
}
```

**Expected Output**: All files mentioning `processPayment`

---

### Step 2: Analyze Each Usage

For each result, get context to understand the call:

**Tool**: `context`

**Parameters**:
```json
{
  "chunk_id": "usage-chunk-id",
  "budget_tokens": 6000,
  "expand": {
    "callers": true,
    "callees": false,
    "max_depth": 1
  }
}
```

**Interpretation**: See how each caller uses the function.

---

### Step 3: Check Test Coverage

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "ecommerce",
  "query": "processPayment test",
  "filter": "code"
}
```

**Expected Output**: All tests for the function

---

### Step 4: Perform Refactoring

(Make changes in your editor using find-and-replace or refactoring tools)

---

### Step 5: Re-index Changed Files

**Tool**: `upsert`

**Parameters**:
```json
{
  "paths": [
    "src/payments/processor.ts",
    "src/api/checkout.ts",
    "src/api/orders.ts",
    "tests/payments/processor.test.ts"
  ],
  "commit": "HEAD",
  "repo": "ecommerce",
  "worktree": "main",
  "root": "/path/to/ecommerce"
}
```

---

### Step 6: Verify Refactoring

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "ecommerce",
  "query": "handlePayment",
  "filter": "code"
}
```

**Expected Output**: All new call sites with `handlePayment`

---

**Tool**: `search` (verify old name is gone)

**Parameters**:
```json
{
  "repo": "ecommerce",
  "query": "processPayment",
  "filter": "code"
}
```

**Expected Output**: Empty results (or only in comments/docs)

---

### Refactoring Checklist

- [ ] Find all usages with search
- [ ] Analyze context for each usage
- [ ] Check test coverage
- [ ] Make changes
- [ ] Re-index affected files
- [ ] Verify new name is findable
- [ ] Verify old name is gone
- [ ] Run tests
- [ ] Commit changes

---

## Workflow 8: Explore Architecture

**Goal**: Understand the overall system architecture

**Scenario**: Onboarding to a new codebase and building a mental model.

### Architecture Discovery Workflow

#### 1. Entry Points

**Search**: "main entry index"
**Goal**: Find where execution begins

---

#### 2. Core Services

**Search**: "service manager class"
**Goal**: Identify main business logic

---

#### 3. Data Layer

**Search**: "database model repository"
**Goal**: Understand data models

---

#### 4. API Layer

**Search**: "router endpoint handler"
**Goal**: Map out API surface

---

#### 5. Configuration

**Search**: "config settings environment"
**Filter**: "config"
**Goal**: Find configuration files

---

#### 6. Utilities

**Search**: "util helper function"
**Goal**: Discover shared utilities

---

#### 7. Tests

**Search**: "test describe integration"
**Goal**: Learn from tests

---

### Building the Mental Model

For each architectural component:

1. **Search** for the concept
2. **Open** the main file
3. **Context** to see dependencies
4. **Explain** key symbols
5. Document your findings

---

### Example Architecture Map

```
Entry Point (index.ts)
  └─> Server Setup (server.ts)
       ├─> Router (routes/index.ts)
       │    ├─> Auth Routes (routes/auth.ts)
       │    │    └─> AuthService (services/auth.ts)
       │    ├─> User Routes (routes/users.ts)
       │    │    └─> UserService (services/users.ts)
       │    └─> Payment Routes (routes/payments.ts)
       │         └─> PaymentService (services/payments.ts)
       ├─> Database (database/index.ts)
       │    ├─> User Model (models/user.ts)
       │    ├─> Order Model (models/order.ts)
       │    └─> Payment Model (models/payment.ts)
       └─> Config (config/index.ts)
```

Use search and context tools to build this map.

---

## Workflow 9: Debug Error Handler

**Goal**: Understand error handling flow when debugging

**Scenario**: Users are seeing errors and you need to trace the error handling.

### Step 1: Find Error Handler

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "error handler middleware",
  "filter": "code"
}
```

**Expected Output**: Error handling middleware

---

### Step 2: Find Error Classes

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "error class exception",
  "filter": "code"
}
```

**Expected Output**: Custom error classes

---

### Step 3: Trace Error Flow

**Tool**: `context`

**Parameters**:
```json
{
  "chunk_id": "error-handler-chunk-id",
  "budget_tokens": 10000,
  "expand": {
    "callers": true,
    "callees": true,
    "max_depth": 2
  }
}
```

**Expected Output**: Where errors are caught and how they're handled

---

### Step 4: Find Error Logging

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "logger error log",
  "filter": "code"
}
```

**Expected Output**: Logging infrastructure

---

### Step 5: Check Error Tests

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "error test should throw",
  "filter": "code"
}
```

**Expected Output**: How errors are tested

---

### Error Handling Checklist

- [ ] Error handler middleware
- [ ] Custom error classes
- [ ] Error propagation flow
- [ ] Logging and monitoring
- [ ] Error response format
- [ ] Test coverage for errors

---

## Workflow 10: Find Configuration

**Goal**: Locate all configuration files and understand config structure

**Scenario**: You need to add a new configuration option.

### Step 1: Find Config Files

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "config settings",
  "filter": "config"
}
```

**Expected Output**: All .json, .yaml, .toml config files

---

### Step 2: Find Config Loading Code

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "load config environment",
  "filter": "code"
}
```

**Expected Output**: Configuration loading logic

---

### Step 3: View Config Schema

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "config schema validation",
  "filter": "code"
}
```

**Expected Output**: Configuration validation (Zod, Joi, etc.)

---

### Step 4: Check Environment Variables

**Tool**: `search`

**Parameters**:
```json
{
  "repo": "crewchief",
  "query": "process.env environment variable",
  "filter": "code"
}
```

**Expected Output**: All environment variable usage

---

### Step 5: View Config Context

**Tool**: `context`

**Parameters**:
```json
{
  "chunk_id": "config-loader-chunk-id",
  "budget_tokens": 8000,
  "expand": {
    "callers": true,
    "callees": true
  }
}
```

**Expected Output**: How config is used throughout the app

---

### Configuration Discovery Checklist

- [ ] Config file locations (.env, config.json, etc.)
- [ ] Config loading mechanism
- [ ] Config schema/validation
- [ ] Environment variables
- [ ] Default values
- [ ] Config usage in code
- [ ] Config tests

---

## Tips for Effective Workflows

### 1. Start with Status

Always begin with the `status` tool to verify:
- Repository is indexed
- Worktrees are available
- Index is up-to-date

---

### 2. Iterate on Queries

If search returns no results:
- Simplify the query (fewer words)
- Try synonyms
- Remove filter to broaden search
- Check worktree parameter

---

### 3. Use Context Wisely

- Start with default budget (6000 tokens)
- Increase for complex understanding
- Reduce for quick checks
- Enable only needed expansions

---

### 4. Combine Tools

Effective workflows combine multiple tools:
1. **Search** to find
2. **Context** to understand relationships
3. **Open** to view details
4. **Explain** for documentation
5. **Upsert** to keep current

---

### 5. Document Your Findings

As you explore:
- Note key files and their purposes
- Map out architectural relationships
- Document configuration requirements
- Identify patterns and conventions

---

### 6. Keep Index Current

Re-index after:
- Major refactoring
- Adding new features
- Renaming symbols
- Merging branches
- Pulling updates

---

## Conclusion

These workflows demonstrate the power of semantic code search for:
- **Onboarding**: Quickly understand new codebases
- **Development**: Find and understand code as you work
- **Refactoring**: Safely change code with full context
- **Debugging**: Trace execution and error flows
- **Documentation**: Discover how systems work
- **Learning**: Study patterns and best practices

The key is combining the right tools in sequence to build comprehensive understanding of your codebase.

---

## Next Steps

1. Try these workflows on your own codebase
2. Develop custom workflows for your use cases
3. Share patterns with your team
4. Automate indexing in your CI/CD pipeline
5. Integrate with your IDE for seamless workflow

Happy coding!
