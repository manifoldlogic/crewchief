import { z } from 'zod';

// Common validation schemas
export const IdSchema = z.string().min(1);
export const UuidSchema = z.string().uuid();
export const TimestampSchema = z.string().datetime();
export const PositiveIntSchema = z.number().int().positive();
export const NonNegativeIntSchema = z.number().int().min(0);

// Pagination schemas
export const PaginationQuerySchema = z.object({
  limit: z.coerce.number().int().min(1).max(100).default(20),
  offset: z.coerce.number().int().min(0).default(0),
  cursor: z.string().optional(),
  sort: z.string().optional(),
  order: z.enum(['asc', 'desc']).default('desc'),
});

export const PaginationResponseSchema = z.object({
  items: z.array(z.any()),
  pagination: z.object({
    total: z.number().int().min(0),
    limit: z.number().int().min(1),
    offset: z.number().int().min(0),
    hasMore: z.boolean(),
    nextCursor: z.string().optional(),
    prevCursor: z.string().optional(),
  }),
});

// Filter schemas
export const DateRangeFilterSchema = z.object({
  from: TimestampSchema.optional(),
  to: TimestampSchema.optional(),
});

export const SearchFilterSchema = z.object({
  query: z.string().min(1).max(255).optional(),
  fields: z.array(z.string()).optional(),
});

// Common response schemas
export const SuccessResponseSchema = z.object({
  success: z.literal(true),
  data: z.any(),
  message: z.string().optional(),
});

export const ErrorResponseSchema = z.object({
  success: z.literal(false),
  error: z.object({
    code: z.string(),
    message: z.string(),
    details: z.any().optional(),
  }),
  requestId: z.string().optional(),
});

export const ApiResponseSchema = z.union([SuccessResponseSchema, ErrorResponseSchema]);

// Type exports
export type PaginationQuery = z.infer<typeof PaginationQuerySchema>;
export type PaginationResponse<T> = {
  items: T[];
  pagination: {
    total: number;
    limit: number;
    offset: number;
    hasMore: boolean;
    nextCursor?: string;
    prevCursor?: string;
  };
};

export type SuccessResponse<T = any> = {
  success: true;
  data: T;
  message?: string;
};

export type ErrorResponse = {
  success: false;
  error: {
    code: string;
    message: string;
    details?: any;
  };
  requestId?: string;
};

export type ApiResponse<T = any> = SuccessResponse<T> | ErrorResponse;