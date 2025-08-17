/**
 * Maproom Service and API Integration Tests
 * 
 * Tests for the MaproomService class and Maproom API endpoints.
 * Includes mocking for the Maproom binary to avoid external dependencies in tests.
 */

import { describe, it, expect, beforeEach, afterEach, vi, MockedFunction } from 'vitest';
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import request from 'supertest';
import app from '../src/server.js';
import { MaproomService, getMaproomService } from '../src/services/maproom.js';
import { EventEmitter } from 'events';

// Mock child_process
vi.mock('node:child_process');
const mockSpawn = spawn as MockedFunction<typeof spawn>;
const mockSpawnSync = spawnSync as MockedFunction<typeof spawnSync>;

// Mock fs
vi.mock('node:fs');
const mockFs = vi.mocked(fs);

// Mock database
vi.mock('../src/db/connection.js', () => ({
  getDatabase: vi.fn(() => ({
    query: vi.fn(),
  })),
  initializeDatabase: vi.fn(),
}));

describe('MaproomService', () => {
  let service: MaproomService;

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Mock successful binary resolution
    mockSpawnSync.mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 12345,
      output: [null, Buffer.from('/usr/local/bin/crewchief-maproom'), Buffer.from('')],
      signal: null,
    });

    service = new MaproomService({
      binaryPath: '/mock/crewchief-maproom',
      timeout: 5000,
      retries: 1,
      cacheEnabled: true,
      cacheTtl: 300000,
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  describe('Binary Resolution', () => {
    it('should resolve binary from environment variable', () => {
      process.env.CREWCHIEF_MAPROOM_BIN = '/custom/path/maproom';
      
      // Mock fs.existsSync to return true for the custom path
      mockFs.existsSync.mockImplementation((path: string) => path === '/custom/path/maproom');

      const customService = new MaproomService();
      expect(customService).toBeDefined();
      
      delete process.env.CREWCHIEF_MAPROOM_BIN;
    });

    it('should throw error when binary is not found', () => {
      mockFs.existsSync.mockReturnValue(false);
      mockSpawnSync.mockReturnValue({ status: 1, stdout: Buffer.from(''), stderr: Buffer.from(''), pid: 0, output: [], signal: null });

      expect(() => new MaproomService({ binaryPath: undefined })).toThrow('Maproom binary not found');
    });
  });

  describe('Search Operations', () => {
    it('should perform a successful search', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();
      mockProcess.kill = vi.fn();

      mockSpawn.mockReturnValue(mockProcess);

      const searchPromise = service.search('test query', { maxResults: 10 });

      // Simulate successful command output
      setTimeout(() => {
        const mockResults = {
          hits: [
            {
              chunk_id: 'test-1',
              file_path: '/test/file.ts',
              line_start: 10,
              line_end: 15,
              content: 'test content',
              score: 0.95,
              language: 'typescript',
            },
          ],
        };
        mockProcess.stdout.emit('data', JSON.stringify(mockResults));
        mockProcess.emit('close', 0);
      }, 10);

      const result = await searchPromise;

      expect(result.query).toBe('test query');
      expect(result.results).toHaveLength(1);
      expect(result.results[0].file_path).toBe('/test/file.ts');
      expect(result.cached).toBe(false);
      expect(mockSpawn).toHaveBeenCalledWith(
        '/mock/crewchief-maproom',
        ['search', 'test query', '--limit', '10', '--format', 'json'],
        { stdio: ['pipe', 'pipe', 'pipe'] }
      );
    });

    it('should return cached results for repeated searches', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();

      mockSpawn.mockReturnValue(mockProcess);

      // First search
      const firstSearchPromise = service.search('cached query');
      setTimeout(() => {
        mockProcess.stdout.emit('data', JSON.stringify({ hits: [] }));
        mockProcess.emit('close', 0);
      }, 10);
      
      const firstResult = await firstSearchPromise;
      expect(firstResult.cached).toBe(false);

      // Second search should be cached
      const secondResult = await service.search('cached query');
      expect(secondResult.cached).toBe(true);
      expect(mockSpawn).toHaveBeenCalledTimes(1); // Only called once
    });

    it('should handle search errors gracefully', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();
      mockProcess.kill = vi.fn();

      mockSpawn.mockReturnValue(mockProcess);

      const searchPromise = service.search('error query');

      // Simulate command failure immediately
      setTimeout(() => {
        mockProcess.stderr.emit('data', 'Command failed');
        mockProcess.emit('close', 1);
      }, 1);

      await expect(searchPromise).rejects.toThrow();
    }, 10000);

    it('should handle timeout', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();
      mockProcess.kill = vi.fn();

      mockSpawn.mockReturnValue(mockProcess);

      const timeoutService = new MaproomService({
        binaryPath: '/mock/crewchief-maproom',
        timeout: 100, // Very short timeout
        retries: 0, // No retries for faster test
      });

      const searchPromise = timeoutService.search('timeout query');

      // Don't emit close event to simulate hanging process
      
      await expect(searchPromise).rejects.toThrow();
      expect(mockProcess.kill).toHaveBeenCalledWith('SIGTERM');
    }, 10000);
  });

  describe('Index Operations', () => {
    it('should start indexing operation', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();

      mockSpawn.mockReturnValue(mockProcess);

      const indexPromise = service.index(['/test/path'], { repo: 'test-repo' });

      // Simulate successful indexing
      setTimeout(() => {
        const mockResult = {
          files_processed: 10,
          total_files: 10,
        };
        mockProcess.stdout.emit('data', JSON.stringify(mockResult));
        mockProcess.emit('close', 0);
      }, 10);

      const result = await indexPromise;

      expect(result.status).toBe('completed');
      expect(result.filesProcessed).toBe(10);
      expect(mockSpawn).toHaveBeenCalledWith(
        '/mock/crewchief-maproom',
        ['scan', '--repo', 'test-repo', '/test/path', '--format', 'json'],
        { stdio: ['pipe', 'pipe', 'pipe'] }
      );
    });
  });

  describe('Status Operations', () => {
    it('should get index status', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();

      mockSpawn.mockReturnValue(mockProcess);

      const statusPromise = service.getStatus();

      // Simulate successful status response
      setTimeout(() => {
        const mockStatus = {
          repos: [
            {
              id: 1,
              name: 'test-repo',
              path: '/test/repo',
              worktrees: [],
            },
          ],
          total_files: 100,
          total_chunks: 500,
          last_updated: '2024-01-01T00:00:00Z',
        };
        mockProcess.stdout.emit('data', JSON.stringify(mockStatus));
        mockProcess.emit('close', 0);
      }, 10);

      const result = await statusPromise;

      expect(result.repos).toHaveLength(1);
      expect(result.totalFiles).toBe(100);
      expect(result.totalChunks).toBe(500);
    });
  });

  describe('Health Check', () => {
    it('should return healthy status with version', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();

      mockSpawn.mockReturnValue(mockProcess);

      const healthPromise = service.healthCheck();

      // Simulate version output
      setTimeout(() => {
        mockProcess.stdout.emit('data', 'crewchief-maproom 0.1.0');
        mockProcess.emit('close', 0);
      }, 10);

      const result = await healthPromise;

      expect(result.healthy).toBe(true);
      expect(result.version).toBe('crewchief-maproom 0.1.0');
    });

    it('should return unhealthy status on error', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();
      mockProcess.kill = vi.fn();

      mockSpawn.mockReturnValue(mockProcess);

      const healthPromise = service.healthCheck();

      // Simulate command failure immediately
      process.nextTick(() => {
        mockProcess.emit('close', 1);
      });

      const result = await healthPromise;

      expect(result.healthy).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('Cache Management', () => {
    it('should clear cache', () => {
      expect(() => service.clearCache()).not.toThrow();
    });

    it('should return cache stats', () => {
      const stats = service.getCacheStats();
      expect(stats).toHaveProperty('size');
      expect(typeof stats.size).toBe('number');
    });
  });
});

describe('Maproom API Endpoints', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    // Mock successful binary resolution
    mockSpawnSync.mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 12345,
      output: [null, Buffer.from('/usr/local/bin/crewchief-maproom'), Buffer.from('')],
      signal: null,
    });

    // Mock fs.existsSync for binary resolution
    mockFs.existsSync.mockReturnValue(true);
  });

  describe('POST /api/maproom/search', () => {
    it('should perform search and return results', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();
      mockProcess.kill = vi.fn();

      mockSpawn.mockReturnValue(mockProcess);

      const searchPromise = request(app)
        .post('/api/maproom/search')
        .send({
          query: 'test search',
          filters: { maxResults: 5 },
        })
        .set('x-session-id', 'test-session');

      // Simulate successful response
      setTimeout(() => {
        const mockResults = {
          hits: [
            {
              chunk_id: 'test-1',
              file_path: '/test/file.ts',
              line_start: 1,
              line_end: 5,
              content: 'test content',
              score: 0.9,
            },
          ],
        };
        mockProcess.stdout.emit('data', JSON.stringify(mockResults));
        mockProcess.emit('close', 0);
      }, 10);

      const response = await searchPromise;

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.query).toBe('test search');
      expect(response.body.data.results).toHaveLength(1);
    });

    it('should validate search request', async () => {
      const response = await request(app)
        .post('/api/maproom/search')
        .send({
          query: '', // Empty query should fail validation
        });

      expect(response.status).toBe(400);
      expect(response.body.error).toBe('Invalid request');
    });
  });

  describe('GET /api/maproom/status', () => {
    it('should return index status', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();

      mockSpawn.mockReturnValue(mockProcess);

      const statusPromise = request(app).get('/api/maproom/status');

      // Simulate successful status response
      setTimeout(() => {
        const mockStatus = {
          repos: [],
          total_files: 50,
          total_chunks: 200,
          last_updated: '2024-01-01T00:00:00Z',
        };
        mockProcess.stdout.emit('data', JSON.stringify(mockStatus));
        mockProcess.emit('close', 0);
      }, 10);

      const response = await statusPromise;

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.totalFiles).toBe(50);
    });
  });

  describe('POST /api/maproom/index', () => {
    it('should start indexing operation', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();

      mockSpawn.mockReturnValue(mockProcess);

      const indexPromise = request(app)
        .post('/api/maproom/index')
        .send({
          paths: ['/test/path'],
          options: { repo: 'test-repo' },
        });

      // Simulate indexing response
      setTimeout(() => {
        const mockResult = {
          files_processed: 5,
          total_files: 5,
        };
        mockProcess.stdout.emit('data', JSON.stringify(mockResult));
        mockProcess.emit('close', 0);
      }, 10);

      const response = await indexPromise;

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.status).toBe('completed');
    });

    it('should validate index request', async () => {
      const response = await request(app)
        .post('/api/maproom/index')
        .send({
          paths: [], // Empty paths should fail validation
        });

      expect(response.status).toBe(400);
      expect(response.body.error).toBe('Invalid request');
    });
  });

  describe('GET /api/maproom/health', () => {
    it('should return health status', async () => {
      const mockProcess = new EventEmitter() as any;
      mockProcess.stdout = new EventEmitter();
      mockProcess.stderr = new EventEmitter();

      mockSpawn.mockReturnValue(mockProcess);

      const healthPromise = request(app).get('/api/maproom/health');

      // Simulate healthy response
      setTimeout(() => {
        mockProcess.stdout.emit('data', 'crewchief-maproom 0.1.0');
        mockProcess.emit('close', 0);
      }, 10);

      const response = await healthPromise;

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.status).toBe('healthy');
    });
  });

  describe('Cache Management Endpoints', () => {
    it('should return cache stats', async () => {
      const response = await request(app).get('/api/maproom/cache/stats');

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data).toHaveProperty('size');
    });

    it('should clear cache', async () => {
      const response = await request(app).delete('/api/maproom/cache');

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.message).toBe('Cache cleared successfully');
    });
  });
});

describe('Error Handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    // Mock fs.existsSync to return false, simulating binary not found
    mockFs.existsSync.mockReturnValue(false);
    mockSpawnSync.mockReturnValue({ status: 1, stdout: Buffer.from(''), stderr: Buffer.from(''), pid: 0, output: [], signal: null });
  });

  it('should handle service initialization errors', async () => {
    const response = await request(app)
      .post('/api/maproom/search')
      .send({ query: 'test' })
      .timeout(1000);

    expect(response.status).toBe(503);
    expect(response.body.error).toBe('Service unavailable');
  });
});