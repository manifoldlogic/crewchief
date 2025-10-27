/**
 * Zod schema for Upsert tool parameter validation
 */

import { z } from 'zod'

/**
 * Schema for Upsert tool parameters
 */
export const UpsertParamsSchema = z.object({
  /** Array of file or directory paths to re-index */
  paths: z
    .array(z.string().min(1, 'Path cannot be empty'))
    .min(1, 'At least one path is required'),

  /** Git commit hash for context */
  commit: z.string().min(1, 'Commit hash is required'),

  /** Repository name */
  repo: z.string().min(1, 'Repository name is required'),

  /** Worktree identifier for isolation */
  worktree: z.string().min(1, 'Worktree name is required'),

  /** Root directory path of the repository */
  root: z.string().min(1, 'Root path is required'),
})

/**
 * Schema for Upsert tool result
 */
export const UpsertResultSchema = z.object({
  /** Number of files updated in index */
  updated_files: z.number().int().min(0),

  /** Number of chunks updated in index */
  updated_chunks: z.number().int().min(0),

  /** Duration of indexing operation in milliseconds */
  duration_ms: z.number().int().min(0),
})

/**
 * Validate Upsert tool parameters
 * @param params - Parameters to validate
 * @returns Validated parameters
 * @throws ZodError if validation fails
 */
export function validateUpsertParams(params: unknown) {
  return UpsertParamsSchema.parse(params)
}

/**
 * Validate Upsert tool result
 * @param result - Result to validate
 * @returns Validated result
 * @throws ZodError if validation fails
 */
export function validateUpsertResult(result: unknown) {
  return UpsertResultSchema.parse(result)
}
