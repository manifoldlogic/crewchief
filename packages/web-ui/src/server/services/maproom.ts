/**
 * Maproom Service
 * 
 * Enhanced service layer for integrating with the Maproom binary for indexing and search operations.
 * Implements the Result pattern, caching, audit logging, and authorization.
 */

import { spawn, spawnSync, ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { 
  BaseService, 
  Result, 
  success, 
  failure, 
  ServiceError, 
  ServiceConfig,
  CacheProvider,
  AuditLogger,
} from './base.js';

// Interfaces from the original service
export interface MaproomConfig extends ServiceConfig {
  binaryPath?: string;
  timeout?: number;
  retries?: number;
  retryDelay?: number;
}

export interface SearchFilters {
  worktree?: string;
  fileTypes?: string[];
  language?: string;
  dateRange?: {
    start?: string;
    end?: string;
  };
  relevanceThreshold?: number;
  maxResults?: number;
}

export interface SearchResult {
  id: string;
  file_path: string;
  line_start: number;
  line_end: number;
  content: string;
  relevance_score: number;
  language?: string;
  chunk_type?: string;
  context?: {
    before?: string;
    after?: string;
  };
}

export interface SearchResponse {
  query: string;
  results: SearchResult[];
  totalCount: number;
  executionTimeMs: number;
  filters: SearchFilters;
  cached: boolean;
  correlationId: string;
}

export interface IndexStatus {
  repos: Array<{
    id: number;
    name: string;
    path: string;
    worktrees: Array<{
      id: number;
      name: string;
      path: string;
      lastIndexed?: string;
      fileCount?: number;
      chunkCount?: number;
    }>;
  }>;
  totalFiles: number;
  totalChunks: number;
  lastUpdated: string;
  correlationId: string;
}

export interface IndexProgress {
  processId: string;
  status: 'running' | 'completed' | 'failed';
  filesProcessed: number;
  totalFiles: number;
  startTime: string;
  endTime?: string;
  error?: string;
  correlationId: string;
}

export class MaproomService extends BaseService {
  private binaryPath: string;
  private timeout: number;
  private retries: number;
  private retryDelay: number;
  private runningProcesses = new Map<string, ChildProcess>();

  constructor(
    config: MaproomConfig = {},
    cache?: CacheProvider,
    auditLogger?: AuditLogger,
  ) {
    super(config, cache, auditLogger);
    
    this.binaryPath = config.binaryPath || this.resolveMaproomBinary();
    this.timeout = config.timeout || 30000;
    this.retries = config.retries || 2;
    this.retryDelay = config.retryDelay || 1000;

    if (!this.binaryPath) {
      throw new ServiceError(
        'Maproom binary not found. Set CREWCHIEF_MAPROOM_BIN environment variable or ensure binary is in PATH',
        'MAPROOM_BINARY_NOT_FOUND',
        500,
      );
    }
  }

  /**
   * Resolve the Maproom binary location
   */
  private resolveMaproomBinary(): string {
    const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom';
    const arch = process.arch === 'x64' ? 'x64' : process.arch === 'arm64' ? 'arm64' : process.arch;
    const platform = `${process.platform}-${arch}`;

    // 1) Explicit env override
    const envBin = process.env.CREWCHIEF_MAPROOM_BIN;
    if (envBin && fs.existsSync(envBin)) return envBin;

    // 2) Packaged inside CLI package with platform subdirectory
    try {
      const here = __dirname;
      const out = path.join(here, '..', '..', '..', 'cli', 'bin', platform, execName);
      if (fs.existsSync(out)) return out;
    } catch {
      // ignore errors
    }

    // 3) Packaged in sibling maproom-mcp package (monorepo dev convenience)
    try {
      const here = __dirname;
      const mcp = path.join(here, '..', '..', '..', 'maproom-mcp', 'bin', platform, execName);
      if (fs.existsSync(mcp)) return mcp;
    } catch {
      // ignore errors
    }

    // 4) Global on PATH
    const which = spawnSync('bash', ['-lc', 'command -v crewchief-maproom']);
    if (which.status === 0) return 'crewchief-maproom';

    throw new ServiceError(
      'Maproom binary not found in any expected location',
      'MAPROOM_BINARY_NOT_FOUND',
      500,
    );
  }

  /**
   * Execute a Maproom command with retries and error handling
   */
  private async executeCommand(
    args: string[],
    options: { json?: boolean; timeout?: number } = {},
    userId?: string,
  ): Promise<string> {
    const { json = false, timeout = this.timeout } = options;
    const operation = args[0] || 'unknown';

    await this.auditLog(operation, 'execute_command', false, {
      userId,
      metadata: { args, timeout },
    });

    for (let attempt = 0; attempt <= this.retries; attempt++) {
      try {
        const result = await new Promise<string>((resolve, reject) => {
          const child = spawn(this.binaryPath, args, {
            stdio: ['pipe', 'pipe', 'pipe'],
          });

          let stdout = '';
          let stderr = '';

          child.stdout.on('data', (data) => {
            stdout += data.toString();
          });

          child.stderr.on('data', (data) => {
            stderr += data.toString();
          });

          const timeoutId = setTimeout(() => {
            child.kill('SIGTERM');
            reject(new ServiceError(
              `Command timed out after ${timeout}ms`,
              'MAPROOM_TIMEOUT',
              408,
              { stderr },
              this.correlationId,
            ));
          }, timeout);

          child.on('close', (code) => {
            clearTimeout(timeoutId);
            
            if (code === 0) {
              // Validate JSON if expected
              if (json) {
                try {
                  JSON.parse(stdout);
                } catch (parseError) {
                  reject(new ServiceError(
                    'Invalid JSON response from Maproom',
                    'MAPROOM_INVALID_JSON',
                    502,
                    { stderr, parseError: parseError.message },
                    this.correlationId,
                  ));
                  return;
                }
              }
              resolve(stdout);
            } else {
              reject(new ServiceError(
                `Maproom command failed with exit code ${code}`,
                'MAPROOM_COMMAND_FAILED',
                502,
                { exitCode: code, stderr },
                this.correlationId,
              ));
            }
          });

          child.on('error', (error) => {
            clearTimeout(timeoutId);
            reject(new ServiceError(
              `Failed to spawn Maproom process: ${error.message}`,
              'MAPROOM_SPAWN_ERROR',
              500,
              { error: error.message, stderr },
              this.correlationId,
            ));
          });
        });

        await this.auditLog(operation, 'execute_command', true, {
          userId,
          metadata: { args, attempt: attempt + 1 },
        });

        return result;
      } catch (error) {
        if (attempt === this.retries) {
          await this.auditLog(operation, 'execute_command', false, {
            userId,
            error: error.message,
            metadata: { args, finalAttempt: true },
          });
          throw error;
        }
        
        console.warn(`Maproom command attempt ${attempt + 1} failed, retrying:`, error);
        await new Promise(resolve => setTimeout(resolve, this.retryDelay));
      }
    }

    throw new ServiceError(
      'Command failed after all retries',
      'MAPROOM_MAX_RETRIES_EXCEEDED',
      500,
      undefined,
      this.correlationId,
    );
  }

  /**
   * Search for code using semantic or full-text search
   */
  async search(
    query: string, 
    filters: SearchFilters = {},
    userId?: string,
  ): Promise<Result<SearchResponse>> {
    try {
      this.checkAuthorization(userId, 'search');
      
      const startTime = Date.now();
      const cacheKey = `maproom:search:${JSON.stringify({ query, filters })}`;

      // Try cache first
      const cachedResult = await this.withCache(
        cacheKey,
        async () => {
          // Build search command arguments
          const args = ['search', query];
          
          if (filters.worktree) {
            args.push('--worktree', filters.worktree);
          }
          
          if (filters.language) {
            args.push('--language', filters.language);
          }
          
          if (filters.maxResults) {
            args.push('--limit', filters.maxResults.toString());
          }
          
          if (filters.relevanceThreshold) {
            args.push('--threshold', filters.relevanceThreshold.toString());
          }

          args.push('--format', 'json');

          const output = await this.executeCommand(args, { json: true }, userId);
          const rawResults = JSON.parse(output);

          // Transform results to our format
          const results: SearchResult[] = rawResults.hits?.map((hit: any) => ({
            id: hit.chunk_id || `${hit.file_path}:${hit.line_start}`,
            file_path: hit.file_path,
            line_start: hit.line_start,
            line_end: hit.line_end,
            content: hit.content,
            relevance_score: hit.score || 0,
            language: hit.language,
            chunk_type: hit.chunk_type,
            context: hit.context,
          })) || [];

          return {
            query,
            results,
            totalCount: results.length,
            executionTimeMs: Date.now() - startTime,
            filters,
            cached: false,
            correlationId: this.correlationId,
          };
        },
        300, // 5 minutes cache
      );

      await this.auditLog('search', 'search_code', true, {
        userId,
        metadata: { 
          query, 
          filters, 
          resultCount: cachedResult.results.length,
          executionTimeMs: cachedResult.executionTimeMs,
          cached: cachedResult.cached,
        },
      });

      return success(cachedResult, this.correlationId);
    } catch (error) {
      await this.auditLog('search', 'search_code', false, {
        userId,
        error: error.message,
        metadata: { query, filters },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Search operation failed: ${error.message}`,
          'MAPROOM_SEARCH_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get the status of the Maproom index
   */
  async getStatus(userId?: string): Promise<Result<IndexStatus>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = 'maproom:status';
      
      const statusData = await this.withCache(
        cacheKey,
        async () => {
          const output = await this.executeCommand(['status', '--format', 'json'], { json: true }, userId);
          const rawStatus = JSON.parse(output);

          return {
            repos: rawStatus.repos || [],
            totalFiles: rawStatus.total_files || 0,
            totalChunks: rawStatus.total_chunks || 0,
            lastUpdated: rawStatus.last_updated || new Date().toISOString(),
            correlationId: this.correlationId,
          };
        },
        60, // 1 minute cache
      );

      await this.auditLog('status', 'get_index_status', true, {
        userId,
        metadata: { 
          totalFiles: statusData.totalFiles,
          totalChunks: statusData.totalChunks,
        },
      });

      return success(statusData, this.correlationId);
    } catch (error) {
      await this.auditLog('status', 'get_index_status', false, {
        userId,
        error: error.message,
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Status operation failed: ${error.message}`,
          'MAPROOM_STATUS_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Index files/directories
   */
  async index(
    paths: string[], 
    options: { repo?: string; worktree?: string; incremental?: boolean } = {},
    userId?: string,
  ): Promise<Result<IndexProgress>> {
    try {
      this.checkAuthorization(userId, 'write');

      const processId = `index_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
      
      const args = ['scan'];
      
      if (options.repo) {
        args.push('--repo', options.repo);
      }
      
      if (options.worktree) {
        args.push('--worktree', options.worktree);
      }
      
      if (options.incremental) {
        args.push('--incremental');
      }

      args.push(...paths);
      args.push('--format', 'json');

      const startTime = new Date().toISOString();

      // Start the indexing process
      const child = spawn(this.binaryPath, args, {
        stdio: ['pipe', 'pipe', 'pipe'],
      });

      this.runningProcesses.set(processId, child);

      let stdout = '';
      let stderr = '';

      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      const progressPromise = new Promise<IndexProgress>((resolve, reject) => {
        child.on('close', (code) => {
          this.runningProcesses.delete(processId);
          
          if (code === 0) {
            try {
              const result = JSON.parse(stdout);
              const progress: IndexProgress = {
                processId,
                status: 'completed',
                filesProcessed: result.files_processed || 0,
                totalFiles: result.total_files || 0,
                startTime,
                endTime: new Date().toISOString(),
                correlationId: this.correlationId,
              };

              this.auditLog('index', 'index_files', true, {
                userId,
                metadata: { 
                  paths, 
                  options, 
                  filesProcessed: progress.filesProcessed,
                  processId,
                },
              });

              resolve(progress);
            } catch (parseError) {
              reject(new ServiceError(
                'Invalid JSON response from index operation',
                'MAPROOM_INDEX_INVALID_JSON',
                502,
                { parseError: parseError.message, stderr },
                this.correlationId,
              ));
            }
          } else {
            reject(new ServiceError(
              `Index operation failed with exit code ${code}`,
              'MAPROOM_INDEX_FAILED',
              502,
              { exitCode: code, stderr },
              this.correlationId,
            ));
          }
        });

        child.on('error', (error) => {
          this.runningProcesses.delete(processId);
          reject(new ServiceError(
            `Failed to spawn index process: ${error.message}`,
            'MAPROOM_INDEX_SPAWN_ERROR',
            500,
            { error: error.message, stderr },
            this.correlationId,
          ));
        });
      });

      // Clear cache after indexing
      await this.clearCachePattern('maproom:*');

      await this.auditLog('index', 'start_indexing', true, {
        userId,
        metadata: { paths, options, processId },
      });

      // Return initial progress immediately
      const initialProgress: IndexProgress = {
        processId,
        status: 'running',
        filesProcessed: 0,
        totalFiles: 0,
        startTime,
        correlationId: this.correlationId,
      };

      return success(initialProgress, this.correlationId);
    } catch (error) {
      await this.auditLog('index', 'start_indexing', false, {
        userId,
        error: error.message,
        metadata: { paths, options },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Index operation failed: ${error.message}`,
          'MAPROOM_INDEX_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Update specific files
   */
  async upsert(
    paths: string[], 
    options: { repo?: string; worktree?: string; commit?: string } = {},
    userId?: string,
  ): Promise<Result<void>> {
    try {
      this.checkAuthorization(userId, 'write');

      const args = ['upsert'];
      
      if (options.repo) {
        args.push('--repo', options.repo);
      }
      
      if (options.worktree) {
        args.push('--worktree', options.worktree);
      }
      
      if (options.commit) {
        args.push('--commit', options.commit);
      }

      args.push(...paths);

      await this.executeCommand(args, {}, userId);

      // Clear cache after upserting
      await this.clearCachePattern('maproom:*');

      await this.auditLog('upsert', 'upsert_files', true, {
        userId,
        metadata: { paths, options },
      });

      return success(undefined, this.correlationId);
    } catch (error) {
      await this.auditLog('upsert', 'upsert_files', false, {
        userId,
        error: error.message,
        metadata: { paths, options },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Upsert operation failed: ${error.message}`,
          'MAPROOM_UPSERT_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get progress of a running indexing operation
   */
  async getIndexProgress(processId: string, userId?: string): Promise<Result<IndexProgress | null>> {
    try {
      this.checkAuthorization(userId, 'read');

      const process = this.runningProcesses.get(processId);
      if (!process) {
        return success(null, this.correlationId);
      }

      const progress: IndexProgress = {
        processId,
        status: 'running',
        filesProcessed: 0, // This would need to be tracked from process output
        totalFiles: 0,     // This would need to be tracked from process output
        startTime: new Date().toISOString(), // This should be stored when process starts
        correlationId: this.correlationId,
      };

      return success(progress, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get index progress: ${error.message}`,
          'MAPROOM_PROGRESS_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Cancel a running indexing operation
   */
  async cancelIndex(processId: string, userId?: string): Promise<Result<boolean>> {
    try {
      this.checkAuthorization(userId, 'write');

      const process = this.runningProcesses.get(processId);
      if (process) {
        process.kill('SIGTERM');
        this.runningProcesses.delete(processId);

        await this.auditLog('index', 'cancel_indexing', true, {
          userId,
          metadata: { processId },
        });

        return success(true, this.correlationId);
      }

      return success(false, this.correlationId);
    } catch (error) {
      await this.auditLog('index', 'cancel_indexing', false, {
        userId,
        error: error.message,
        metadata: { processId },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to cancel index: ${error.message}`,
          'MAPROOM_CANCEL_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Health check for the Maproom service
   */
  async healthCheck(): Promise<{ healthy: boolean; details?: any }> {
    try {
      const output = await this.executeCommand(['--version'], { timeout: 5000 });
      return {
        healthy: true,
        details: {
          version: output.trim(),
          binaryPath: this.binaryPath,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    } catch (error) {
      return {
        healthy: false,
        details: {
          error: error.message,
          binaryPath: this.binaryPath,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    }
  }
}

// Export factory function for dependency injection
export function createMaproomService(
  config?: MaproomConfig,
  cache?: CacheProvider,
  auditLogger?: AuditLogger,
): MaproomService {
  return new MaproomService(config, cache, auditLogger);
}