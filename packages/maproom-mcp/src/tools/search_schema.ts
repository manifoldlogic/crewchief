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
 */
export const SearchParamsSchema = z.object({
  query: z
    .string()
    .trim()
    .min(1, {
      message:
        "Query cannot be empty. Provide a search query to find relevant code.",
    })
    .describe("Search query text - use 2-3 keyword concepts for best results"),
  repo: z
    .string({
      required_error:
        "Repository name is required. Use 'crewchief status' to list available repositories.",
    })
    .min(1, {
      message:
        "Repository name is required. Use 'crewchief status' to list available repositories.",
    })
    .describe('Repository name to search (e.g., "crewchief")'),
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
    .describe("Maximum number of results to return (default: 20, max: 1000)"),
  mode: z
    .enum(["fts", "vector", "hybrid"], {
      errorMap: () => ({
        message: "Invalid search mode. Use 'fts', 'vector', or 'hybrid'.",
      }),
    })
    .default("fts")
    .describe(
      'Search mode: "fts" for full-text, "vector" for semantic, "hybrid" for combined (default: fts)',
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
  include_related: z
    .boolean()
    .default(false)
    .describe(
      "Include related chunks for high-confidence results via graph traversal. " +
        "Finds top 5 related chunks for results with high confidence (source_count >= 2 OR is_exact_match). " +
        "Automatically enables confidence scoring. Performance impact: <20ms overhead. " +
        "Response structure: High-confidence results get a 'related' array field. " +
        "(default: false)",
    ),
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
