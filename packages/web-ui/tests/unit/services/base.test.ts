/**
 * Base Service Tests
 * 
 * Unit tests for the base service class demonstrating mocked dependencies.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  BaseService,
  ServiceConfig,
  CacheProvider,
  AuditLogger,
  MemoryCache,
  success,
  failure,
  ServiceError,
  AuthorizationError,
  ValidationError,
  NotFoundError,
} from '../../../src/server/services/base.js';

// Mock implementations for testing
class MockCacheProvider implements CacheProvider {
  private storage = new Map<string, any>();

  async get<T>(key: string): Promise<T | null> {
    return this.storage.get(key) || null;
  }

  async set<T>(key: string, value: T, ttlSeconds?: number): Promise<void> {
    this.storage.set(key, value);
  }

  async del(key: string): Promise<void> {
    this.storage.delete(key);
  }

  async clear(): Promise<void> {
    this.storage.clear();
  }

  isAvailable(): boolean {
    return true;
  }
}

class MockAuditLogger implements AuditLogger {
  public logs: any[] = [];

  async log(event: any): Promise<void> {
    this.logs.push(event);
  }
}

class TestService extends BaseService {
  constructor(
    config: ServiceConfig = {},
    cache?: CacheProvider,
    auditLogger?: AuditLogger,
  ) {
    super(config, cache, auditLogger);
  }

  async testCacheOperation() {
    return this.withCache('test-key', async () => {
      return 'cached-value';
    });
  }

  async testAuditLog() {
    await this.auditLog('test', 'test_operation', true, {
      userId: 'test-user',
      metadata: { test: true },
    });
  }

  async testAuthorization(userId?: string) {
    this.checkAuthorization(userId, 'read');
  }

  async testTransaction() {
    return this.withTransaction(async (tx) => {
      return 'transaction-result';
    });
  }

  async healthCheck() {
    return { healthy: true };
  }
}

// Mock the database connection
vi.mock('../../../src/db/connection.js', () => ({
  getDatabase: () => ({
    pool: {
      connect: () => ({
        query: vi.fn().mockResolvedValue({ rows: [] }),
        release: vi.fn(),
      }),
    },
  }),
}));

describe('BaseService', () => {
  let mockCache: MockCacheProvider;
  let mockAuditLogger: MockAuditLogger;
  let service: TestService;

  beforeEach(() => {
    mockCache = new MockCacheProvider();
    mockAuditLogger = new MockAuditLogger();
    service = new TestService({}, mockCache, mockAuditLogger);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Result Pattern', () => {
    it('should create success result', () => {
      const result = success('test-data', 'correlation-123');
      
      expect(result.success).toBe(true);
      expect(result.data).toBe('test-data');
      expect(result.correlationId).toBe('correlation-123');
    });

    it('should create failure result', () => {
      const error = new Error('test error');
      const result = failure(error, 'correlation-123');
      
      expect(result.success).toBe(false);
      expect(result.error).toBe(error);
      expect(result.correlationId).toBe('correlation-123');
    });

    it('should create service error with details', () => {
      const error = new ServiceError('test message', 'TEST_CODE', 400, { detail: 'test' });
      
      expect(error.message).toBe('test message');
      expect(error.code).toBe('TEST_CODE');
      expect(error.statusCode).toBe(400);
      expect(error.details).toEqual({ detail: 'test' });
    });

    it('should create authorization error', () => {
      const error = new AuthorizationError('Unauthorized', 'correlation-123');
      
      expect(error.message).toBe('Unauthorized');
      expect(error.code).toBe('AUTHORIZATION_ERROR');
      expect(error.statusCode).toBe(403);
      expect(error.correlationId).toBe('correlation-123');
    });

    it('should create validation error', () => {
      const error = new ValidationError('Invalid input', { field: 'name' }, 'correlation-123');
      
      expect(error.message).toBe('Invalid input');
      expect(error.code).toBe('VALIDATION_ERROR');
      expect(error.statusCode).toBe(400);
      expect(error.details).toEqual({ field: 'name' });
    });

    it('should create not found error', () => {
      const error = new NotFoundError('User', 'correlation-123');
      
      expect(error.message).toBe('User not found');
      expect(error.code).toBe('NOT_FOUND');
      expect(error.statusCode).toBe(404);
    });
  });

  describe('Cache Operations', () => {
    it('should cache and retrieve values', async () => {
      const result = await service.testCacheOperation();
      expect(result).toBe('cached-value');

      // Verify it was cached
      const cachedValue = await mockCache.get('test-key');
      expect(cachedValue).toBe('cached-value');
    });

    it('should use cached value on subsequent calls', async () => {
      // Pre-populate cache
      await mockCache.set('test-key', 'pre-cached-value');

      const result = await service.testCacheOperation();
      expect(result).toBe('pre-cached-value');
    });

    it('should handle cache errors gracefully', async () => {
      // Mock cache to throw error
      const errorCache = {
        get: vi.fn().mockRejectedValue(new Error('Cache error')),
        set: vi.fn().mockRejectedValue(new Error('Cache error')),
        del: vi.fn(),
        clear: vi.fn(),
        isAvailable: () => false,
      };

      const serviceWithErrorCache = new TestService({}, errorCache, mockAuditLogger);
      
      // Should still work despite cache errors
      const result = await serviceWithErrorCache.testCacheOperation();
      expect(result).toBe('cached-value');
    });
  });

  describe('Audit Logging', () => {
    it('should log audit events', async () => {
      await service.testAuditLog();

      expect(mockAuditLogger.logs).toHaveLength(1);
      expect(mockAuditLogger.logs[0]).toMatchObject({
        service: 'TestService',
        operation: 'test',
        action: 'test_operation',
        success: true,
        userId: 'test-user',
        metadata: { test: true },
      });
    });

    it('should include correlation ID in audit logs', async () => {
      await service.testAuditLog();

      expect(mockAuditLogger.logs[0].correlationId).toBeDefined();
      expect(typeof mockAuditLogger.logs[0].correlationId).toBe('string');
    });

    it('should include timestamp in audit logs', async () => {
      await service.testAuditLog();

      expect(mockAuditLogger.logs[0].timestamp).toBeDefined();
      expect(new Date(mockAuditLogger.logs[0].timestamp)).toBeInstanceOf(Date);
    });
  });

  describe('Authorization', () => {
    it('should allow access with valid user', () => {
      expect(() => service.testAuthorization('valid-user')).not.toThrow();
    });

    it('should throw authorization error without user', () => {
      expect(() => service.testAuthorization()).toThrow(AuthorizationError);
    });

    it('should throw authorization error with undefined user', () => {
      expect(() => service.testAuthorization(undefined)).toThrow(AuthorizationError);
    });

    it('should include correlation ID in authorization error', () => {
      try {
        service.testAuthorization();
      } catch (error) {
        expect(error).toBeInstanceOf(AuthorizationError);
        expect(error.correlationId).toBeDefined();
      }
    });
  });

  describe('Memory Cache', () => {
    let memoryCache: MemoryCache;

    beforeEach(() => {
      memoryCache = new MemoryCache(1); // 1 second TTL for testing
    });

    it('should store and retrieve values', async () => {
      await memoryCache.set('key1', 'value1');
      const result = await memoryCache.get('key1');
      expect(result).toBe('value1');
    });

    it('should expire values after TTL', async () => {
      await memoryCache.set('key1', 'value1', 0.1); // 0.1 second TTL
      
      // Should be available immediately
      let result = await memoryCache.get('key1');
      expect(result).toBe('value1');

      // Wait for expiration
      await new Promise(resolve => setTimeout(resolve, 150));
      
      result = await memoryCache.get('key1');
      expect(result).toBeNull();
    });

    it('should delete values', async () => {
      await memoryCache.set('key1', 'value1');
      await memoryCache.del('key1');
      const result = await memoryCache.get('key1');
      expect(result).toBeNull();
    });

    it('should clear all values', async () => {
      await memoryCache.set('key1', 'value1');
      await memoryCache.set('key2', 'value2');
      await memoryCache.clear();
      
      const result1 = await memoryCache.get('key1');
      const result2 = await memoryCache.get('key2');
      expect(result1).toBeNull();
      expect(result2).toBeNull();
    });

    it('should always be available', () => {
      expect(memoryCache.isAvailable()).toBe(true);
    });
  });

  describe('Encryption/Decryption', () => {
    it('should encrypt and decrypt data when key is provided', () => {
      const serviceWithEncryption = new TestService({
        security: { encryptionKey: 'test-key' },
      });

      const originalData = 'sensitive-data';
      const encrypted = (serviceWithEncryption as any).encrypt(originalData);
      const decrypted = (serviceWithEncryption as any).decrypt(encrypted);

      expect(encrypted).not.toBe(originalData);
      expect(decrypted).toBe(originalData);
    });

    it('should return original data when no encryption key', () => {
      const originalData = 'sensitive-data';
      const encrypted = (service as any).encrypt(originalData);
      const decrypted = (service as any).decrypt(originalData);

      expect(encrypted).toBe(originalData);
      expect(decrypted).toBe(originalData);
    });
  });

  describe('Health Check', () => {
    it('should return health status', async () => {
      const health = await service.healthCheck();
      expect(health.healthy).toBe(true);
    });
  });

  describe('Correlation ID', () => {
    it('should generate unique correlation IDs', () => {
      const service1 = new TestService();
      const service2 = new TestService();

      expect((service1 as any).correlationId).toBeDefined();
      expect((service2 as any).correlationId).toBeDefined();
      expect((service1 as any).correlationId).not.toBe((service2 as any).correlationId);
    });

    it('should allow setting correlation ID', () => {
      const customId = 'custom-correlation-id';
      (service as any).setCorrelationId(customId);
      expect((service as any).correlationId).toBe(customId);
    });
  });

  describe('Error Handling', () => {
    it('should handle service errors properly', () => {
      const error = new ServiceError('Test error', 'TEST_ERROR', 500);
      expect(error.name).toBe('ServiceError');
      expect(error.message).toBe('Test error');
      expect(error.code).toBe('TEST_ERROR');
      expect(error.statusCode).toBe(500);
    });

    it('should handle nested error scenarios', () => {
      const originalError = new Error('Original error');
      const serviceError = new ServiceError(
        'Wrapped error',
        'WRAPPED_ERROR',
        500,
        { originalError: originalError.message },
      );

      expect(serviceError.details.originalError).toBe('Original error');
    });
  });
});

describe('Database Audit Logger', () => {
  let mockDatabase: any;
  let auditLogger: any;

  beforeEach(async () => {
    mockDatabase = {
      query: vi.fn().mockResolvedValue({ rows: [] }),
    };

    // Import dynamically to avoid module caching issues
    const { DatabaseAuditLogger } = await import('../../../src/server/services/base.js');
    auditLogger = new DatabaseAuditLogger();
    
    // Mock the database
    vi.doMock('../../../src/db/connection.js', () => ({
      getDatabase: () => mockDatabase,
    }));
  });

  it('should store audit events in database', async () => {
    const event = {
      correlationId: 'test-correlation',
      service: 'TestService',
      operation: 'test',
      action: 'test_action',
      success: true,
      timestamp: new Date().toISOString(),
    };

    await auditLogger.log(event);

    expect(mockDatabase.query).toHaveBeenCalledWith(
      expect.stringContaining('INSERT INTO audit_log'),
      expect.arrayContaining([
        'test-correlation',
        'TestService',
        'test',
        undefined, // user_id
        undefined, // resource
        'test_action',
        true,
        undefined, // error
        '{}', // metadata
        event.timestamp,
        undefined, // ip_address
        undefined, // user_agent
      ]),
    );
  });

  it('should handle database errors gracefully', async () => {
    mockDatabase.query.mockRejectedValue(new Error('Database error'));

    const event = {
      correlationId: 'test-correlation',
      service: 'TestService',
      operation: 'test',
      action: 'test_action',
      success: true,
      timestamp: new Date().toISOString(),
    };

    // Should not throw error
    await expect(auditLogger.log(event)).resolves.toBeUndefined();
  });
});