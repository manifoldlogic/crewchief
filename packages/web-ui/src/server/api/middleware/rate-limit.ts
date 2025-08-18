import rateLimit from 'express-rate-limit';
import { type Request, type Response } from 'express';
import { type ErrorResponse } from '../schemas/common.js';

// Custom key generator for rate limiting
function generateKey(req: Request): string {
  // Use API key if available, otherwise fall back to IP + user agent
  const apiKey = req.headers['x-api-key'] as string;
  if (apiKey) {
    return `api-key:${apiKey}`;
  }
  
  // Use user ID if authenticated
  const user = (req as any).user;
  if (user?.id) {
    return `user:${user.id}`;
  }
  
  // Fall back to IP address
  const ip = req.ip || req.connection.remoteAddress || 'unknown';
  return `ip:${ip}`;
}

// Custom error handler for rate limiting
function rateLimitErrorHandler(req: Request, res: Response): void {
  const errorResponse: ErrorResponse = {
    success: false,
    error: {
      code: 'RATE_LIMIT_EXCEEDED',
      message: 'Too many requests. Please try again later.',
      details: {
        retryAfter: res.getHeader('Retry-After'),
        limit: res.getHeader('X-RateLimit-Limit'),
        remaining: res.getHeader('X-RateLimit-Remaining'),
        reset: res.getHeader('X-RateLimit-Reset'),
      },
    },
    requestId: req.headers['x-request-id'] as string,
  };
  
  res.status(429).json(errorResponse);
}

// Standard rate limiting for general API endpoints
export const standardRateLimit = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // Limit each key to 100 requests per windowMs
  keyGenerator: generateKey,
  standardHeaders: true, // Return rate limit info in the `RateLimit-*` headers
  legacyHeaders: false, // Disable the `X-RateLimit-*` headers
  handler: rateLimitErrorHandler,
  skip: (req) => {
    // Skip rate limiting in development mode if bypass header is present
    const isDevelopment = process.env.NODE_ENV === 'development';
    const bypassHeader = req.headers['x-bypass-rate-limit'] as string;
    return isDevelopment && bypassHeader === 'true';
  },
});

// Strict rate limiting for sensitive operations
export const strictRateLimit = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 20, // Limit each key to 20 requests per windowMs
  keyGenerator: generateKey,
  standardHeaders: true,
  legacyHeaders: false,
  handler: rateLimitErrorHandler,
  skip: (req) => {
    const isDevelopment = process.env.NODE_ENV === 'development';
    const bypassHeader = req.headers['x-bypass-rate-limit'] as string;
    return isDevelopment && bypassHeader === 'true';
  },
});

// Relaxed rate limiting for read-only operations
export const readOnlyRateLimit = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 300, // Limit each key to 300 requests per windowMs
  keyGenerator: generateKey,
  standardHeaders: true,
  legacyHeaders: false,
  handler: rateLimitErrorHandler,
  skip: (req) => {
    const isDevelopment = process.env.NODE_ENV === 'development';
    const bypassHeader = req.headers['x-bypass-rate-limit'] as string;
    return isDevelopment && bypassHeader === 'true';
  },
});

// Very strict rate limiting for authentication endpoints
export const authRateLimit = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 5, // Limit each key to 5 requests per windowMs
  keyGenerator: (req: Request) => {
    // For auth endpoints, always use IP address
    const ip = req.ip || req.connection.remoteAddress || 'unknown';
    return `auth-ip:${ip}`;
  },
  standardHeaders: true,
  legacyHeaders: false,
  handler: rateLimitErrorHandler,
  skip: (req) => {
    const isDevelopment = process.env.NODE_ENV === 'development';
    const bypassHeader = req.headers['x-bypass-rate-limit'] as string;
    return isDevelopment && bypassHeader === 'true';
  },
});

// Search rate limiting with burst allowance
export const searchRateLimit = rateLimit({
  windowMs: 60 * 1000, // 1 minute
  max: 30, // Limit each key to 30 searches per minute
  keyGenerator: generateKey,
  standardHeaders: true,
  legacyHeaders: false,
  handler: rateLimitErrorHandler,
  skip: (req) => {
    const isDevelopment = process.env.NODE_ENV === 'development';
    const bypassHeader = req.headers['x-bypass-rate-limit'] as string;
    return isDevelopment && bypassHeader === 'true';
  },
});

// Custom rate limiter factory for specific use cases
export function createCustomRateLimit(options: {
  windowMs: number;
  max: number;
  keyPrefix?: string;
  skipCondition?: (req: Request) => boolean;
}) {
  return rateLimit({
    windowMs: options.windowMs,
    max: options.max,
    keyGenerator: (req: Request) => {
      const baseKey = generateKey(req);
      return options.keyPrefix ? `${options.keyPrefix}:${baseKey}` : baseKey;
    },
    standardHeaders: true,
    legacyHeaders: false,
    handler: rateLimitErrorHandler,
    skip: (req) => {
      // Custom skip condition
      if (options.skipCondition && options.skipCondition(req)) {
        return true;
      }
      
      // Default development bypass
      const isDevelopment = process.env.NODE_ENV === 'development';
      const bypassHeader = req.headers['x-bypass-rate-limit'] as string;
      return isDevelopment && bypassHeader === 'true';
    },
  });
}

// Rate limiting for file upload endpoints
export const uploadRateLimit = createCustomRateLimit({
  windowMs: 60 * 60 * 1000, // 1 hour
  max: 10, // 10 uploads per hour
  keyPrefix: 'upload',
});

// Rate limiting for export endpoints
export const exportRateLimit = createCustomRateLimit({
  windowMs: 60 * 60 * 1000, // 1 hour
  max: 5, // 5 exports per hour
  keyPrefix: 'export',
});

// Rate limiting for bulk operations
export const bulkOperationRateLimit = createCustomRateLimit({
  windowMs: 60 * 60 * 1000, // 1 hour
  max: 3, // 3 bulk operations per hour
  keyPrefix: 'bulk',
});

// Dynamic rate limiting based on user tier/permissions
export function dynamicRateLimit() {
  return (req: Request, res: Response, next: Function) => {
    const user = (req as any).user;
    
    // Default limits
    let windowMs = 15 * 60 * 1000; // 15 minutes
    let max = 100;
    
    // Adjust limits based on user permissions
    if (user?.permissions?.includes('admin')) {
      max = 1000; // Admins get higher limits
    } else if (user?.permissions?.includes('premium')) {
      max = 300; // Premium users get higher limits
    } else if (user?.permissions?.includes('api_access')) {
      max = 200; // API users get moderate limits
    }
    
    // Create dynamic rate limiter
    const dynamicLimiter = rateLimit({
      windowMs,
      max,
      keyGenerator: generateKey,
      standardHeaders: true,
      legacyHeaders: false,
      handler: rateLimitErrorHandler,
      skip: (req) => {
        const isDevelopment = process.env.NODE_ENV === 'development';
        const bypassHeader = req.headers['x-bypass-rate-limit'] as string;
        return isDevelopment && bypassHeader === 'true';
      },
    });
    
    dynamicLimiter(req, res, next);
  };
}

// Utility function to reset rate limits for a specific key (admin function)
export function resetRateLimit(key: string) {
  // This would require access to the rate limiter store
  // Implementation depends on the store being used (memory, Redis, etc.)
  console.log(`Rate limit reset requested for key: ${key}`);
  // TODO: Implement actual reset logic when store is configured
}

// Middleware to add rate limit headers to all responses
export function addRateLimitHeaders() {
  return (req: Request, res: Response, next: Function) => {
    // Add custom headers for rate limit visibility
    res.setHeader('X-API-Version', '1.0');
    res.setHeader('X-Rate-Limit-Policy', 'https://docs.crewchief.dev/api/rate-limits');
    
    next();
  };
}