/**
 * Base Service Layer
 * 
 * Provides common functionality for all services including:
 * - Result pattern for consistent error handling
 * - Correlation ID tracking
 * - Audit logging
 * - Security utilities
 */

import { v4 as uuidv4 } from 'uuid';
import { createClient, RedisClientType } from 'redis';
import { getDatabase } from '../../db/connection.js';

// Result pattern for consistent error handling
export type Result<T, E = Error> = 
  | { success: true; data: T; correlationId: string }
  | { success: false; error: E; correlationId: string };

export function success<T>(data: T, correlationId?: string): Result<T> {
  return {
    success: true,
    data,
    correlationId: correlationId || uuidv4(),
  };
}

export function failure<T, E = Error>(error: E, correlationId?: string): Result<T, E> {
  return {
    success: false,
    error,
    correlationId: correlationId || uuidv4(),
  };
}

// Service error types
export class ServiceError extends Error {
  constructor(
    message: string,
    public code: string,
    public statusCode: number = 500,
    public details?: any,
    public correlationId?: string,
  ) {
    super(message);
    this.name = 'ServiceError';
  }
}

export class AuthorizationError extends ServiceError {
  constructor(message: string, correlationId?: string) {
    super(message, 'AUTHORIZATION_ERROR', 403, undefined, correlationId);
    this.name = 'AuthorizationError';
  }
}

export class ValidationError extends ServiceError {
  constructor(message: string, details?: any, correlationId?: string) {
    super(message, 'VALIDATION_ERROR', 400, details, correlationId);
    this.name = 'ValidationError';
  }
}

export class NotFoundError extends ServiceError {
  constructor(resource: string, correlationId?: string) {
    super(`${resource} not found`, 'NOT_FOUND', 404, undefined, correlationId);
    this.name = 'NotFoundError';
  }
}

// Service configuration interface
export interface ServiceConfig {
  redis?: {
    url?: string;
    enabled?: boolean;
    ttl?: number;
  };
  audit?: {
    enabled?: boolean;
    level?: 'info' | 'debug' | 'warn' | 'error';
  };
  security?: {
    encryptionKey?: string;
    auditSensitiveData?: boolean;
  };
}

// Cache interface for dependency injection
export interface CacheProvider {
  get<T>(key: string): Promise<T | null>;
  set<T>(key: string, value: T, ttlSeconds?: number): Promise<void>;
  del(key: string): Promise<void>;
  clear(): Promise<void>;
  isAvailable(): boolean;
}

// Redis cache implementation
export class RedisCache implements CacheProvider {
  private client: RedisClientType | null = null;
  private connected = false;

  constructor(private config: { url?: string; ttl?: number }) {
    this.initialize();
  }

  private async initialize(): Promise<void> {
    try {
      this.client = createClient({
        url: this.config.url || process.env.REDIS_URL || 'redis://localhost:6379',
      });

      this.client.on('error', (err) => {
        console.warn('Redis client error:', err);
        this.connected = false;
      });

      this.client.on('connect', () => {
        this.connected = true;
      });

      await this.client.connect();
    } catch (error) {
      console.warn('Failed to connect to Redis, falling back to memory cache:', error);
      this.connected = false;
    }
  }

  async get<T>(key: string): Promise<T | null> {
    if (!this.isAvailable()) return null;
    
    try {
      const value = await this.client!.get(key);
      return value ? JSON.parse(value) : null;
    } catch (error) {
      console.warn('Redis get error:', error);
      return null;
    }
  }

  async set<T>(key: string, value: T, ttlSeconds?: number): Promise<void> {
    if (!this.isAvailable()) return;
    
    try {
      const serialized = JSON.stringify(value);
      const ttl = ttlSeconds || this.config.ttl || 300; // 5 minutes default
      await this.client!.setEx(key, ttl, serialized);
    } catch (error) {
      console.warn('Redis set error:', error);
    }
  }

  async del(key: string): Promise<void> {
    if (!this.isAvailable()) return;
    
    try {
      await this.client!.del(key);
    } catch (error) {
      console.warn('Redis del error:', error);
    }
  }

  async clear(): Promise<void> {
    if (!this.isAvailable()) return;
    
    try {
      await this.client!.flushDb();
    } catch (error) {
      console.warn('Redis clear error:', error);
    }
  }

  isAvailable(): boolean {
    return this.connected && this.client !== null;
  }
}

// Memory cache fallback
export class MemoryCache implements CacheProvider {
  private cache = new Map<string, { value: any; expires: number }>();

  constructor(private defaultTtl: number = 300) {
    // Clean up expired entries every minute
    setInterval(() => this.cleanup(), 60000);
  }

  async get<T>(key: string): Promise<T | null> {
    const entry = this.cache.get(key);
    if (!entry) return null;
    
    if (Date.now() > entry.expires) {
      this.cache.delete(key);
      return null;
    }
    
    return entry.value;
  }

  async set<T>(key: string, value: T, ttlSeconds?: number): Promise<void> {
    const ttl = ttlSeconds || this.defaultTtl;
    const expires = Date.now() + (ttl * 1000);
    this.cache.set(key, { value, expires });
  }

  async del(key: string): Promise<void> {
    this.cache.delete(key);
  }

  async clear(): Promise<void> {
    this.cache.clear();
  }

  isAvailable(): boolean {
    return true;
  }

  private cleanup(): void {
    const now = Date.now();
    for (const [key, entry] of this.cache.entries()) {
      if (now > entry.expires) {
        this.cache.delete(key);
      }
    }
  }
}

// Audit logger interface
export interface AuditLogger {
  log(event: AuditEvent): Promise<void>;
}

export interface AuditEvent {
  correlationId: string;
  service: string;
  operation: string;
  userId?: string;
  resource?: string;
  action: string;
  success: boolean;
  error?: string;
  metadata?: Record<string, any>;
  timestamp: string;
  ipAddress?: string;
  userAgent?: string;
}

// Database audit logger implementation
export class DatabaseAuditLogger implements AuditLogger {
  async log(event: AuditEvent): Promise<void> {
    try {
      const db = getDatabase();
      await db.query(`
        INSERT INTO audit_log (
          correlation_id, service, operation, user_id, resource, action,
          success, error, metadata, timestamp, ip_address, user_agent
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
      `, [
        event.correlationId,
        event.service,
        event.operation,
        event.userId,
        event.resource,
        event.action,
        event.success,
        event.error,
        JSON.stringify(event.metadata || {}),
        event.timestamp,
        event.ipAddress,
        event.userAgent,
      ]);
    } catch (error) {
      console.error('Failed to log audit event:', error);
      // Don't throw - audit logging should not break operations
    }
  }
}

// Console audit logger fallback
export class ConsoleAuditLogger implements AuditLogger {
  async log(event: AuditEvent): Promise<void> {
    console.log('AUDIT:', JSON.stringify(event, null, 2));
  }
}

// Base service class
export abstract class BaseService {
  protected correlationId: string;
  protected cache: CacheProvider;
  protected auditLogger: AuditLogger;
  protected config: ServiceConfig;

  constructor(
    config: ServiceConfig = {},
    cache?: CacheProvider,
    auditLogger?: AuditLogger,
  ) {
    this.correlationId = uuidv4();
    this.config = config;
    
    // Initialize cache
    if (cache) {
      this.cache = cache;
    } else if (config.redis?.enabled !== false) {
      this.cache = new RedisCache(config.redis || {});
    } else {
      this.cache = new MemoryCache();
    }

    // Initialize audit logger
    this.auditLogger = auditLogger || new DatabaseAuditLogger();
  }

  protected setCorrelationId(correlationId: string): void {
    this.correlationId = correlationId;
  }

  protected async auditLog(
    operation: string,
    action: string,
    success: boolean,
    options: {
      userId?: string;
      resource?: string;
      error?: string;
      metadata?: Record<string, any>;
      ipAddress?: string;
      userAgent?: string;
    } = {},
  ): Promise<void> {
    await this.auditLogger.log({
      correlationId: this.correlationId,
      service: this.constructor.name,
      operation,
      action,
      success,
      timestamp: new Date().toISOString(),
      ...options,
    });
  }

  protected async withCache<T>(
    key: string,
    fetcher: () => Promise<T>,
    ttlSeconds?: number,
  ): Promise<T> {
    try {
      // Try to get from cache first
      const cached = await this.cache.get<T>(key);
      if (cached !== null) {
        return cached;
      }

      // Not in cache, fetch and store
      const result = await fetcher();
      await this.cache.set(key, result, ttlSeconds);
      return result;
    } catch (error) {
      // If cache fails, just fetch directly
      return await fetcher();
    }
  }

  protected async clearCachePattern(pattern: string): Promise<void> {
    // Simple implementation - in production you might want a more sophisticated pattern matching
    if (pattern.includes('*')) {
      await this.cache.clear();
    } else {
      await this.cache.del(pattern);
    }
  }

  // Authorization helper
  protected checkAuthorization(
    userId: string | undefined,
    requiredRole: string,
    resource?: string,
  ): void {
    if (!userId) {
      throw new AuthorizationError('Authentication required', this.correlationId);
    }

    // This would integrate with your actual authorization system
    // For now, we'll assume all authenticated users have access
    // In production, you'd check roles against the database
  }

  // Encryption helpers for sensitive data
  protected encrypt(data: string): string {
    // Simple encryption - in production use proper encryption libraries
    if (!this.config.security?.encryptionKey) {
      return data;
    }
    
    // This is a placeholder - implement proper encryption
    return Buffer.from(data).toString('base64');
  }

  protected decrypt(encryptedData: string): string {
    // Simple decryption - in production use proper encryption libraries
    if (!this.config.security?.encryptionKey) {
      return encryptedData;
    }
    
    // This is a placeholder - implement proper decryption
    return Buffer.from(encryptedData, 'base64').toString();
  }

  // Transaction support
  protected async withTransaction<T>(
    operation: (transaction: any) => Promise<T>,
  ): Promise<T> {
    const db = getDatabase();
    const client = await db.pool.connect();
    
    try {
      await client.query('BEGIN');
      const result = await operation(client);
      await client.query('COMMIT');
      return result;
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  // Health check method that all services should implement
  abstract healthCheck(): Promise<{ healthy: boolean; details?: any }>;
}