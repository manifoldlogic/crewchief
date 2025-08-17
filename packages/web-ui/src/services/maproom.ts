/**
 * Maproom Service
 * 
 * Service layer for integrating with the Maproom binary for indexing and search operations.
 * Handles binary execution, process management, output parsing, and error handling.
 */

import { spawn, spawnSync, ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { getDatabase } from '../db/connection.js';

export interface MaproomConfig {
  binaryPath?: string;
  timeout: number;
  retries: number;
  retryDelay: number;
  cacheEnabled: boolean;
  cacheTtl: number; // in milliseconds
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
}

export interface IndexProgress {
  processId: string;
  status: 'running' | 'completed' | 'failed';
  filesProcessed: number;
  totalFiles: number;
  startTime: string;
  endTime?: string;
  error?: string;
}

export interface MaproomError extends Error {
  code: string;
  exitCode?: number;
  stderr?: string;
  operation: string;
}

// Cache for search results
interface CacheEntry {
  data: SearchResponse;
  timestamp: number;
  ttl: number;
}

export class MaproomService {
  private config: MaproomConfig;
  private cache = new Map<string, CacheEntry>();
  private runningProcesses = new Map<string, ChildProcess>();

  constructor(config: Partial<MaproomConfig> = {}) {
    this.config = {
      binaryPath: config.binaryPath || this.resolveMaproomBinary(),
      timeout: config.timeout || 30000,
      retries: config.retries || 2,
      retryDelay: config.retryDelay || 1000,
      cacheEnabled: config.cacheEnabled ?? true,
      cacheTtl: config.cacheTtl || 300000, // 5 minutes default
    };

    if (!this.config.binaryPath) {
      throw new Error('Maproom binary not found. Set CREWCHIEF_MAPROOM_BIN environment variable or ensure binary is in PATH');
    }

    // Clean up cache periodically
    setInterval(() => this.cleanExpiredCache(), 60000); // Every minute
  }

  /**
   * Resolve the Maproom binary location
   * Based on the patterns from packages/cli/src/cli/maproom.ts
   */
  private resolveMaproomBinary(): string | null {
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

    return null;
  }

  /**
   * Execute a Maproom command with retries and error handling
   */
  private async executeCommand(
    args: string[],
    options: { json?: boolean; timeout?: number } = {}
  ): Promise<string> {
    const { json = false, timeout = this.config.timeout } = options;

    for (let attempt = 0; attempt <= this.config.retries; attempt++) {
      try {
        return await new Promise<string>((resolve, reject) => {
          const child = spawn(this.config.binaryPath!, args, {
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
            reject(this.createError('TIMEOUT', `Command timed out after ${timeout}ms`, 'execute', undefined, stderr));
          }, timeout);

          child.on('close', (code) => {
            clearTimeout(timeoutId);
            
            if (code === 0) {
              // Validate JSON if expected
              if (json) {
                try {
                  JSON.parse(stdout);
                } catch (parseError) {
                  reject(this.createError('INVALID_JSON', 'Invalid JSON response from Maproom', 'execute', code, stderr));
                  return;
                }
              }
              resolve(stdout);
            } else {
              reject(this.createError('COMMAND_FAILED', `Maproom command failed with exit code ${code}`, 'execute', code, stderr));
            }
          });

          child.on('error', (error) => {
            clearTimeout(timeoutId);
            reject(this.createError('SPAWN_ERROR', `Failed to spawn Maproom process: ${error.message}`, 'execute', undefined, stderr));
          });
        });
      } catch (error) {
        if (attempt === this.config.retries) {
          throw error;
        }
        
        console.warn(`Maproom command attempt ${attempt + 1} failed, retrying:`, error);
        await new Promise(resolve => setTimeout(resolve, this.config.retryDelay));
      }
    }

    throw new Error('Command failed after all retries');
  }

  /**
   * Create a standardized MaproomError
   */
  private createError(code: string, message: string, operation: string, exitCode?: number, stderr?: string): MaproomError {
    const error = new Error(message) as MaproomError;
    error.code = code;
    error.operation = operation;
    error.exitCode = exitCode;
    error.stderr = stderr;
    return error;
  }

  /**
   * Generate cache key for search operations
   */
  private getCacheKey(query: string, filters: SearchFilters): string {
    return JSON.stringify({ query, filters });
  }

  /**
   * Clean expired cache entries
   */
  private cleanExpiredCache(): void {
    const now = Date.now();
    for (const [key, entry] of this.cache.entries()) {
      if (now - entry.timestamp > entry.ttl) {
        this.cache.delete(key);
      }
    }
  }

  /**
   * Search for code using semantic or full-text search
   */
  async search(query: string, filters: SearchFilters = {}): Promise<SearchResponse> {
    const startTime = Date.now();
    const cacheKey = this.getCacheKey(query, filters);

    // Check cache first
    if (this.config.cacheEnabled) {
      const cached = this.cache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < cached.ttl) {
        return { ...cached.data, cached: true };
      }
    }

    try {
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

      const output = await this.executeCommand(args, { json: true });
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

      const response: SearchResponse = {
        query,
        results,
        totalCount: results.length,
        executionTimeMs: Date.now() - startTime,
        filters,
        cached: false,
      };

      // Cache the results
      if (this.config.cacheEnabled) {
        this.cache.set(cacheKey, {
          data: response,
          timestamp: Date.now(),
          ttl: this.config.cacheTtl,
        });
      }

      return response;
    } catch (error) {
      throw this.createError('SEARCH_FAILED', `Search operation failed: ${error.message}`, 'search');
    }
  }

  /**
   * Get the status of the Maproom index
   */
  async getStatus(): Promise<IndexStatus> {
    try {
      const output = await this.executeCommand(['status', '--format', 'json'], { json: true });
      const statusData = JSON.parse(output);

      return {
        repos: statusData.repos || [],
        totalFiles: statusData.total_files || 0,
        totalChunks: statusData.total_chunks || 0,
        lastUpdated: statusData.last_updated || new Date().toISOString(),
      };
    } catch (error) {
      throw this.createError('STATUS_FAILED', `Status operation failed: ${error.message}`, 'status');
    }
  }

  /**
   * Index files/directories
   */
  async index(paths: string[], options: { repo?: string; worktree?: string; incremental?: boolean } = {}): Promise<IndexProgress> {
    const processId = `index_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    try {
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
      const child = spawn(this.config.binaryPath!, args, {
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

      return new Promise((resolve, reject) => {
        child.on('close', (code) => {
          this.runningProcesses.delete(processId);
          
          if (code === 0) {
            try {
              const result = JSON.parse(stdout);
              resolve({
                processId,
                status: 'completed',
                filesProcessed: result.files_processed || 0,
                totalFiles: result.total_files || 0,
                startTime,
                endTime: new Date().toISOString(),
              });
            } catch (parseError) {
              reject(this.createError('INVALID_JSON', 'Invalid JSON response from index operation', 'index', code, stderr));
            }
          } else {
            reject(this.createError('INDEX_FAILED', `Index operation failed with exit code ${code}`, 'index', code, stderr));
          }
        });

        child.on('error', (error) => {
          this.runningProcesses.delete(processId);
          reject(this.createError('SPAWN_ERROR', `Failed to spawn index process: ${error.message}`, 'index', undefined, stderr));
        });

        // Return initial progress immediately
        setTimeout(() => {
          resolve({
            processId,
            status: 'running',
            filesProcessed: 0,
            totalFiles: 0,
            startTime,
          });
        }, 100);
      });
    } catch (error) {
      throw this.createError('INDEX_FAILED', `Index operation failed: ${error.message}`, 'index');
    }
  }

  /**
   * Update specific files
   */
  async upsert(paths: string[], options: { repo?: string; worktree?: string; commit?: string } = {}): Promise<void> {
    try {
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

      await this.executeCommand(args);
    } catch (error) {
      throw this.createError('UPSERT_FAILED', `Upsert operation failed: ${error.message}`, 'upsert');
    }
  }

  /**
   * Get progress of a running indexing operation
   */
  getIndexProgress(processId: string): IndexProgress | null {
    const process = this.runningProcesses.get(processId);
    if (!process) {
      return null;
    }

    return {
      processId,
      status: 'running',
      filesProcessed: 0, // This would need to be tracked from process output
      totalFiles: 0,     // This would need to be tracked from process output
      startTime: new Date().toISOString(), // This should be stored when process starts
    };
  }

  /**
   * Cancel a running indexing operation
   */
  cancelIndex(processId: string): boolean {
    const process = this.runningProcesses.get(processId);
    if (process) {
      process.kill('SIGTERM');
      this.runningProcesses.delete(processId);
      return true;
    }
    return false;
  }

  /**
   * Clear the search cache
   */
  clearCache(): void {
    this.cache.clear();
  }

  /**
   * Get cache statistics
   */
  getCacheStats(): { size: number; hitRate?: number } {
    return {
      size: this.cache.size,
      // Hit rate calculation would require tracking hits/misses
    };
  }

  /**
   * Store search in history (integrates with web_search_history table)
   */
  async storeSearchHistory(
    sessionId: string,
    query: string,
    filters: SearchFilters,
    results: SearchResult[],
    executionTimeMs: number,
    userId?: string
  ): Promise<void> {
    try {
      const db = getDatabase();
      
      await db.query(`
        INSERT INTO web_search_history (
          session_id, user_id, query, search_type, filters, 
          result_count, execution_time_ms, top_results
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      `, [
        sessionId,
        userId || null,
        query,
        'semantic', // Default to semantic search
        JSON.stringify(filters),
        results.length,
        executionTimeMs,
        JSON.stringify(results.slice(0, 10).map(r => ({
          id: r.id,
          file_path: r.file_path,
          relevance_score: r.relevance_score,
          content: r.content.substring(0, 200), // Store first 200 chars
        }))),
      ]);
    } catch (error) {
      console.error('Failed to store search history:', error);
      // Don't throw - this shouldn't break the search operation
    }
  }

  /**
   * Health check for the Maproom service
   */
  async healthCheck(): Promise<{ healthy: boolean; version?: string; error?: string }> {
    try {
      const output = await this.executeCommand(['--version'], { timeout: 5000 });
      return {
        healthy: true,
        version: output.trim(),
      };
    } catch (error) {
      return {
        healthy: false,
        error: error.message,
      };
    }
  }
}

// Export a singleton instance
let maproomServiceInstance: MaproomService | null = null;

export function getMaproomService(config?: Partial<MaproomConfig>): MaproomService {
  if (!maproomServiceInstance) {
    maproomServiceInstance = new MaproomService(config);
  }
  return maproomServiceInstance;
}