import { type Request, type Response, type NextFunction } from 'express';
import jwt from 'jsonwebtoken';
import { type ErrorResponse } from '../schemas/common.js';
import { JWTService, type AccessTokenPayload } from '../../auth/services/jwt.js';

// Extended Request interface to include user information
export interface AuthenticatedRequest extends Request {
  user?: {
    id: number;
    uuid: string;
    email: string;
    sessionId: string;
    permissions: string[];
    roles: string[];
    expiresAt?: Date;
  };
}

// JWT secret (should be loaded from environment variables)
const JWT_SECRET = process.env.JWT_SECRET || 'dev-secret-key-change-in-production';
const API_KEY_HEADER = 'x-api-key';
const BEARER_TOKEN_HEADER = 'authorization';

// Valid API keys (in production, these should be stored in database/env)
const VALID_API_KEYS = new Set([
  process.env.API_KEY_1,
  process.env.API_KEY_2,
  'dev-api-key-1', // Development only
  'dev-api-key-2', // Development only
].filter(Boolean));

// API Key authentication middleware
export function authenticateApiKey() {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
    const apiKey = req.headers[API_KEY_HEADER] as string;
    
    if (!apiKey) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'MISSING_API_KEY',
          message: `API key required in ${API_KEY_HEADER} header`,
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(401).json(errorResponse);
    }
    
    if (!VALID_API_KEYS.has(apiKey)) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'INVALID_API_KEY',
          message: 'Invalid API key provided',
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(401).json(errorResponse);
    }
    
    // Set user context for API key authentication
    req.user = {
      id: 0, // Special ID for API keys
      uuid: `api-key-${apiKey.slice(-8)}`,
      email: `api-key-${apiKey.slice(-8)}@api.local`,
      sessionId: `api-session-${Date.now()}`,
      permissions: ['api_access'],
      roles: ['api'],
    };
    
    next();
  };
}

// JWT token authentication middleware
export function authenticateJWT() {
  return async (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
    // Try Authorization header first
    const authHeader = req.headers[BEARER_TOKEN_HEADER] as string;
    let token: string | undefined;

    if (authHeader && authHeader.startsWith('Bearer ')) {
      token = authHeader.slice(7); // Remove 'Bearer ' prefix
    } else if (req.cookies?.accessToken) {
      // Fallback to cookie-based authentication
      token = req.cookies.accessToken;
    }
    
    if (!token) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'MISSING_TOKEN',
          message: 'Bearer token required in Authorization header or accessToken cookie',
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(401).json(errorResponse);
    }
    
    try {
      // Use the new JWT service for verification
      const jwtService = new JWTService(req.app.locals.db);
      const decoded = jwtService.verifyAccessToken(token);
      
      if (!decoded) {
        const errorResponse: ErrorResponse = {
          success: false,
          error: {
            code: 'INVALID_TOKEN',
            message: 'Invalid JWT token',
          },
          requestId: req.headers['x-request-id'] as string,
        };
        
        return res.status(401).json(errorResponse);
      }
      
      req.user = {
        id: decoded.userId,
        uuid: decoded.userUuid,
        email: decoded.email,
        sessionId: decoded.sessionId,
        permissions: decoded.permissions || [],
        roles: decoded.roles || [],
        expiresAt: decoded.exp ? new Date(decoded.exp * 1000) : undefined,
      };
      
      next();
    } catch (error) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'INVALID_TOKEN',
          message: error instanceof Error ? error.message : 'Invalid JWT token',
          details: { error: error instanceof Error ? error.message : 'Unknown error' },
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(401).json(errorResponse);
    }
  };
}

// Flexible authentication middleware (supports both API key and JWT)
export function authenticateRequest() {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
    const apiKey = req.headers[API_KEY_HEADER] as string;
    const authHeader = req.headers[BEARER_TOKEN_HEADER] as string;
    
    // Try API key authentication first
    if (apiKey) {
      return authenticateApiKey()(req, res, next);
    }
    
    // Try JWT authentication
    if (authHeader) {
      return authenticateJWT()(req, res, next);
    }
    
    // No authentication provided
    const errorResponse: ErrorResponse = {
      success: false,
      error: {
        code: 'AUTHENTICATION_REQUIRED',
        message: 'Authentication required. Provide either API key or Bearer token.',
        details: {
          methods: ['API key in x-api-key header', 'JWT token in Authorization header'],
        },
      },
      requestId: req.headers['x-request-id'] as string,
    };
    
    return res.status(401).json(errorResponse);
  };
}

// Permission-based authorization middleware
export function requirePermissions(requiredPermissions: string[]) {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
    if (!req.user) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'AUTHENTICATION_REQUIRED',
          message: 'User authentication required',
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(401).json(errorResponse);
    }
    
    const userPermissions = req.user.permissions || [];
    const userRoles = req.user.roles || [];
    
    // Check if user has admin role (grants all permissions)
    if (userRoles.includes('admin') || userPermissions.includes('*')) {
      return next();
    }
    
    // Check specific permissions
    const hasRequiredPermissions = requiredPermissions.every(permission => 
      userPermissions.includes(permission)
    );
    
    if (!hasRequiredPermissions) {
      const errorResponse: ErrorResponse = {
        success: false,
        error: {
          code: 'INSUFFICIENT_PERMISSIONS',
          message: 'Insufficient permissions for this operation',
          details: {
            required: requiredPermissions,
            user: userPermissions,
            roles: userRoles,
          },
        },
        requestId: req.headers['x-request-id'] as string,
      };
      
      return res.status(403).json(errorResponse);
    }
    
    next();
  };
}

// Optional authentication middleware (doesn't fail if no auth provided)
export function optionalAuthentication() {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
    const apiKey = req.headers[API_KEY_HEADER] as string;
    const authHeader = req.headers[BEARER_TOKEN_HEADER] as string;
    
    if (!apiKey && !authHeader) {
      return next(); // No authentication provided, continue without user context
    }
    
    // Use the main authentication middleware but catch errors
    authenticateRequest()(req, res, (error) => {
      if (error) {
        // Log authentication failure but continue without user context
        console.warn('Optional authentication failed:', error);
      }
      next(); // Always continue
    });
  };
}

// Development mode bypass (for testing purposes)
export function bypassAuthInDev() {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
    const isDevelopment = process.env.NODE_ENV === 'development';
    const bypassHeader = req.headers['x-bypass-auth'] as string;
    
    if (isDevelopment && bypassHeader === 'true') {
      req.user = {
        id: 999999,
        uuid: 'dev-user-uuid',
        email: 'dev@crewchief.local',
        sessionId: 'dev-session',
        permissions: ['*'],
        roles: ['admin'],
      };
      
      console.warn('🚨 Authentication bypassed in development mode');
      return next();
    }
    
    next();
  };
}

// Utility function to generate JWT tokens (for testing/development)
export function generateJWTToken(payload: {
  userId: string;
  sessionId: string;
  permissions?: string[];
  expiresIn?: string;
}) {
  return jwt.sign(
    {
      userId: payload.userId,
      sessionId: payload.sessionId,
      permissions: payload.permissions || [],
    },
    JWT_SECRET,
    {
      expiresIn: payload.expiresIn || '24h',
      issuer: 'crewchief-web-ui',
      audience: 'crewchief-api',
    }
  );
}

// Utility function to verify and decode JWT tokens
export function verifyJWTToken(token: string) {
  try {
    return jwt.verify(token, JWT_SECRET) as any;
  } catch (error) {
    throw new Error(`Invalid JWT token: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}