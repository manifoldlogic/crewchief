/**
 * Zod schema for Open tool parameter validation
 */

import { z } from 'zod'

/**
 * Schema for line range parameter
 */
export const RangeSchema = z.object({
  start: z.number().int().min(1, 'Start line must be >= 1'),
  end: z.number().int().min(1, 'End line must be >= 1'),
}).refine(
  (data) => data.start <= data.end,
  {
    message: 'Start line must be <= end line',
    path: ['start'],
  }
)

/**
 * Schema for Open tool parameters
 */
export const OpenParamsSchema = z.object({
  relpath: z.string().min(1, 'relpath is required and cannot be empty'),
  range: RangeSchema.optional(),
  worktree: z.string().optional(),
  commit: z.string().optional(),
})

/**
 * Validate Open tool parameters
 * @param params - Parameters to validate
 * @returns Validated parameters
 * @throws ZodError if validation fails
 */
export function validateOpenParams(params: unknown) {
  return OpenParamsSchema.parse(params)
}
