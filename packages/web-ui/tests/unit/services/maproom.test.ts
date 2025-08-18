/**
 * Maproom Service Tests
 * 
 * Unit tests for the MaproomService demonstrating mocked dependencies.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MaproomService } from '../../../src/server/services/maproom.js';
import type { CacheProvider, AuditLogger } from '../../../src/server/services/base.js';

// Mock implementations
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

// Mock child_process
const mockSpawn = vi.fn();
const mockSpawnSync = vi.fn();

vi.mock('node:child_process', () => ({
  spawn: mockSpawn,
  spawnSync: mockSpawnSync,
}));

// Mock fs
const mockExistsSync = vi.fn();
vi.mock('node:fs', () => ({
  existsSync: mockExistsSync,
}));

// Mock database
vi.mock('../../../src/db/connection.js', () => ({
  getDatabase: () => ({
    query: vi.fn().mockResolvedValue({ rows: [] }),
  }),
}));

describe('MaproomService', () => {
  let mockCache: MockCacheProvider;
  let mockAuditLogger: MockAuditLogger;
  let service: MaproomService;
  let mockProcess: any;

  beforeEach(() => {
    mockCache = new MockCacheProvider();
    mockAuditLogger = new MockAuditLogger();
    
    // Mock process object
    mockProcess = {
      stdout: { on: vi.fn() },
      stderr: { on: vi.fn() },
      on: vi.fn(),
      kill: vi.fn(),
    };

    // Setup default mocks
    mockExistsSync.mockReturnValue(true);
    mockSpawnSync.mockReturnValue({ status: 0 });
    mockSpawn.mockReturnValue(mockProcess);

    // Create service with mocked binary path
    service = new MaproomService(
      { binaryPath: '/mock/path/to/maproom' },
      mockCache,
      mockAuditLogger,
    );
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Construction', () => {
    it('should initialize with provided config', () => {
      const config = {
        binaryPath: '/custom/path',
        timeout: 5000,
        retries: 3,
      };

      const customService = new MaproomService(config, mockCache, mockAuditLogger);
      expect(customService).toBeInstanceOf(MaproomService);
    });

    it('should throw error if binary not found', () => {
      mockExistsSync.mockReturnValue(false);
      mockSpawnSync.mockReturnValue({ status: 1 });

      expect(() => {
        new MaproomService({}, mockCache, mockAuditLogger);
      }).toThrow('Maproom binary not found');
    });

    it('should resolve binary from environment variable', () => {
      process.env.CREWCHIEF_MAPROOM_BIN = '/env/path/to/maproom';
      mockExistsSync.mockImplementation((path) => path === '/env/path/to/maproom');

      const service = new MaproomService({}, mockCache, mockAuditLogger);
      expect(service).toBeInstanceOf(MaproomService);

      delete process.env.CREWCHIEF_MAPROOM_BIN;
    });
  });

  describe('Search Operations', () => {
    it('should perform search with valid query', async () => {
      // Mock successful command execution
      const mockResults = {
        hits: [
          {
            chunk_id: 'test-chunk-1',
            file_path: '/test/file.ts',
            line_start: 1,
            line_end: 5,
            content: 'test content',
            score: 0.8,
            language: 'typescript',
          },
        ],
      };

      mockProcess.stdout.on.mockImplementation((event, callback) => {
        if (event === 'data') {
          setTimeout(() => callback(JSON.stringify(mockResults)), 0);
        }
      });
      mockProcess.on.mockImplementation((event, callback) => {
        if (event === 'close') {
          setTimeout(() => callback(0), 0);
        }
      });

      const result = await service.search('test query', {}, 'test-user');

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.query).toBe('test query');
        expect(result.data.results).toHaveLength(1);
        expect(result.data.results[0].id).toBe('test-chunk-1');
        expect(result.data.cached).toBe(false);
      }
    });

    it('should use cached results when available', async () => {
      const cachedResponse = {
        query: 'cached query',
        results: [],
        totalCount: 0,
        executionTimeMs: 100,
        filters: {},
        cached: false,
        correlationId: 'test-correlation',
      };

      await mockCache.set('maproom:search:{"query":"cached query","filters":{}}', cachedResponse);

      const result = await service.search('cached query', {}, 'test-user');

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.cached).toBe(true);
        expect(result.data.query).toBe('cached query');
      }

      // Should not have spawned a process
      expect(mockSpawn).not.toHaveBeenCalled();
    });

    it('should handle authorization errors', async () => {
      const result = await service.search('test query', {}, undefined);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.code).toBe('AUTHORIZATION_ERROR');
      }
    });

    it('should handle command timeout', async () => {
      mockProcess.on.mockImplementation((event, callback) => {
        // Don't call the callback to simulate timeout
      });

      const timeoutService = new MaproomService(
        { binaryPath: '/mock/path/to/maproom', timeout: 100 },
        mockCache,
        mockAuditLogger,
      );

      const result = await timeoutService.search('test query', {}, 'test-user');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.code).toBe('MAPROOM_TIMEOUT');
      }
    });
  });

  describe('Health Check', () => {
    it('should return healthy status', async () => {
      mockProcess.stdout.on.mockImplementation((event, callback) => {
        if (event === 'data') {
          setTimeout(() => callback('maproom v1.0.0'), 0);
        }
      });
      mockProcess.on.mockImplementation((event, callback) => {
        if (event === 'close') {
          setTimeout(() => callback(0), 0);
        }
      });

      const health = await service.healthCheck();

      expect(health.healthy).toBe(true);
      expect(health.details.version).toBe('maproom v1.0.0');
      expect(health.details.cacheAvailable).toBe(true);
    });

    it('should return unhealthy status on error', async () => {
      mockProcess.on.mockImplementation((event, callback) => {
        if (event === 'error') {
          setTimeout(() => callback(new Error('Binary not found')), 0);
        }
      });

      const health = await service.healthCheck();

      expect(health.healthy).toBe(false);
      expect(health.details.error).toContain('Binary not found');
    });
  });

  describe('Audit Logging', () => {
    it('should log search operations', async () => {
      mockProcess.stdout.on.mockImplementation((event, callback) => {
        if (event === 'data') {
          setTimeout(() => callback('{"hits":[]}'), 0);
        }
      });
      mockProcess.on.mockImplementation((event, callback) => {
        if (event === 'close') {
          setTimeout(() => callback(0), 0);
        }
      });

      await service.search('test query', {}, 'test-user');

      expect(mockAuditLogger.logs).toContainEqual(
        expect.objectContaining({
          service: 'MaproomService',
          operation: 'search',
          action: 'search_code',
          success: true,
          userId: 'test-user',
        }),
      );
    });

    it('should log failed operations', async () => {
      mockProcess.on.mockImplementation((event, callback) => {
        if (event === 'close') {
          setTimeout(() => callback(1), 0); // Exit code 1
        }
      });

      await service.search('test query', {}, 'test-user');

      expect(mockAuditLogger.logs).toContainEqual(
        expect.objectContaining({
          service: 'MaproomService',
          operation: 'search',
          action: 'search_code',
          success: false,
          userId: 'test-user',
        }),
      );
    });
  });
});