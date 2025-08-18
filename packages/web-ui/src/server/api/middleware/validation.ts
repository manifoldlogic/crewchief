import { type Request, type Response, type NextFunction } from 'express';
import { type ZodSchema, ZodError } from 'zod';
import { type ErrorResponse } from '../schemas/common.js';

// Validation middleware factory
export function validateRequest<T>(schema: ZodSchema<T>, property: 'body' | 'query' | 'params' = 'body') {
  return (req: Request, res: Response, next: NextFunction) => {
    try {
      const data = req[property];
      const validated = schema.parse(data);
      
      // Replace the original data with validated data
      (req as any)[property] = validated;
      
      next();
    } catch (error) {
      if (error instanceof ZodError) {
        const errorResponse: ErrorResponse = {
          success: false,
          error: {
            code: 'VALIDATION_ERROR',
            message: 'Request validation failed',
            details: {
              property,
              issues: error.issues.map(issue => ({
                path: issue.path.join('.'),
                message: issue.message,
                code: issue.code,
                received: issue.received || undefined,
              })),
            },
          },
          requestId: req.headers['x-request-id'] as string,
        };
        
        return res.status(400).json(errorResponse);
      }
      
      next(error);
    }
  };
}

// Body validation middleware
export function validateBody<T>(schema: ZodSchema<T>) {
  return validateRequest(schema, 'body');
}

// Query validation middleware
export function validateQuery<T>(schema: ZodSchema<T>) {
  return validateRequest(schema, 'query');
}

// Params validation middleware
export function validateParams<T>(schema: ZodSchema<T>) {
  return validateRequest(schema, 'params');
}

// Content type validation middleware
export function validateContentType(allowedTypes: string[] = ['application/json']) {
  return (req: Request, res: Response, next: NextFunction) => {
    const contentType = req.headers['content-type'];
    
    if (req.method === 'GET' || req.method === 'DELETE') {
      return next();
    }
    
    if (!contentType || !allowedTypes.some(type => contentType.includes(type))) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'INVALID_CONTENT_TYPE',
          message: `Content-Type must be one of: ${allowedTypes.join(', ')}`,
          details: { received: contentType, allowed: allowedTypes },
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(415).json(errorResponse);
    }
    
    next();
  };
}

// Request size validation middleware
export function validateRequestSize(maxSizeBytes: number = 10 * 1024 * 1024) { // 10MB default
  return (req: Request, res: Response, next: NextFunction) => {
    const contentLength = req.headers['content-length'];
    
    if (contentLength && parseInt(contentLength, 10) > maxSizeBytes) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'REQUEST_TOO_LARGE',
          message: `Request size exceeds maximum allowed size of ${maxSizeBytes} bytes`,
          details: { 
            received: parseInt(contentLength, 10), 
            maxAllowed: maxSizeBytes,
          },
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(413).json(errorResponse);
    }
    
    next();
  };
}

// Custom validation for specific business rules
export function validateBusinessRules() {
  return (req: Request, res: Response, next: NextFunction) => {
    const errors: string[] = [];
    
    // Example business rule validations
    if (req.method === 'POST' && req.path.includes('/runs')) {
      const body = req.body as any;
      
      // Validate commit SHA format
      if (body.commit_sha && !/^[a-f0-9]{40}$/i.test(body.commit_sha)) {
        errors.push('commit_sha must be a valid 40-character hexadecimal string');
      }
      
      // Validate agent_id format
      if (body.agent_id && body.agent_id.length > 255) {
        errors.push('agent_id must not exceed 255 characters');
      }
      
      // Validate task description length
      if (body.task_description && body.task_description.length < 10) {
        errors.push('task_description must be at least 10 characters long');
      }
    }
    
    if (req.method === 'POST' && req.path.includes('/worktrees')) {
      const body = req.body as any;
      
      // Validate worktree path format
      if (body.worktree_path && !body.worktree_path.startsWith('/')) {
        errors.push('worktree_path must be an absolute path');
      }
      
      // Validate branch name format
      if (body.current_branch && !/^[a-zA-Z0-9/_-]+$/.test(body.current_branch)) {
        errors.push('current_branch contains invalid characters');
      }
    }
    
    if (errors.length > 0) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'BUSINESS_RULE_VIOLATION',
          message: 'Business rule validation failed',
          details: { violations: errors },
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(400).json(errorResponse);
    }
    
    next();
  };
}

// SQL injection prevention (additional layer of protection)
export function preventSqlInjection() {
  return (req: Request, res: Response, next: NextFunction) => {
    const suspiciousPatterns = [
      /(\b(SELECT|INSERT|UPDATE|DELETE|DROP|CREATE|ALTER|EXEC|UNION)\b)/i,
      /(;|--|\|\|)/,
      /(\bOR\b.*=.*\bOR\b)/i,
      /(\bAND\b.*=.*\bAND\b)/i,
      /(script|javascript|vbscript)/i,
    ];
    
    const checkValue = (value: any, path: string = ''): string[] => {
      const violations: string[] = [];
      
      if (typeof value === 'string') {
        for (const pattern of suspiciousPatterns) {
          if (pattern.test(value)) {
            violations.push(`Suspicious pattern detected in ${path || 'request'}: ${pattern.source}`);
          }
        }
      } else if (typeof value === 'object' && value !== null) {
        for (const [key, val] of Object.entries(value)) {
          violations.push(...checkValue(val, path ? `${path}.${key}` : key));
        }
      }
      
      return violations;
    };
    
    const violations: string[] = [];
    violations.push(...checkValue(req.query, 'query'));
    violations.push(...checkValue(req.body, 'body'));
    violations.push(...checkValue(req.params, 'params'));
    
    if (violations.length > 0) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'SECURITY_VIOLATION',
          message: 'Request contains potentially dangerous content',
          details: { violations },
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(400).json(errorResponse);
    }
    
    next();
  };
}