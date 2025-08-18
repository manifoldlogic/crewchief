import { type Response } from 'express';
import { 
  type SuccessResponse, 
  type ErrorResponse, 
  type PaginationResponse 
} from '../schemas/common.js';

// Success response utility
export function sendSuccess<T>(
  res: Response, 
  data: T, 
  message?: string, 
  statusCode: number = 200
): void {
  const response: SuccessResponse<T> = {
    success: true,
    data,
    message,
  };
  
  res.status(statusCode).json(response);
}

// Error response utility
export function sendError(
  res: Response,
  code: string,
  message: string,
  statusCode: number = 400,
  details?: any,
  requestId?: string
): void {
  const response: ErrorResponse = {
    success: false,
    error: {
      code,
      message,
      details,
    },
    requestId,
  };
  
  res.status(statusCode).json(response);
}

// Paginated response utility
export function sendPaginatedSuccess<T>(
  res: Response,
  paginatedData: PaginationResponse<T>,
  message?: string,
  statusCode: number = 200
): void {
  const response: SuccessResponse<PaginationResponse<T>> = {
    success: true,
    data: paginatedData,
    message,
  };
  
  res.status(statusCode).json(response);
}

// Created response utility (201 status)
export function sendCreated<T>(res: Response, data: T, message?: string): void {
  sendSuccess(res, data, message || 'Resource created successfully', 201);
}

// No content response utility (204 status)
export function sendNoContent(res: Response): void {
  res.status(204).send();
}

// Not found error utility
export function sendNotFound(
  res: Response, 
  resource: string = 'Resource', 
  id?: string | number,
  requestId?: string
): void {
  const message = id 
    ? `${resource} with ID '${id}' not found`
    : `${resource} not found`;
    
  sendError(res, 'NOT_FOUND', message, 404, { resource, id }, requestId);
}

// Bad request error utility
export function sendBadRequest(
  res: Response,
  message: string,
  details?: any,
  requestId?: string
): void {
  sendError(res, 'BAD_REQUEST', message, 400, details, requestId);
}

// Validation error utility
export function sendValidationError(
  res: Response,
  errors: Array<{ field: string; message: string }>,
  requestId?: string
): void {
  sendError(
    res,
    'VALIDATION_ERROR',
    'Request validation failed',
    400,
    { validationErrors: errors },
    requestId
  );
}

// Unauthorized error utility
export function sendUnauthorized(
  res: Response,
  message: string = 'Authentication required',
  requestId?: string
): void {
  sendError(res, 'UNAUTHORIZED', message, 401, undefined, requestId);
}

// Forbidden error utility
export function sendForbidden(
  res: Response,
  message: string = 'Access forbidden',
  requestId?: string
): void {
  sendError(res, 'FORBIDDEN', message, 403, undefined, requestId);
}

// Conflict error utility
export function sendConflict(
  res: Response,
  message: string,
  details?: any,
  requestId?: string
): void {
  sendError(res, 'CONFLICT', message, 409, details, requestId);
}

// Internal server error utility
export function sendInternalError(
  res: Response,
  message: string = 'Internal server error',
  details?: any,
  requestId?: string
): void {
  // Log internal errors for debugging
  console.error('Internal server error:', { message, details, requestId });
  
  // Don't expose internal error details in production
  const isDevelopment = process.env.NODE_ENV === 'development';
  const errorDetails = isDevelopment ? details : undefined;
  
  sendError(res, 'INTERNAL_ERROR', message, 500, errorDetails, requestId);
}

// Rate limit error utility
export function sendRateLimitError(
  res: Response,
  retryAfter: number,
  requestId?: string
): void {
  res.setHeader('Retry-After', retryAfter);
  sendError(
    res,
    'RATE_LIMIT_EXCEEDED',
    'Too many requests. Please try again later.',
    429,
    { retryAfter },
    requestId
  );
}

// Service unavailable error utility
export function sendServiceUnavailable(
  res: Response,
  message: string = 'Service temporarily unavailable',
  requestId?: string
): void {
  sendError(res, 'SERVICE_UNAVAILABLE', message, 503, undefined, requestId);
}

// Async error handler wrapper
export function asyncHandler(
  fn: (req: any, res: Response, next: any) => Promise<any>
) {
  return (req: any, res: Response, next: any) => {
    Promise.resolve(fn(req, res, next)).catch((error) => {
      // Extract request ID for error tracking
      const requestId = req.headers['x-request-id'] as string;
      
      // Handle different types of errors
      if (error.name === 'ValidationError') {
        sendValidationError(res, error.details || [], requestId);
      } else if (error.code === '23505') { // PostgreSQL unique violation
        sendConflict(res, 'Resource already exists', { constraint: error.constraint }, requestId);
      } else if (error.code === '23503') { // PostgreSQL foreign key violation
        sendBadRequest(res, 'Invalid reference to related resource', { constraint: error.constraint }, requestId);
      } else if (error.code === '23502') { // PostgreSQL not null violation
        sendBadRequest(res, 'Required field is missing', { column: error.column }, requestId);
      } else {
        // Generic internal server error
        sendInternalError(res, error.message, error.stack, requestId);
      }
    });
  };
}

// Response time middleware
export function responseTime() {
  return (req: any, res: Response, next: any) => {
    const startTime = Date.now();
    
    res.on('finish', () => {
      const duration = Date.now() - startTime;
      res.setHeader('X-Response-Time', `${duration}ms`);
      
      // Log slow responses
      if (duration > 200) {
        console.warn(`Slow response detected (${duration}ms):`, {
          method: req.method,
          path: req.path,
          statusCode: res.statusCode,
        });
      }
    });
    
    next();
  };
}

// Cache control utility
export function setCacheHeaders(
  res: Response,
  maxAge: number = 300, // 5 minutes default
  isPrivate: boolean = false
): void {
  const cacheControl = isPrivate 
    ? `private, max-age=${maxAge}`
    : `public, max-age=${maxAge}`;
    
  res.setHeader('Cache-Control', cacheControl);
  res.setHeader('ETag', `"${Date.now()}"`);
}

// CORS headers utility
export function setCorsHeaders(res: Response): void {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, x-api-key, x-request-id');
  res.setHeader('Access-Control-Expose-Headers', 'X-Total-Count, X-Response-Time, X-RateLimit-*');
}

// Security headers utility
export function setSecurityHeaders(res: Response): void {
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.setHeader('X-Frame-Options', 'DENY');
  res.setHeader('X-XSS-Protection', '1; mode=block');
  res.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin');
}

// API version header utility
export function setApiVersionHeaders(res: Response, version: string = '1.0'): void {
  res.setHeader('X-API-Version', version);
  res.setHeader('X-Powered-By', 'CrewChief Web UI');
}

// Request ID utility
export function generateRequestId(): string {
  return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

// Add request ID middleware
export function addRequestId() {
  return (req: any, res: Response, next: any) => {
    const requestId = req.headers['x-request-id'] || generateRequestId();
    req.headers['x-request-id'] = requestId;
    res.setHeader('X-Request-ID', requestId);
    next();
  };
}

// Health check response utility
export function sendHealthCheck(res: Response, checks: Record<string, any> = {}): void {
  const health = {
    status: 'healthy',
    timestamp: new Date().toISOString(),
    uptime: process.uptime(),
    version: process.env.npm_package_version || '1.0.0',
    environment: process.env.NODE_ENV || 'development',
    checks,
  };
  
  sendSuccess(res, health, 'System is healthy');
}

// API info response utility
export function sendApiInfo(res: Response): void {
  const info = {
    name: 'CrewChief Web UI API',
    version: '1.0.0',
    description: 'REST API for CrewChief Web UI',
    documentation: '/api/docs',
    endpoints: {
      health: '/api/health',
      worktrees: '/api/worktrees',
      agents: '/api/agents',
      runs: '/api/runs',
      config: '/api/config',
    },
    rateLimit: {
      standard: '100 requests per 15 minutes',
      search: '30 requests per minute',
      auth: '5 requests per 15 minutes',
    },
  };
  
  sendSuccess(res, info, 'API information');
}