/**
 * Search Tool Schema - Parameter validation for semantic code search
 */

import { z } from "zod";

/**
 * Zod schema for search filters
 */
export const SearchFiltersSchema = z
  .object({
    file_type: z
      .string()
      .optional()
      .describe('Comma-separated list of file extensions (e.g., "ts,tsx,js")'),
    worktree_id: z
      .number()
      .int()
      .optional()
      .describe("Filter by specific worktree ID"),
  })
  .optional()
  .default({});

/**
 * Zod schema for search tool parameters
 *
 * Scope rules (D-9 / R1 / R2):
 * - Exactly ONE of {repo, repos, allRepos} MUST be provided.
 * - Omitting all three is an error (prevents accidental 170k-chunk sweeps).
 * - The daemon enforces exactly-one-of on the wire (-32602); we mirror it client-side
 *   so the error message is actionable before any RPC round-trip.
 */
export const SearchParamsSchema = z
  .object({
    query: z
      .string()
      .trim()
      .min(1, {
        message:
          "Query cannot be empty. Provide a search query to find relevant code.",
      })
      .describe("Search query text - use 2-3 keyword concepts for best results"),
    // R1: repo is now optional (single-repo scope; legacy / default form)
    repo: z
      .string()
      .min(1, {
        message:
          "Repository name cannot be empty. Use 'crewchief status' to list available repositories.",
      })
      .optional()
      .describe('Single-repo scope: search exactly this repository (e.g., "crewchief")'),
    // R1: multi-repo scope — search exactly these repos in one query
    repos: z
      .array(z.string().min(1))
      .min(1, { message: "repos must contain at least one repository name." })
      .optional()
      .describe("Multi-repo scope: search these repositories in a single query (requires maproom >= 0.3.0)"),
    // R1: all-repos scope — search every repo in the index
    allRepos: z
      .boolean()
      .optional()
      .describe("All-repos scope: search every indexed repository (requires maproom >= 0.3.0; use with caution on large indices)"),
    worktree: z
      .string()
      .optional()
      .describe('Worktree/branch name to search (e.g., "main")'),
    limit: z
      .number()
      .int()
      .positive({
        message: "Limit must be a positive integer.",
      })
      .max(1000, {
        message: "Limit cannot exceed 1000 results.",
      })
      .default(20)
      .describe(
        "Maximum number of results to return per repo (default: 20, max: 1000). " +
          "In cross-repo mode (repos/allRepos) this is a per-repo cap; " +
          "results are grouped and labelled by repo.",
      ),
    mode: z
      .enum(["fts", "vector", "hybrid"], {
        errorMap: () => ({
          message: "Invalid search mode. Use 'fts', 'vector', or 'hybrid'.",
        }),
      })
      .default("hybrid")
      .describe(
        'Search mode: "fts" for full-text, "vector" for semantic, "hybrid" for combined (default: hybrid, with graceful FTS fallback when no embedding provider is configured)',
      ),
    filter: z
      .enum(["all", "code", "docs", "config", "tests"])
      .default("all")
      .describe("Content type filter"),
    filters: SearchFiltersSchema,
    debug: z
      .boolean()
      .default(false)
      .describe("Include score breakdown and debug information in results"),
    deduplicate: z
      .boolean()
      .default(true)
      .describe(
        "Deduplicate results across worktrees. When true, results with the same " +
          "file path, symbol name, and line number are grouped, returning only the " +
          "highest-scoring instance. Set false to see all results including duplicates. " +
          "(default: true)",
      ),
    include_confidence: z
      .boolean()
      .default(false)
      .describe(
        "Include confidence signals for result quality assessment. Adds source_count, " +
          "score_gap, and is_exact_match fields to results. (default: false)",
      ),
  })
  .superRefine((data, ctx) => {
    // D-9 / R2: exactly-one-of {repo, repos, allRepos:true} — cross-repo requires explicit opt-in.
    // allRepos:false is treated as "not provided" (mirrors Rust Option<bool> Some(true) semantics).
    const scopeCount =
      (data.repo !== undefined ? 1 : 0) +
      (data.repos !== undefined ? 1 : 0) +
      (data.allRepos === true ? 1 : 0);

    if (scopeCount === 0) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["repo"],
        message:
          "A repository scope is required. Provide exactly one of: " +
          'repo (single repo, e.g. "crewchief"), ' +
          'repos (list, e.g. ["crewchief","specs"]), ' +
          "or allRepos:true (all indexed repos, use with caution).",
      });
    } else if (scopeCount > 1) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["repo"],
        message:
          "Only one repository scope may be specified. " +
          "Provide exactly one of: repo, repos, or allRepos:true — not multiple.",
      });
    }
  });

export type SearchParams = z.infer<typeof SearchParamsSchema>;
export type SearchFilters = z.infer<typeof SearchFiltersSchema>;

/**
 * Validate search tool parameters
 * @param params - Raw parameters to validate
 * @returns Validated and normalized parameters
 * @throws ZodError if validation fails
 */
export function validateSearchParams(params: unknown): SearchParams {
  return SearchParamsSchema.parse(params);
}
