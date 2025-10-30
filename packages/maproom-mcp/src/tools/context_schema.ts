/**
 * Context Tool Schema - Parameter validation for context assembly
 */

import { z } from 'zod'

/**
 * Zod schema for expand options
 */
export const ExpandOptionsSchema = z
  .object({
    callers: z.boolean().default(true).describe('Include chunks that call this function'),
    callees: z.boolean().default(true).describe('Include chunks called by this function'),
    tests: z.boolean().default(true).describe('Include test chunks for this code'),
    docs: z.boolean().default(false).describe('Include documentation chunks'),
    config: z.boolean().default(false).describe('Include related configuration files'),
    max_depth: z
      .number()
      .int()
      .min(1)
      .max(5)
      .default(2)
      .describe('Maximum relationship traversal depth'),
  })
  .optional()
  .default({
    callers: true,
    callees: true,
    tests: true,
    docs: false,
    config: false,
    max_depth: 2,
  })

/**
 * Zod schema for context tool parameters
 */
export const ContextParamsSchema = z.object({
  chunk_id: z
    .string()
    .describe('UUID or ID of the target chunk to retrieve context for (from search results)')
    .refine((val) => {
      const parsed = parseInt(val, 10)
      return !isNaN(parsed) && parsed > 0
    }, 'chunk_id must be a valid positive integer'),
  budget_tokens: z
    .number()
    .int()
    .min(1000)
    .max(20000)
    .default(6000)
    .describe('Maximum number of tokens to include in the context bundle (default: 6000, range: 1000-20000)'),
  expand: ExpandOptionsSchema,
})

export type ContextParams = z.infer<typeof ContextParamsSchema>
export type ExpandOptions = z.infer<typeof ExpandOptionsSchema>

/**
 * Validate context tool parameters
 * @param params - Raw parameters to validate
 * @returns Validated and normalized parameters
 * @throws ZodError if validation fails
 */
export function validateContextParams(params: unknown): ContextParams {
  return ContextParamsSchema.parse(params)
}
