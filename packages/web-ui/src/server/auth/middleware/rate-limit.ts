import rateLimit from 'express-rate-limit';
import { type Request, type Response, type NextFunction } from 'express';
import { Pool } from 'pg';

// Rate limit configuration
export interface RateLimitConfig {
  windowMs: number; // Time window in milliseconds
  max: number; // Maximum requests per window
  skipSuccessfulRequests?: boolean;
  skipFailedRequests?: boolean;
  keyGenerator?: (req: Request) => string;
}

// Default rate limits for different endpoints
export const RATE_LIMITS = {
  login: {
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 10, // 10 attempts per 15 minutes per IP
    skipSuccessfulRequests: true,
  },
  register: {
    windowMs: 60 * 60 * 1000, // 1 hour
    max: 5, // 5 registration attempts per hour per IP
    skipSuccessfulRequests: true,
  },
  passwordReset: {
    windowMs: 60 * 60 * 1000, // 1 hour
    max: 3, // 3 password reset attempts per hour per IP
    skipSuccessfulRequests: true,
  },
  refresh: {
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 50, // 50 refresh attempts per 15 minutes per IP
    skipSuccessfulRequests: true,
  },
  api: {
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 1000, // 1000 API requests per 15 minutes per IP
    skipSuccessfulRequests: true,
  },
} as const;

export class RateLimitService {
  constructor(private db: Pool) {}

  // Check if identifier is rate limited
  async checkRateLimit(
    identifierType: 'ip' | 'email' | 'user_id',
    identifierValue: string,
    endpoint: string,
    maxAttempts: number,
    windowMs: number
  ): Promise<boolean> {
    try {
      const windowStart = new Date(Date.now() - windowMs);
      
      // Get current rate limit entry
      const result = await this.db.query(`
        SELECT * FROM auth_rate_limits
        WHERE identifier_type = $1 AND identifier_value = $2 AND endpoint = $3
      `, [identifierType, identifierValue, endpoint]);

      if (result.rows.length === 0) {
        // No existing entry, create one
        await this.db.query(`
          INSERT INTO auth_rate_limits (identifier_type, identifier_value, endpoint, attempt_count, window_start)
          VALUES ($1, $2, $3, 1, NOW())
        `, [identifierType, identifierValue, endpoint]);
        return false;
      }

      const rateLimitEntry = result.rows[0];
      const windowStartTime = new Date(rateLimitEntry.window_start);
      const now = new Date();

      // Check if we're in a new window
      if (now.getTime() - windowStartTime.getTime() > windowMs) {
        // Reset the window
        await this.db.query(`
          UPDATE auth_rate_limits
          SET attempt_count = 1, window_start = NOW(), blocked_until = NULL
          WHERE identifier_type = $1 AND identifier_value = $2 AND endpoint = $3
        `, [identifierType, identifierValue, endpoint]);
        return false;
      }

      // Check if currently blocked
      if (rateLimitEntry.blocked_until && new Date(rateLimitEntry.blocked_until) > now) {
        return true;
      }

      // Check if we've exceeded the limit
      if (rateLimitEntry.attempt_count >= maxAttempts) {
        // Block for the remainder of the window
        const blockedUntil = new Date(windowStartTime.getTime() + windowMs);
        await this.db.query(`
          UPDATE auth_rate_limits
          SET blocked_until = $1
          WHERE identifier_type = $2 AND identifier_value = $3 AND endpoint = $4
        `, [blockedUntil, identifierType, identifierValue, endpoint]);
        return true;
      }

      // Increment attempt count
      await this.db.query(`
        UPDATE auth_rate_limits
        SET attempt_count = attempt_count + 1
        WHERE identifier_type = $1 AND identifier_value = $2 AND endpoint = $3
      `, [identifierType, identifierValue, endpoint]);

      return false;

    } catch (error) {
      console.error('Rate limit check error:', error);
      // On error, don't block (fail open)
      return false;
    }
  }

  // Record login attempt for auditing
  async recordLoginAttempt(
    email: string,
    userId: number | null,
    ipAddress: string,
    userAgent: string,
    attemptType: 'password' | 'oauth' | '2fa',
    success: boolean,
    failureReason?: string,
    providerName?: string
  ): Promise<void> {
    try {
      await this.db.query(`
        INSERT INTO auth_login_attempts (
          email, user_id, ip_address, user_agent, attempt_type, 
          provider_name, success, failure_reason
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      `, [
        email,
        userId,
        ipAddress,
        userAgent,
        attemptType,
        providerName || null,
        success,
        failureReason || null,
      ]);
    } catch (error) {
      console.error('Failed to record login attempt:', error);
    }
  }

  // Clean up old rate limit entries
  async cleanupOldRateLimits(): Promise<number> {
    try {
      const result = await this.db.query(`
        DELETE FROM auth_rate_limits 
        WHERE window_start < NOW() - INTERVAL '24 hours'
      `);
      return result.rowCount || 0;
    } catch (error) {
      console.error('Rate limit cleanup error:', error);
      return 0;
    }
  }

  // Get rate limit status for an identifier
  async getRateLimitStatus(
    identifierType: 'ip' | 'email' | 'user_id',
    identifierValue: string,
    endpoint: string
  ): Promise<{
    isLimited: boolean;
    attemptCount: number;
    windowStart: Date;
    blockedUntil?: Date;
  } | null> {
    try {
      const result = await this.db.query(`
        SELECT * FROM auth_rate_limits
        WHERE identifier_type = $1 AND identifier_value = $2 AND endpoint = $3
      `, [identifierType, identifierValue, endpoint]);

      if (result.rows.length === 0) {
        return null;
      }

      const entry = result.rows[0];
      const now = new Date();
      const isLimited = entry.blocked_until && new Date(entry.blocked_until) > now;

      return {
        isLimited: !!isLimited,
        attemptCount: entry.attempt_count,
        windowStart: new Date(entry.window_start),
        blockedUntil: entry.blocked_until ? new Date(entry.blocked_until) : undefined,
      };

    } catch (error) {
      console.error('Get rate limit status error:', error);
      return null;
    }
  }
}

// Express rate limit middleware factory
export function createRateLimit(config: RateLimitConfig) {
  return rateLimit({
    windowMs: config.windowMs,
    max: config.max,
    standardHeaders: true, // Return rate limit info in the `RateLimit-*` headers
    legacyHeaders: false, // Disable the `X-RateLimit-*` headers
    skipSuccessfulRequests: config.skipSuccessfulRequests,
    skipFailedRequests: config.skipFailedRequests,
    // Use default key generator to handle IPv6 properly
    keyGenerator: config.keyGenerator,
    handler: (req: Request, res: Response) => {
      res.status(429).json({
        success: false,
        error: {
          code: 'RATE_LIMIT_EXCEEDED',
          message: 'Too many requests, please try again later.',
          retryAfter: Math.round(config.windowMs / 1000),
        },
      });
    },
  });
}

// Specific rate limiters
export const loginRateLimit = createRateLimit(RATE_LIMITS.login);
export const registerRateLimit = createRateLimit(RATE_LIMITS.register);
export const passwordResetRateLimit = createRateLimit(RATE_LIMITS.passwordReset);
export const refreshRateLimit = createRateLimit(RATE_LIMITS.refresh);
export const apiRateLimit = createRateLimit(RATE_LIMITS.api);

// Email-based rate limiting (for registration/password reset)
export function createEmailRateLimit(maxAttempts: number, windowMs: number) {
  return async (req: Request, res: Response, next: NextFunction) => {
    const email = req.body.email;
    
    if (!email) {
      return next();
    }

    // This would need to be implemented with a proper store (Redis/Database)
    // For now, we'll use the IP-based rate limiting
    next();
  };
}

// Progressive delay for failed attempts
export function createProgressiveDelay() {
  const delays = new Map<string, { count: number; lastAttempt: number }>();

  return async (req: Request, res: Response, next: NextFunction) => {
    const key = req.ip || 'unknown';
    const now = Date.now();
    const entry = delays.get(key);

    if (entry) {
      // Reset if more than 1 hour has passed
      if (now - entry.lastAttempt > 60 * 60 * 1000) {
        delays.delete(key);
        return next();
      }

      // Calculate delay: 2^attempts seconds (max 30 seconds)
      const delaySeconds = Math.min(Math.pow(2, entry.count), 30);
      const delayMs = delaySeconds * 1000;

      if (now - entry.lastAttempt < delayMs) {
        return res.status(429).json({
          success: false,
          error: {
            code: 'RATE_LIMIT_DELAY',
            message: `Please wait ${delaySeconds} seconds before trying again.`,
            retryAfter: delaySeconds,
          },
        });
      }
    }

    next();
  };
}

// Account lockout middleware
export function createAccountLockout(db: Pool) {
  return async (req: Request, res: Response, next: NextFunction) => {
    const email = req.body.email;
    
    if (!email) {
      return next();
    }

    try {
      // Check if account is locked
      const userResult = await db.query(`
        SELECT is_locked, locked_until, failed_login_attempts
        FROM auth_users 
        WHERE email = $1 AND is_active = true
      `, [email.toLowerCase()]);

      if (userResult.rows.length === 0) {
        return next();
      }

      const user = userResult.rows[0];

      if (user.is_locked) {
        if (user.locked_until && new Date(user.locked_until) > new Date()) {
          const unlockTime = new Date(user.locked_until);
          const minutesRemaining = Math.ceil((unlockTime.getTime() - Date.now()) / (60 * 1000));
          
          return res.status(423).json({
            success: false,
            error: {
              code: 'ACCOUNT_LOCKED',
              message: `Account is locked due to too many failed login attempts. Try again in ${minutesRemaining} minutes.`,
              lockedUntil: unlockTime.toISOString(),
            },
          });
        } else {
          // Unlock expired account
          await db.query(`
            UPDATE auth_users 
            SET is_locked = false, locked_until = NULL, failed_login_attempts = 0
            WHERE email = $1
          `, [email.toLowerCase()]);
        }
      }

      next();

    } catch (error) {
      console.error('Account lockout check error:', error);
      next(); // Continue on error
    }
  };
}

// IP-based security middleware
export function createIPSecurityCheck() {
  return async (req: Request, res: Response, next: NextFunction) => {
    const ip = req.ip;
    const userAgent = req.headers['user-agent'];

    // Check for suspicious patterns
    if (!userAgent || userAgent.length < 10) {
      return res.status(400).json({
        success: false,
        error: {
          code: 'SUSPICIOUS_REQUEST',
          message: 'Request appears to be automated or suspicious.',
        },
      });
    }

    // Add IP to request for logging
    (req as any).clientIP = ip;
    
    next();
  };
}

// CSRF-like protection for state-changing operations
export function createCSRFProtection() {
  return async (req: Request, res: Response, next: NextFunction) => {
    const origin = req.headers.origin;
    const referer = req.headers.referer;
    const allowedOrigins = [
      process.env.FRONTEND_URL || 'http://localhost:3000',
      process.env.BASE_URL || 'http://localhost:3456',
    ];

    // For state-changing operations, check origin/referer
    if (['POST', 'PUT', 'DELETE', 'PATCH'].includes(req.method)) {
      if (!origin && !referer) {
        return res.status(403).json({
          success: false,
          error: {
            code: 'MISSING_ORIGIN',
            message: 'Origin or Referer header required for this operation.',
          },
        });
      }

      const sourceUrl = origin || referer;
      const isAllowed = allowedOrigins.some(allowed => 
        sourceUrl?.startsWith(allowed)
      );

      if (!isAllowed) {
        return res.status(403).json({
          success: false,
          error: {
            code: 'INVALID_ORIGIN',
            message: 'Request origin not allowed.',
          },
        });
      }
    }

    next();
  };
}