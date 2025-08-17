import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MaproomService, type MaproomConfig, type SearchFilters } from '../../../src/services/maproom.js';
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';

// Mock dependencies
vi.mock('node:child_process');
vi.mock('node:fs');
vi.mock('../../../src/db/connection.js', () => ({
  getDatabase: vi.fn(() => ({
    query: vi.fn(),
  })),
}));

const mockSpawn = vi.mocked(spawn);
const mockSpawnSync = vi.mocked(spawnSync);
const mockFs = vi.mocked(fs);

describe('MaproomService', () => {
  let service: MaproomService;
  let mockChildProcess: any;

  beforeEach(() => {
    // Reset all mocks
    vi.clearAllMocks();
    
    // Mock fs.existsSync to return true for binary path
    mockFs.existsSync = vi.fn().mockReturnValue(true);
    
    // Mock spawnSync for binary resolution
    mockSpawnSync.mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
    } as any);

    // Create mock child process
    mockChildProcess = {
      stdout: {
        on: vi.fn((event, callback) => {
          if (event === 'data') {
            // Store callback for later use
            mockChildProcess.stdoutCallback = callback;
          }
        }),
      },
      stderr: {
        on: vi.fn((event, callback) => {
          if (event === 'data') {
            mockChildProcess.stderrCallback = callback;
          }
        }),
      },
      on: vi.fn((event, callback) => {
        if (event === 'close') {
          mockChildProcess.closeCallback = callback;
        } else if (event === 'error') {
          mockChildProcess.errorCallback = callback;
        }
      }),
      kill: vi.fn(),
    };

    mockSpawn.mockReturnValue(mockChildProcess);

    // Create service instance
    const config: Partial<MaproomConfig> = {
      binaryPath: '/usr/local/bin/crewchief-maproom',
      timeout: 5000,
      retries: 1,
      retryDelay: 100,
      cacheEnabled: true,
      cacheTtl: 10000,
    };
    
    service = new MaproomService(config);
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  describe('constructor', () => {
    it('should initialize with default config', () => {
      expect(service).toBeInstanceOf(MaproomService);
    });

    it('should throw error if binary not found', () => {
      mockFs.existsSync = vi.fn().mockReturnValue(false);
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('command not found'),
      } as any);

      expect(() => new MaproomService()).toThrow('Maproom binary not found');
    });
  });

  describe('search', () => {
    it('should perform a successful search', async () => {
      const mockSearchResult = {
        hits: [
          {
            chunk_id: 'test-chunk-1',
            file_path: '/path/to/file.ts',
            line_start: 1,
            line_end: 10,
            content: 'function test() { return true; }',
            score: 0.95,
            language: 'typescript',
            chunk_type: 'function',
            context: {
              before: 'import { test } from "test";',
              after: 'export default test;',
            },
          },
        ],
      };

      // Set up the spawn mock to simulate successful command execution
      const searchPromise = service.search('test query');
      
      // Simulate stdout data
      mockChildProcess.stdoutCallback(JSON.stringify(mockSearchResult));
      
      // Simulate process close with success
      setTimeout(() => {
        mockChildProcess.closeCallback(0);
      }, 10);

      const result = await searchPromise;

      expect(result).toEqual({
        query: 'test query',
        results: [
          {
            id: 'test-chunk-1',
            file_path: '/path/to/file.ts',
            line_start: 1,
            line_end: 10,
            content: 'function test() { return true; }',
            relevance_score: 0.95,
            language: 'typescript',
            chunk_type: 'function',
            context: {
              before: 'import { test } from "test";',
              after: 'export default test;',
            },
          },
        ],
        totalCount: 1,
        executionTimeMs: expect.any(Number),
        filters: {},
        cached: false,
      });

      expect(mockSpawn).toHaveBeenCalledWith(
        '/usr/local/bin/crewchief-maproom',
        ['search', 'test query', '--format', 'json'],
        { stdio: ['pipe', 'pipe', 'pipe'] }
      );
    });

    it('should apply search filters correctly', async () => {
      const filters: SearchFilters = {
        worktree: 'test-worktree',
        language: 'typescript',
        maxResults: 10,
        relevanceThreshold: 0.5,
      };

      const searchPromise = service.search('test query', filters);
      
      // Simulate empty results
      mockChildProcess.stdoutCallback('{"hits": []}');
      setTimeout(() => {
        mockChildProcess.closeCallback(0);
      }, 10);

      await searchPromise;

      expect(mockSpawn).toHaveBeenCalledWith(
        '/usr/local/bin/crewchief-maproom',
        [
          'search',
          'test query',
          '--worktree',
          'test-worktree',
          '--language',
          'typescript',
          '--limit',
          '10',
          '--threshold',
          '0.5',
          '--format',
          'json',
        ],
        { stdio: ['pipe', 'pipe', 'pipe'] }
      );
    });

    it('should return cached results when available', async () => {
      // First search
      const searchPromise1 = service.search('cached query');
      mockChildProcess.stdoutCallback('{"hits": [{"chunk_id": "test"}]}');
      setTimeout(() => {
        mockChildProcess.closeCallback(0);
      }, 10);
      
      const result1 = await searchPromise1;
      expect(result1.cached).toBe(false);

      // Second search should return cached result
      const result2 = await service.search('cached query');
      expect(result2.cached).toBe(true);
      expect(result2.results).toEqual(result1.results);
    });

    it('should handle search errors', async () => {
      const searchPromise = service.search('failing query');
      
      // Simulate process close with error
      setTimeout(() => {
        mockChildProcess.closeCallback(1);
      }, 10);

      await expect(searchPromise).rejects.toThrow('Search operation failed');
    });

    it('should handle command timeout', async () => {
      vi.useFakeTimers();
      
      const searchPromise = service.search('slow query');
      
      // Fast-forward time to trigger timeout
      vi.advanceTimersByTime(6000);

      await expect(searchPromise).rejects.toThrow('Command timed out');
      
      vi.useRealTimers();
    });
  });

  describe('getStatus', () => {
    it('should return index status', async () => {
      const mockStatus = {
        repos: [
          {
            id: 1,
            name: 'test-repo',
            path: '/path/to/repo',
            worktrees: [
              {
                id: 1,
                name: 'main',
                path: '/path/to/worktree',
                lastIndexed: '2024-01-01T00:00:00Z',
                fileCount: 100,
                chunkCount: 500,
              },
            ],
          },
        ],
        total_files: 100,
        total_chunks: 500,
        last_updated: '2024-01-01T00:00:00Z',
      };

      const statusPromise = service.getStatus();
      
      mockChildProcess.stdoutCallback(JSON.stringify(mockStatus));
      setTimeout(() => {
        mockChildProcess.closeCallback(0);
      }, 10);

      const result = await statusPromise;

      expect(result).toEqual({
        repos: mockStatus.repos,
        totalFiles: 100,
        totalChunks: 500,
        lastUpdated: '2024-01-01T00:00:00Z',
      });

      expect(mockSpawn).toHaveBeenCalledWith(
        '/usr/local/bin/crewchief-maproom',
        ['status', '--format', 'json'],
        { stdio: ['pipe', 'pipe', 'pipe'] }
      );
    });
  });

  describe('index', () => {
    it('should start indexing operation', async () => {
      const mockIndexResult = {
        files_processed: 50,
        total_files: 100,
      };

      const indexPromise = service.index(['/path/to/index'], {
        repo: 'test-repo',
        worktree: 'test-worktree',
        incremental: true,
      });

      // Simulate successful indexing
      mockChildProcess.stdoutCallback(JSON.stringify(mockIndexResult));
      setTimeout(() => {
        mockChildProcess.closeCallback(0);
      }, 200); // Longer delay to allow initial promise resolution

      const result = await indexPromise;

      expect(result).toEqual({
        processId: expect.any(String),
        status: 'running',
        filesProcessed: 0,
        totalFiles: 0,
        startTime: expect.any(String),
      });

      expect(mockSpawn).toHaveBeenCalledWith(
        '/usr/local/bin/crewchief-maproom',
        [
          'scan',
          '--repo',
          'test-repo',
          '--worktree',
          'test-worktree',
          '--incremental',
          '/path/to/index',
          '--format',
          'json',
        ],
        { stdio: ['pipe', 'pipe', 'pipe'] }
      );
    });
  });

  describe('upsert', () => {
    it('should update specific files', async () => {
      const upsertPromise = service.upsert(['/path/to/file.ts'], {
        repo: 'test-repo',
        worktree: 'test-worktree',
        commit: 'abc123',
      });

      // Simulate successful upsert
      setTimeout(() => {
        mockChildProcess.closeCallback(0);
      }, 10);

      await upsertPromise;

      expect(mockSpawn).toHaveBeenCalledWith(
        '/usr/local/bin/crewchief-maproom',
        [
          'upsert',
          '--repo',
          'test-repo',
          '--worktree',
          'test-worktree',
          '--commit',
          'abc123',
          '/path/to/file.ts',
        ],
        { stdio: ['pipe', 'pipe', 'pipe'] }
      );
    });
  });

  describe('healthCheck', () => {
    it('should return healthy status', async () => {
      const healthPromise = service.healthCheck();
      
      mockChildProcess.stdoutCallback('crewchief-maproom 1.0.0\n');
      setTimeout(() => {
        mockChildProcess.closeCallback(0);
      }, 10);

      const result = await healthPromise;

      expect(result).toEqual({
        healthy: true,
        version: 'crewchief-maproom 1.0.0',
      });
    });

    it('should return unhealthy status on error', async () => {
      const healthPromise = service.healthCheck();
      
      setTimeout(() => {
        mockChildProcess.closeCallback(1);
      }, 10);

      const result = await healthPromise;

      expect(result).toEqual({
        healthy: false,
        error: expect.any(String),
      });
    });
  });

  describe('cache management', () => {
    it('should clear cache', () => {
      // First, populate cache
      service.search('test query').catch(() => {}); // Ignore promise
      
      service.clearCache();
      
      const stats = service.getCacheStats();
      expect(stats.size).toBe(0);
    });

    it('should return cache statistics', () => {
      const stats = service.getCacheStats();
      expect(stats).toHaveProperty('size');
      expect(typeof stats.size).toBe('number');
    });
  });

  describe('process management', () => {
    it('should cancel running index operation', async () => {
      const indexPromise = service.index(['/path/to/index']);
      
      // Get the process ID from the initial promise resolution
      const initialResult = await indexPromise;
      const processId = initialResult.processId;
      
      const cancelled = service.cancelIndex(processId);
      expect(cancelled).toBe(true);
      expect(mockChildProcess.kill).toHaveBeenCalledWith('SIGTERM');
    });

    it('should return false when cancelling non-existent process', () => {
      const cancelled = service.cancelIndex('non-existent-id');
      expect(cancelled).toBe(false);
    });
  });
});