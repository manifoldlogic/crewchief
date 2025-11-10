# Search Optimization Examples

Example scripts demonstrating the AGENTOPT competition framework.

## Available Examples

### 1. Test Setup (`test-setup.ts`)

Verify your environment is configured correctly.

```bash
pnpm search-optimization:test-setup
```

**What it checks:**

- Node.js version >= 18.0.0
- `ANTHROPIC_API_KEY` environment variable
- `MAPROOM_DATABASE_URL` environment variable
- PostgreSQL connection
- pgvector extension
- Anthropic API access
- Claude Agent SDK installation

**Cost**: Free

**Time**: ~30 seconds

### 2. Single Competition (`run-single-competition.ts`)

Run a minimal competition with 2 variants on 1 task.

```bash
pnpm search-optimization:run-example
```

**What it does:**

- Tests two tool description variants
- Runs a simple "Find CLI Entry Point" task
- Spawns 2 agents sequentially
- Compares performance
- Generates a competition report

**Cost**: ~$0.50-1.00

**Time**: ~2-5 minutes

### 3. Benchmark Suite (`run-suite-example.ts`)

Run benchmark suites with mock or real data.

```bash
tsx src/search-optimization/examples/run-suite-example.ts
```

**What it does:**

- Demonstrates suite execution modes
- Shows aggregate metrics calculation
- Compares sequential vs parallel execution
- Analyzes individual task performance

**Cost**: Free (uses mock data) or $12-20 (real execution)

**Time**: Instant (mock) or 30-60 minutes (real)

## Getting Started

### Step 1: Set Up Environment

```bash
# Set required environment variables
export ANTHROPIC_API_KEY="sk-ant-..."
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"

# Optional: Choose LLM provider
export LLM_PROVIDER="anthropic"  # or "openai"
```

### Step 2: Start PostgreSQL

```bash
cd packages/maproom-mcp
docker compose -f config/docker-compose.yml up -d
```

### Step 3: Verify Setup

```bash
cd packages/cli
pnpm search-optimization:test-setup
```

### Step 4: Run Example

```bash
pnpm search-optimization:run-example
```

## Creating Custom Examples

Use existing examples as templates:

```typescript
// my-custom-competition.ts
import { runCompetition } from '../competition-runner.js'
import { TASK_FIND_WORKTREE_CREATION } from '../tasks/implementation.js'

const variants = [
  {
    id: 'custom-variant',
    name: 'My Custom Description',
    searchToolDescription: 'Your custom tool description here...',
  },
]

const result = await runCompetition({
  task: TASK_FIND_WORKTREE_CREATION,
  variants,
  timeout: 300,
})

console.log('Winner:', result.winner.variantName)
```

Run it:

```bash
tsx src/search-optimization/examples/my-custom-competition.ts
```

## Cost Estimates

| Example                  | Cost       | Time      | LLM Calls   |
| ------------------------ | ---------- | --------- | ----------- |
| test-setup               | Free       | 30s       | 1 test call |
| run-single-competition   | $0.50-1.00 | 2-5 min   | 2 agents    |
| run-suite-example (mock) | Free       | Instant   | 0           |
| run-suite-example (real) | $12-20     | 30-60 min | 24 agents   |

## Environment Variables

### Required

- `ANTHROPIC_API_KEY` - Your Anthropic API key
- `MAPROOM_DATABASE_URL` - PostgreSQL connection string

### Optional

- `LLM_PROVIDER` - Choose provider: "anthropic" (default) or "openai"
- `ANTHROPIC_MODEL` - Specify model (default: "claude-3-5-sonnet-latest")
- `OPENAI_API_KEY` - Required if using OpenAI provider
- `OPENAI_MODEL` - Specify OpenAI model (default: "gpt-4o-mini")

## Troubleshooting

### "ANTHROPIC_API_KEY is not set"

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
```

### "Connection to database failed"

```bash
# Start PostgreSQL
cd packages/maproom-mcp
docker compose -f config/docker-compose.yml up -d

# Verify connection
psql $MAPROOM_DATABASE_URL -c "SELECT 1;"
```

### "Module not found: @anthropic-ai/claude-agent-sdk"

```bash
cd packages/cli
pnpm install
```

### "Rate limit exceeded"

Use sequential execution:

```typescript
await runCompetition({
  parallelExecution: false, // Prevents parallel API calls
  // ...
})
```

## Further Reading

- **[Competition Framework Guide](../../../docs/search-optimization/competition-framework.md)** - Complete setup and usage
- **[Task Design Guide](../../../docs/search-optimization/task-design-guide.md)** - Creating custom tasks
- **[Validation Guide](../../../docs/search-optimization/validation-guide.md)** - Validating task quality

## Questions?

Open an issue with the `search-optimization` tag or check the documentation.
