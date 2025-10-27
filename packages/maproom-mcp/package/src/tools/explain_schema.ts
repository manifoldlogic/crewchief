/**
 * Zod schema for Explain tool parameter validation
 */

import { z } from 'zod'

/**
 * Schema for Explain tool parameters
 */
export const ExplainParamsSchema = z.object({
  chunk_id: z.union([
    z.string().min(1, 'chunk_id is required and cannot be empty'),
    z.number().int().positive('chunk_id must be a positive integer'),
  ]).transform((val) => {
    // Convert to number if it's a string
    const num = typeof val === 'string' ? parseInt(val, 10) : val
    if (isNaN(num) || num <= 0) {
      throw new Error('chunk_id must be a valid positive integer')
    }
    return num
  }),
})

/**
 * Validate Explain tool parameters
 * @param params - Parameters to validate
 * @returns Validated parameters with chunk_id as number
 * @throws ZodError if validation fails
 */
export function validateExplainParams(params: unknown) {
  return ExplainParamsSchema.parse(params)
}
