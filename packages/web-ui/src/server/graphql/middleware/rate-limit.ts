import type { ApolloServerPlugin } from '@apollo/server';

interface RateLimitOptions {
  windowMs: number;
  max: number;
  skipSuccessfulRequests?: boolean;
}

// Simple in-memory rate limiter (in production, use Redis)
const requestCounts = new Map<string, { count: number; resetTime: number }>();

export function createRateLimitPlugin(options: RateLimitOptions): ApolloServerPlugin {
  return {
    requestDidStart() {
      return {
        willSendResponse({ request, response, contextValue }) {
          // Extract client identifier (IP address or user ID)
          const clientId = contextValue.req?.ip || 
                          contextValue.req?.connection?.remoteAddress || 
                          'unknown';
          
          const now = Date.now();
          const windowStart = now - options.windowMs;
          
          // Clean up old entries
          for (const [key, data] of requestCounts.entries()) {
            if (data.resetTime < windowStart) {
              requestCounts.delete(key);
            }
          }
          
          // Get or create entry for this client
          let entry = requestCounts.get(clientId);
          if (!entry || entry.resetTime < windowStart) {
            entry = { count: 0, resetTime: now + options.windowMs };
            requestCounts.set(clientId, entry);
          }
          
          // Increment count
          entry.count++;
          
          // Check if limit exceeded
          if (entry.count > options.max) {
            throw new Error(`Rate limit exceeded. Try again in ${Math.ceil((entry.resetTime - now) / 1000)} seconds.`);
          }
          
          // Add rate limit headers
          if (response.http) {
            response.http.headers.set('X-RateLimit-Limit', options.max.toString());
            response.http.headers.set('X-RateLimit-Remaining', (options.max - entry.count).toString());
            response.http.headers.set('X-RateLimit-Reset', Math.ceil(entry.resetTime / 1000).toString());
          }
        },
      };
    },
  };
}