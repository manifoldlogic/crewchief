import { type Request, type Response, type NextFunction } from 'express';
import crypto from 'crypto';

// CSRF configuration
const CSRF_TOKEN_LENGTH = 32;
const CSRF_HEADER_NAME = 'x-csrf-token';
const CSRF_COOKIE_NAME = 'csrf-token';
const CSRF_SECRET_LENGTH = 64;

// Store CSRF secrets (in production, use Redis or database)
const csrfSecrets = new Map<string, { secret: string; expires: number }>();

export interface CSRFProtectedRequest extends Request {
  csrfToken?: string;
  generateCSRFToken?: () => string;
}

// Generate a cryptographically secure random token
function generateSecureToken(length: number): string {
  return crypto.randomBytes(length).toString('hex');
}

// Generate CSRF secret for session
function generateCSRFSecret(): string {
  return generateSecureToken(CSRF_SECRET_LENGTH);
}

// Generate CSRF token from secret
function generateCSRFToken(secret: string): string {
  const timestamp = Date.now().toString();
  const hmac = crypto.createHmac('sha256', secret);
  hmac.update(timestamp);
  const signature = hmac.digest('hex');
  return `${timestamp}.${signature}`;
}

// Verify CSRF token
function verifyCSRFToken(token: string, secret: string): boolean {
  try {
    const [timestamp, signature] = token.split('.');
    
    if (!timestamp || !signature) {
      return false;
    }

    // Check token age (max 1 hour)
    const tokenAge = Date.now() - parseInt(timestamp);
    if (tokenAge > 60 * 60 * 1000) {
      return false;
    }

    // Verify signature
    const hmac = crypto.createHmac('sha256', secret);
    hmac.update(timestamp);
    const expectedSignature = hmac.digest('hex');
    
    return crypto.timingSafeEqual(
      Buffer.from(signature, 'hex'),
      Buffer.from(expectedSignature, 'hex')
    );
  } catch (error) {
    return false;
  }
}

// Get or create CSRF secret for session
function getOrCreateCSRFSecret(sessionId: string): string {
  const existing = csrfSecrets.get(sessionId);
  
  if (existing && existing.expires > Date.now()) {
    return existing.secret;
  }

  const secret = generateCSRFSecret();
  const expires = Date.now() + (24 * 60 * 60 * 1000); // 24 hours
  
  csrfSecrets.set(sessionId, { secret, expires });
  
  // Cleanup expired secrets
  cleanupExpiredSecrets();
  
  return secret;
}

// Clean up expired CSRF secrets
function cleanupExpiredSecrets(): void {
  const now = Date.now();
  for (const [sessionId, data] of csrfSecrets.entries()) {
    if (data.expires <= now) {
      csrfSecrets.delete(sessionId);
    }
  }
}

// CSRF protection middleware
export function csrfProtection(options: {
  ignoreMethods?: string[];
  cookieOptions?: {
    httpOnly?: boolean;
    secure?: boolean;
    sameSite?: 'strict' | 'lax' | 'none';
    domain?: string;
    path?: string;
  };
} = {}) {
  const {
    ignoreMethods = ['GET', 'HEAD', 'OPTIONS'],
    cookieOptions = {}
  } = options;

  return (req: CSRFProtectedRequest, res: Response, next: NextFunction) => {
    const sessionId = req.sessionID || req.session?.id || generateSecureToken(16);
    const method = req.method.toUpperCase();
    
    // Get or create CSRF secret
    const secret = getOrCreateCSRFSecret(sessionId);
    
    // Generate new token for this request
    const token = generateCSRFToken(secret);
    req.csrfToken = token;
    
    // Add token generator function to request
    req.generateCSRFToken = () => generateCSRFToken(secret);
    
    // Set CSRF token in cookie for client access
    const defaultCookieOptions = {
      httpOnly: false, // Allow client-side access for AJAX requests
      secure: process.env.NODE_ENV === 'production',
      sameSite: process.env.NODE_ENV === 'production' ? 'strict' : 'lax',
      maxAge: 60 * 60 * 1000, // 1 hour
      path: '/',
      ...cookieOptions,
    };
    
    res.cookie(CSRF_COOKIE_NAME, token, defaultCookieOptions as any);
    
    // Add CSRF token to response locals for template rendering
    res.locals.csrfToken = token;
    
    // Skip validation for safe methods
    if (ignoreMethods.includes(method)) {
      return next();
    }
    
    // Get token from header or body
    const submittedToken = 
      req.headers[CSRF_HEADER_NAME] as string ||
      req.body?._csrf ||
      req.query?._csrf as string;
    
    if (!submittedToken) {
      return res.status(403).json({
        success: false,
        error: {
          code: 'CSRF_TOKEN_MISSING',
          message: 'CSRF token missing from request',
        },
      });
    }
    
    // Verify the submitted token
    if (!verifyCSRFToken(submittedToken, secret)) {
      return res.status(403).json({
        success: false,
        error: {
          code: 'CSRF_TOKEN_INVALID',
          message: 'Invalid or expired CSRF token',
        },
      });
    }
    
    next();
  };
}

// Double-submit cookie pattern (additional security)
export function doubleSubmitCookie(options: {
  cookieName?: string;
  headerName?: string;
  secure?: boolean;
} = {}) {
  const {
    cookieName = 'double-submit-token',
    headerName = 'x-double-submit-token',
    secure = process.env.NODE_ENV === 'production',
  } = options;

  return (req: Request, res: Response, next: NextFunction) => {
    const method = req.method.toUpperCase();
    
    // Generate token for safe methods
    if (['GET', 'HEAD', 'OPTIONS'].includes(method)) {
      const token = generateSecureToken(32);
      res.cookie(cookieName, token, {
        httpOnly: false,
        secure,
        sameSite: secure ? 'strict' : 'lax',
        maxAge: 60 * 60 * 1000, // 1 hour
      });
      return next();
    }
    
    // Verify token for unsafe methods
    const cookieToken = req.cookies[cookieName];
    const headerToken = req.headers[headerName] as string;
    
    if (!cookieToken || !headerToken) {
      return res.status(403).json({
        success: false,
        error: {
          code: 'DOUBLE_SUBMIT_TOKEN_MISSING',
          message: 'Double-submit token missing',
        },
      });
    }
    
    if (!crypto.timingSafeEqual(
      Buffer.from(cookieToken),
      Buffer.from(headerToken)
    )) {
      return res.status(403).json({
        success: false,
        error: {
          code: 'DOUBLE_SUBMIT_TOKEN_MISMATCH',
          message: 'Double-submit token mismatch',
        },
      });
    }
    
    next();
  };
}

// Origin validation middleware
export function validateOrigin(allowedOrigins: string[] = []) {
  const defaultAllowedOrigins = [
    process.env.FRONTEND_URL || 'http://localhost:3000',
    process.env.BASE_URL || 'http://localhost:3456',
    'http://localhost:3000',
    'http://localhost:3456',
    'http://localhost:5173', // Vite dev server
  ];

  const origins = [...defaultAllowedOrigins, ...allowedOrigins];

  return (req: Request, res: Response, next: NextFunction) => {
    const origin = req.headers.origin;
    const referer = req.headers.referer;
    const method = req.method.toUpperCase();

    // Skip validation for safe methods from same origin
    if (['GET', 'HEAD', 'OPTIONS'].includes(method)) {
      return next();
    }

    // Check origin header
    if (origin) {
      const isAllowed = origins.some(allowed => 
        origin === allowed || origin.startsWith(allowed)
      );

      if (!isAllowed) {
        return res.status(403).json({
          success: false,
          error: {
            code: 'INVALID_ORIGIN',
            message: 'Request origin not allowed',
          },
        });
      }
    } else if (referer) {
      // Fallback to referer if origin not available
      const isAllowed = origins.some(allowed => 
        referer.startsWith(allowed)
      );

      if (!isAllowed) {
        return res.status(403).json({
          success: false,
          error: {
            code: 'INVALID_REFERER',
            message: 'Request referer not allowed',
          },
        });
      }
    } else {
      // No origin or referer for state-changing request
      return res.status(403).json({
        success: false,
        error: {
          code: 'MISSING_ORIGIN',
          message: 'Origin or referer header required',
        },
      });
    }

    next();
  };
}

// Secure headers middleware
export function secureHeaders() {
  return (req: Request, res: Response, next: NextFunction) => {
    // Content Security Policy
    res.setHeader('Content-Security-Policy', [
      "default-src 'self'",
      "script-src 'self' 'unsafe-inline' 'unsafe-eval'", // Allow inline scripts for development
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: https:",
      "font-src 'self' data:",
      "connect-src 'self' ws: wss:",
      "frame-ancestors 'none'",
    ].join('; '));

    // X-Frame-Options
    res.setHeader('X-Frame-Options', 'DENY');

    // X-Content-Type-Options
    res.setHeader('X-Content-Type-Options', 'nosniff');

    // Referrer Policy
    res.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin');

    // X-XSS-Protection
    res.setHeader('X-XSS-Protection', '1; mode=block');

    // Strict-Transport-Security (HTTPS only)
    if (req.secure || req.headers['x-forwarded-proto'] === 'https') {
      res.setHeader('Strict-Transport-Security', 'max-age=31536000; includeSubDomains; preload');
    }

    // Permissions Policy
    res.setHeader('Permissions-Policy', [
      'geolocation=()',
      'microphone=()',
      'camera=()',
      'payment=()',
      'usb=()',
    ].join(', '));

    next();
  };
}

// Cookie security configuration
export function secureCookies() {
  return (req: Request, res: Response, next: NextFunction) => {
    const isProduction = process.env.NODE_ENV === 'production';
    const isSecure = req.secure || req.headers['x-forwarded-proto'] === 'https';

    // Override res.cookie to apply security defaults
    const originalCookie = res.cookie;
    res.cookie = function(name: string, value: any, options: any = {}) {
      const secureOptions = {
        httpOnly: true,
        secure: isProduction && isSecure,
        sameSite: isProduction ? 'strict' : 'lax',
        ...options,
      };
      
      return originalCookie.call(this, name, value, secureOptions);
    };

    next();
  };
}

// Rate limiting for CSRF token generation
const csrfTokenRequests = new Map<string, { count: number; resetTime: number }>();

export function csrfTokenRateLimit(maxRequests = 100, windowMs = 60 * 60 * 1000) {
  return (req: Request, res: Response, next: NextFunction) => {
    const identifier = req.ip || 'unknown';
    const now = Date.now();
    const entry = csrfTokenRequests.get(identifier);

    if (!entry || now > entry.resetTime) {
      csrfTokenRequests.set(identifier, { count: 1, resetTime: now + windowMs });
      return next();
    }

    if (entry.count >= maxRequests) {
      return res.status(429).json({
        success: false,
        error: {
          code: 'CSRF_TOKEN_RATE_LIMIT',
          message: 'Too many CSRF token requests',
        },
      });
    }

    entry.count++;
    next();
  };
}

// Cleanup function for CSRF secrets (call periodically)
export function cleanupCSRFSecrets(): number {
  const now = Date.now();
  let cleaned = 0;
  
  for (const [sessionId, data] of csrfSecrets.entries()) {
    if (data.expires <= now) {
      csrfSecrets.delete(sessionId);
      cleaned++;
    }
  }
  
  return cleaned;
}

// CSRF token endpoint
export function csrfTokenEndpoint() {
  return (req: CSRFProtectedRequest, res: Response) => {
    res.json({
      success: true,
      csrfToken: req.csrfToken,
    });
  };
}