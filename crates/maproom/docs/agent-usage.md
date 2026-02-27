# Agent Usage Guide

How to use `maproom` effectively from an LLM agent.
This document distills findings from 24 controlled agent runs on a 500k-LOC
TypeScript/React codebase into actionable guidance for agent plugin developers
and prompt engineers.

This guide covers the CLI surface as of the AFM epic (AFM-01 through AFM-07).

All CLI examples use `--format agent` output and follow the exit-code contract
described in [CLAUDE.md](../CLAUDE.md).

---

## Quick Reference

Use this decision table to choose the right search mode, result count, and
whether to supplement with Grep.

| Query Type | Search Mode | k-value | Grep? | Example |
|:---|:---|:---|:---|:---|
| Identifier lookup | FTS | 10 | No | `"handleLogin"` |
| Concept exploration | FTS or Vector | 10-15 | No | `"authentication flow"` |
| Exhaustive enumeration | FTS | 20-30 | Yes (1 call) | `"all UserProfile renderers"` |
| Absence proof | FTS then Grep | 10 | Yes | `"is there TTL caching?"` |
| Hybrid re-ranking | Hybrid | 10-15 | No | Combined FTS+Vector score |

**Key numbers at a glance:**

| Metric | Maproom | Baseline (Grep/Glob/Read) |
|:---|:---|:---|
| Total score (15 rounds) | **162/180** | 152/180 |
| Avg tool calls | **37.9** | 54.8 |
| Avg wall time | **182s** | 235s |
| Coverage | 42/45 | **45/45** |
| Accuracy | **45/45** | 44/45 |
| Efficiency | **38/45** | 29/45 |

Source: competition-summary.md, 15 scored rounds on Mattermost webapp (~8k
indexed chunks). Both agents ran on Claude Haiku to isolate tool differences.

---

## The Three-Phase Workflow

Every successful agent interaction follows the same pattern: narrow the search
space, deepen understanding, then read source code. The three phases are
**Search**, **Context**, and **Read + Grep**.

### Phase 1: Search (2-4 calls)

Use `maproom search` to locate relevant code chunks by keyword or
concept. This is the entry point for every query.

```bash
maproom search \
  --query "authentication" \
  --repo myrepo \
  --mode fts \
  --format agent
```

Example output:

```text
SEARCH query="authentication" | hits=10 | total_estimate=25 | mode=fts
src/auth/login.ts:42 | func handleLogin | 12.35 | Handles user login with credential validation
src/auth/session.ts:10 | func createSession | 9.88 | Creates a new session after authentication
src/middleware/auth.ts:5 | func authMiddleware | 8.12 | Express middleware for route protection
```

**Empirical budget:** Perfect-score runs (12/12 points) used 2-4 search calls,
then transitioned to context and reading. The worst run (8/12 points) used 13
searches -- it kept rephrasing the same concept instead of deepening
(report-maproom-plugin.md, section 2).

**Soft limit: 5 searches.** After 5 searches without finding useful starting
points, switch to Phase 2 or Phase 3.

**Hard limit: 10 searches.** Do not exceed 10 search calls in a single task.
Agents that hit the cap early sometimes produce better results than agents that
use their full budget, because forced transitions into Phase 2 are productive
transitions (report-maproom-plugin.md, section 2).

### Phase 2: Context (3-8 calls)

Once you have chunk IDs from search results, use `maproom context` to
explore the call graph around each chunk. This reveals callers, callees, tests,
and related code without reading full files.

```bash
maproom context \
  --chunk-id 12345 \
  --callers --callees \
  --format agent
```

Example output:

```text
CONTEXT chunk_id=12345 | tokens=750/6000 | items=4 | truncated=no
primary | src/auth/login.ts:42-68 | 450 | Target function | export async function handleLogin(req, res) {   const { username, password } = req.body;   const user = await authenticate(username, password);
caller | src/routes/api.ts:100-120 | 120 | Calls handleLogin
caller | src/routes/web.ts:55-70 | 90 | Calls handleLogin
callee | src/auth/verify.ts:10-30 | 90 | Called by handleLogin
```

**When to use `--callers` / `--callees`:** Relationship queries ("what calls
X?", "what renders Y?"). Context calls were most valuable for these queries.
Well-performing agents used 3-8 context calls per run (report-maproom-cli.md,
section 4).

**When to skip context:** Pattern queries ("how are feature flags checked?")
and concept queries ("what is the auth flow?"). Agents often went straight from
search to Read for these query types (report-maproom-cli.md, section 4).

**Budget recommendation:** Allocate 3-8 context calls. Use `--budget` to
control token consumption per call (default: 6000 tokens). For advanced
context assembly integration, see
[context_assembly_api.md](context_assembly_api.md).

### Phase 3: Read + Grep (deepen)

After search and context have narrowed the search space, use file reading and
targeted Grep to confirm findings and catch edge cases.

**Read:** Open specific files at specific line ranges identified in Phases 1-2.
The `file_relpath` and line numbers from search/context output point directly to
the relevant code.

**Grep:** Use a single targeted Grep sweep when completeness matters. Grep
catches the 5-15% of results that FTS misses because they score below the top-k
threshold (report-maproom-plugin.md, section 4). For example, in round R8, FTS
found 6 of 7 `UserProfile` renderers. One `grep -r "UserProfile"` would have
caught the missing `SearchResultsItem`.

**When to Grep:**

- Enumeration queries: "find all places X is used"
- Absence proofs: "is there any TTL caching?"
- After FTS returns fewer results than expected

**Grep budget:** 0-3 calls per task. Typically 1 call is sufficient.

---

## Search Strategy

### FTS (Full-Text Search)

FTS accounted for approximately 88% of all Maproom agent search calls across 15
scored rounds (report-maproom-plugin.md, section 1). It is the primary search
mode for agent workflows.

**When to use FTS:**

- Identifier queries: function names, class names, variable names
- Technical terms: error codes, API endpoints, config keys
- Any query involving known vocabulary from the codebase

**Why FTS dominates:** Code queries involve specific identifiers and technical
terms -- exactly what BM25 (the same ranking algorithm used by Elasticsearch and Lucene) excels at. The semantic similarity advantage of
vector search is less pronounced in code (where naming is precise) than in
natural language (where synonymy is common) (report-maproom-cli.md, section 2).

```bash
maproom search \
  --query "handleLogin" \
  --repo myrepo \
  --mode fts \
  --k 10 \
  --format agent
```

### Vector Search

Vector search uses embedding similarity to find conceptually related code. It
requires an embedding provider to be configured.

**When to use Vector Search:**

- Conceptual queries with no known identifiers: "how does auth work?"
- Exploratory queries where you do not know the vocabulary
- After FTS returns zero results for a concept

**Provider configuration:** Vector search requires an embedding provider. See
[configuration_guide.md](configuration_guide.md) for setup and
[CLAUDE.md](../CLAUDE.md) for environment variables (`MAPROOM_EMBEDDING_PROVIDER`,
`OPENAI_API_KEY`, etc.). Without a configured provider, vector search will
return a structured error with exit code 2 (config error -- do not retry).

```bash
maproom search \
  --query "authentication flow" \
  --repo myrepo \
  --mode vector \
  --k 15 \
  --format agent
```

**Note:** During competition testing, all 15 vector-only runs failed due to
provider misconfiguration. FTS alone was sufficient to win the benchmark
(report-competition-summary.md, Vector-Only Experiment). Vector search is a
useful supplement but not load-bearing.

### Hybrid Search

Hybrid mode (`--mode hybrid`) combines FTS and vector scores using Reciprocal
Rank Fusion (RRF). It is useful when neither FTS nor vector search alone
produces sufficient coverage -- for example, when a query mixes known
identifiers with conceptual terms. Hybrid mode requires an embedding provider
to be configured (same as vector search).

```bash
maproom search \
  --query "authentication session handling" \
  --repo myrepo \
  --mode hybrid \
  --k 15 \
  --format agent
```

### When to Supplement with Grep

Grep is the safety net for FTS coverage gaps. FTS returns the top-k results
ranked by BM25 score. If a relevant mention scores below the k-th result, it
never surfaces. Grep returns ALL matches.

**Empirical catch rate:** A single targeted Grep call catches 5-15% of items
that FTS misses (report-maproom-plugin.md, section 4). This is most significant
for enumeration queries where missing a location would cause a bug.

**Decision rule:**

1. If the query asks for "all X" or "every X" -- plan one Grep call in Phase 3
2. If FTS returns `total_estimate` much larger than `hits` -- consider
   increasing k or supplementing with Grep
3. If the answer requires completeness (e.g., a refactoring task) -- always Grep

---

## Interpreting Results

### Agent Format Output

The `--format agent` flag produces compact, pipe-delimited output optimized for
LLM token budgets. Two output types exist:

**Search output** (from `search` and `vector-search` commands):

```text
SEARCH query="<query>" | hits=N | total_estimate=M | mode=<fts|vector>
<file_relpath>:<start_line> | <kind> [<symbol>] | <score> | <preview>
```

Each result line contains four segments separated by ` | `:

1. **Location:** `file_relpath:start_line` -- the file path (relative to repo
   root) and starting line number
2. **Kind:** The chunk type and optional symbol name (e.g., `func handleLogin`,
   `class AuthService`, `heading_2`)
3. **Score:** BM25 relevance score (FTS) or cosine similarity (vector),
   formatted to 2 decimal places
4. **Preview:** First line of the chunk content, or `-` if unavailable

**Context output** (from `context` command):

```text
CONTEXT chunk_id=N | tokens=T/B | items=I | truncated=yes|no
<role> | <file_relpath>:<start>-<end> | <tokens> | <reason> | <preview>
<role> | <file_relpath>:<start>-<end> | <tokens> | <reason>
```

- Primary items include a content preview (first 3 lines, capped at 200 chars)
- Supporting items (caller, callee, test, etc.) omit the preview to save tokens
- The `tokens` field in the header shows total consumed vs budget

**Field name convention:** All output uses `file_relpath` (relative path from
repository root), not `file_path`. This was standardized in AFM-2 across both
FTS and vector search commands.

### Metadata Header

Every search response begins with a metadata header line:

```text
SEARCH query="authentication" | hits=10 | total_estimate=25 | mode=fts
```

- **query:** The search query as submitted (double quotes in query text are
  backslash-escaped)
- **hits:** Number of results returned in this response (bounded by k)
- **total_estimate:** Estimated total matching chunks in the database
- **mode:** Search mode used (`fts` or `vector`)

### Understanding total_estimate

The `total_estimate` field helps agents decide whether to increase k or
supplement with Grep:

| Scenario | Interpretation | Action |
|:---|:---|:---|
| `hits=10, total_estimate=10` | All matches returned | Proceed to Phase 2 |
| `hits=10, total_estimate=25` | More results available | Increase k if needed |
| `hits=10, total_estimate=200+` | Many matches, broad query | Refine query terms |
| `hits=0, total_estimate=0` | No matches | Try different terms or Grep |

When `total_estimate` is significantly larger than `hits`, you have three
options:

1. **Increase k** on the same query to see more results
2. **Refine the query** to narrow the result set
3. **Accept the top-k** and supplement with a targeted Grep in Phase 3

---

## Error Handling

### Structured Error Format

When `--format agent` is active, errors are written to stdout as a single
structured line:

```text
ERROR | type=<error_type> | message=<msg> | suggestion=<action>
```

Pipe characters in the message and suggestion fields are replaced with dashes.
Newlines are replaced with spaces. The error type is a controlled value from a
fixed taxonomy.

All `--format agent` output, including ERROR lines, is written to stdout.

### Error Types and Recovery Actions

| Error Type | Exit Code | Retryable | Recovery Action |
|:---|:---:|:---:|:---|
| `config_error` | 2 | No | Report to user. Missing environment variable or invalid configuration. |
| `embedding_provider` | 2 | No | Provider misconfigured. Check API keys and provider settings. Fall back to FTS. |
| `database` | 1 | Yes | Database connection or corruption. Retry once, then report. |
| `not_found` | 1 | No | Chunk or repository not found. Verify chunk ID or repo name. |
| `validation` | 1 | No | Invalid input (empty query, bad parameters). Fix input and retry. |
| `timeout` | 1 | Yes | Search timed out. Retry with a simpler query or smaller k. |
| `unknown` | 1 | Maybe | Unclassified error. Check logs for details. For daemon log inspection, see the Daemon Mode section in CLAUDE.md. |

**Decision tree for agents:**

```text
Exit code 0 -> Process results normally
Exit code 1 -> Read the error type:
  - database/timeout: Retry once
  - not_found/validation: Fix input, do not retry blindly
  - unknown: Report error, do not retry
Exit code 2 -> Do not retry. This is a configuration problem:
  - config_error: Report missing config to user
  - embedding_provider: Fall back to FTS search mode
```

**Example: vector search with missing provider**

```text
ERROR | type=embedding_provider | message=Failed to create embedding service | suggestion=Set OPENAI_API_KEY or configure MAPROOM_EMBEDDING_PROVIDER
```

Exit code: 2. The agent should fall back to FTS rather than retrying.

**Example: database connection failure**

```text
ERROR | type=database | message=Failed to connect to database at ~/.maproom/maproom.db | suggestion=Check database connection settings
```

Exit code: 1. The agent may retry once.

### Exit Codes

| Exit Code | Meaning | Agent Action |
|---:|:---|:---|
| 0 | Success (results may be empty) | Process results |
| 1 | Runtime error (transient) | Retry once or report |
| 2 | Configuration error (persistent) | Do not retry; fall back or report to user |

The 0/1/2 contract applies when using `--format agent`. Default format
(`--format json`) exit codes may differ.

**Important:** Exit code 0 with zero hits is NOT an error. It means the query
executed successfully but found no matches. The agent should broaden the query
or try Grep -- not treat it as a failure.

---

## Index Hygiene

### Why Index Quality Matters

The quality of search results depends directly on what is in the index. When
non-code files (translations, documentation, test fixtures) are indexed, they
consume top-k result slots and push relevant code out of view.

**Empirical impact of index cleanup:** Adding `.maproomignore` with
`i18n/*.json` on the Mattermost webapp codebase reduced the index from ~59k
chunks to ~8k chunks. Measured effects on the same 9 queries
(report-competition-summary.md, i18n Cleanup Impact):

| Metric | Before Cleanup | After Cleanup | Change |
|:---|:---|:---|:---|
| Score (9 rounds) | 91/108 | 95/108 | **+4 points** |
| Coverage | 23/27 | 25/27 | +2 |
| Avg tool calls | 38.2 | 32.1 | **-16%** |
| Avg wall time | 102s | 92s | **-10%** |

A single `.maproomignore` pattern improved every measured dimension. Index
hygiene is the highest-leverage optimization available
(report-maproom-plugin.md, section 8).

> **Note:** `.maproomignore` must be at the repository root; subdirectory files are silently ignored.

### Common Ignore Patterns

Create a `.maproomignore` file at the repository root. Pattern syntax follows
gitignore conventions. See [CLAUDE.md](../CLAUDE.md) for full syntax
documentation.

Recommended patterns for common codebases:

```gitignore
# Translation / localization files
i18n/**
locales/**
*.po
*.mo

# Test fixtures and data
test-fixtures/**
tests/data/**
__snapshots__/**

# Build output
build/
dist/
target/
.next/

# Large data files
*.sql
*.csv
*.json.gz
data/**

# Generated code
*.generated.ts
*.g.dart
```

### Applying Ignore Patterns to Existing Indexes

After adding or modifying `.maproomignore`, use `clean-ignored` for surgical
removal of already-indexed chunks:

```bash
# Preview what will be removed
maproom clean-ignored --repo myrepo --worktree main --dry-run

# Remove matching chunks
maproom clean-ignored --repo myrepo --worktree main
```

This is faster than a full rescan and avoids re-indexing unchanged files. See
[CLAUDE.md](../CLAUDE.md) for additional details on the `clean-ignored` command
and `.maproomignore` integration.

---

## Appendix: Competition Data

Aggregate metrics from 15 scored rounds (V2 + V3) of the Search Olympics
benchmark on the Mattermost webapp codebase (~500k LOC, TypeScript/React). Both
agents ran on Claude Haiku. Full methodology and per-round data are in the
competition reference reports.

### Overall Results

| Metric | Maproom | Explore (Baseline) | Delta |
|:---|:---|:---|:---|
| Total score | **162/180** | 152/180 | +10 |
| Rounds won | **8** | 2 | -- |
| Rounds tied | 5 | 5 | -- |
| Avg tool calls | **37.9** | 54.8 | **-31%** |
| Avg wall time | **182s** | 235s | **-23%** |

Source: report-competition-summary.md, Combined V2+V3.

### Dimension Breakdown

| Dimension | Maproom | Explore | Notes |
|:---|:---|:---|:---|
| Speed | 37/45 | 34/45 | Maproom faster on average |
| Coverage | 42/45 | **45/45** | Explore achieved perfect coverage |
| Accuracy | **45/45** | 44/45 | Maproom slightly more accurate |
| Efficiency | **38/45** | 29/45 | Largest gap; primary advantage |

Maproom's 10-point lead is almost entirely explained by the Efficiency dimension
(38 vs 29). When both agents used similar call counts, they tied
(report-maproom-plugin.md, section 6).

### Search Behavior

| Metric | Value | Source |
|:---|:---|:---|
| FTS usage share | ~88% | report-maproom-plugin.md, section 1 |
| Searches in perfect runs | 2-4 | report-maproom-plugin.md, section 2 |
| Searches in worst run | 13 | report-maproom-plugin.md, section 2 |
| Context calls per run | 3-8 | report-maproom-cli.md, section 4 |
| Grep catch rate | 5-15% of FTS misses | report-maproom-plugin.md, section 4 |

### Index Hygiene Impact

| Metric | Before (59k chunks) | After (8k chunks) | Change |
|:---|:---|:---|:---|
| Score (9 rounds) | 91/108 | 95/108 | +4 |
| Coverage | 23/27 | 25/27 | +2 |
| Avg tool calls | 38.2 | 32.1 | -16% |
| Avg wall time | 102s | 92s | -10% |

Source: report-competition-summary.md, V1 vs V2 comparison.
Cleanup action: added `i18n/*.json` to `.maproomignore`
(report-maproom-plugin.md, section 8).

### Coverage Analysis

Maproom scored 42/45 on coverage, losing points in 3 of 15 rounds. The failure
mode was consistent: Maproom found the core answer but missed 1-2 peripheral
locations. The Explore baseline scored a perfect 45/45 because iterative Grep
sweeps are inherently exhaustive (report-maproom-plugin.md, section 7).

For queries where missing a location would cause a bug (e.g., "find every place
X is rendered so we can update them all"), supplement Maproom with a Grep sweep.
For queries where understanding the pattern matters more than exhaustive
enumeration, Maproom alone is sufficient and faster.

### Limitations of This Data

- All runs used Claude Haiku; behavior on other models may differ
- Codebase was a single React/Redux frontend; results may vary on other stacks
- Vector search was not successfully tested due to provider misconfiguration
- Scores for Coverage and Accuracy are judged (by orchestrating Opus agent),
  not measured
