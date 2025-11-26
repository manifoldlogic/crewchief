/**
 * Search Tool Schema - Parameter validation for semantic code search
 */

import { z } from 'zod'

/**
 * Zod schema for search filters
 */
export const SearchFiltersSchema = z
  .object({
    file_type: z.string().optional().describe('Comma-separated list of file extensions (e.g., "ts,tsx,js")'),
    worktree_id: z.number().int().optional().describe('Filter by specific worktree ID'),
  })
  .optional()
  .default({})

/**
 * Zod schema for search tool parameters
 */
export const SearchParamsSchema = z.object({
  query: z
    .string()
    .trim()
    .min(1, 'query is required and cannot be empty')
    .describe('Search query text - use 2-3 keyword concepts for best results'),
  repo: z
    .string()
    .optional()
    .describe('Repository name to search (e.g., "crewchief")'),
  worktree: z
    .string()
    .optional()
    .describe('Worktree/branch name to search (e.g., "main")'),
  limit: z
    .number()
    .int()
    .min(1)
    .max(100)
    .default(20)
    .describe('Maximum number of results to return (default: 20, max: 100)'),
  mode: z
    .enum(['fts', 'vector', 'hybrid'])
    .default('fts')
    .describe('Search mode: "fts" for full-text, "vector" for semantic, "hybrid" for combined (default: fts)'),
  filter: z
    .enum(['all', 'code', 'docs', 'config', 'tests'])
    .default('all')
    .describe('Content type filter'),
  filters: SearchFiltersSchema,
  debug: z
    .boolean()
    .default(false)
    .describe('Include score breakdown and debug information in results'),
  deduplicate: z
    .boolean()
    .default(true)
    .describe('Deduplicate results across worktrees (default: true)'),
})

export type SearchParams = z.infer<typeof SearchParamsSchema>
export type SearchFilters = z.infer<typeof SearchFiltersSchema>

/**
 * Validate search tool parameters
 * @param params - Raw parameters to validate
 * @returns Validated and normalized parameters
 * @throws ZodError if validation fails
 */
export function validateSearchParams(params: unknown): SearchParams {
  return SearchParamsSchema.parse(params)
}
