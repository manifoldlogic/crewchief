/**
 * Worktree Service
 * 
 * Service layer for git worktree operations including creation, management, and merging.
 * Implements the Result pattern, caching, audit logging, and authorization.
 */

import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs/promises';
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
import { getDatabase } from '../../db/connection.js';

export interface WorktreeConfig extends ServiceConfig {
  baseRepoPath?: string;
  worktreeBasePath?: string;
  maxWorktrees?: number;
  autoCleanup?: boolean;
  cleanupAfterDays?: number;
}

export interface WorktreeInfo {
  id: string;
  name: string;
  path: string;
  branch: string;
  commit: string;
  status: 'active' | 'inactive' | 'merging' | 'conflicted' | 'deleted';
  createdAt: string;
  updatedAt: string;
  createdBy?: string;
  description?: string;
  metadata?: Record<string, any>;
}

export interface WorktreeCreationOptions {
  name: string;
  branch?: string;
  baseBranch?: string;
  description?: string;
  metadata?: Record<string, any>;
  copyIgnoredFiles?: boolean;
}

export interface MergeOptions {
  strategy?: 'merge' | 'squash' | 'rebase';
  message?: string;
  autoDelete?: boolean;
  targetBranch?: string;
}

export interface MergeResult {
  success: boolean;
  targetBranch: string;
  commit?: string;
  conflicts?: string[];
  message?: string;
}

export interface BranchInfo {
  name: string;
  commit: string;
  message: string;
  author: string;
  date: string;
  isRemote: boolean;
  upstream?: string;
}

export class WorktreeService extends BaseService {
  private baseRepoPath: string;
  private worktreeBasePath: string;
  private maxWorktrees: number;
  private autoCleanup: boolean;
  private cleanupAfterDays: number;

  constructor(
    config: WorktreeConfig = {},
    cache?: CacheProvider,
    auditLogger?: AuditLogger,
  ) {
    super(config, cache, auditLogger);
    
    this.baseRepoPath = config.baseRepoPath || process.cwd();
    this.worktreeBasePath = config.worktreeBasePath || path.join(this.baseRepoPath, '.crewchief', 'worktrees');
    this.maxWorktrees = config.maxWorktrees || 50;
    this.autoCleanup = config.autoCleanup ?? true;
    this.cleanupAfterDays = config.cleanupAfterDays || 30;

    // Schedule cleanup if enabled
    if (this.autoCleanup) {
      setInterval(() => this.cleanup(), 24 * 60 * 60 * 1000); // Daily
    }
  }

  /**
   * Execute git command with error handling
   */
  private async executeGitCommand(
    args: string[],
    options: { cwd?: string; timeout?: number } = {},
    userId?: string,
  ): Promise<string> {
    const { cwd = this.baseRepoPath, timeout = 30000 } = options;

    return new Promise((resolve, reject) => {
      const child = spawn('git', args, {
        cwd,
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
          `Git command timed out after ${timeout}ms`,
          'GIT_TIMEOUT',
          408,
          { args, stderr },
          this.correlationId,
        ));
      }, timeout);

      child.on('close', (code) => {
        clearTimeout(timeoutId);
        
        if (code === 0) {
          resolve(stdout.trim());
        } else {
          reject(new ServiceError(
            `Git command failed with exit code ${code}: ${stderr}`,
            'GIT_COMMAND_FAILED',
            502,
            { args, exitCode: code, stderr },
            this.correlationId,
          ));
        }
      });

      child.on('error', (error) => {
        clearTimeout(timeoutId);
        reject(new ServiceError(
          `Failed to spawn git process: ${error.message}`,
          'GIT_SPAWN_ERROR',
          500,
          { args, error: error.message },
          this.correlationId,
        ));
      });
    });
  }

  /**
   * Validate worktree name
   */
  private validateWorktreeName(name: string): void {
    if (!name || name.length < 1 || name.length > 100) {
      throw new ServiceError(
        'Worktree name must be between 1 and 100 characters',
        'INVALID_WORKTREE_NAME',
        400,
        undefined,
        this.correlationId,
      );
    }

    if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
      throw new ServiceError(
        'Worktree name can only contain letters, numbers, underscores, and hyphens',
        'INVALID_WORKTREE_NAME',
        400,
        undefined,
        this.correlationId,
      );
    }
  }

  /**
   * Get worktree path
   */
  private getWorktreePath(name: string): string {
    return path.join(this.worktreeBasePath, name);
  }

  /**
   * Store worktree info in database
   */
  private async storeWorktreeInfo(worktree: WorktreeInfo): Promise<void> {
    const db = getDatabase();
    
    await db.query(`
      INSERT INTO worktree_status (
        id, name, path, branch, commit, status, created_at, updated_at,
        created_by, description, metadata
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
      ON CONFLICT (id) DO UPDATE SET
        status = EXCLUDED.status,
        branch = EXCLUDED.branch,
        commit = EXCLUDED.commit,
        updated_at = EXCLUDED.updated_at,
        description = EXCLUDED.description,
        metadata = EXCLUDED.metadata
    `, [
      worktree.id,
      worktree.name,
      worktree.path,
      worktree.branch,
      worktree.commit,
      worktree.status,
      worktree.createdAt,
      worktree.updatedAt,
      worktree.createdBy,
      worktree.description,
      JSON.stringify(worktree.metadata || {}),
    ]);
  }

  /**
   * Get worktree info from database
   */
  private async getWorktreeInfoFromDb(id: string): Promise<WorktreeInfo | null> {
    const db = getDatabase();
    
    const result = await db.query(`
      SELECT * FROM worktree_status WHERE id = $1
    `, [id]);

    if (result.rows.length === 0) {
      return null;
    }

    const row = result.rows[0];
    return {
      id: row.id,
      name: row.name,
      path: row.path,
      branch: row.branch,
      commit: row.commit,
      status: row.status,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      createdBy: row.created_by,
      description: row.description,
      metadata: row.metadata || {},
    };
  }

  /**
   * Create a new worktree
   */
  async createWorktree(
    options: WorktreeCreationOptions,
    userId?: string,
  ): Promise<Result<WorktreeInfo>> {
    try {
      this.checkAuthorization(userId, 'write');
      this.validateWorktreeName(options.name);

      // Check if worktree already exists
      const existingWorktree = await this.getWorktreeInfoFromDb(options.name);
      if (existingWorktree && existingWorktree.status !== 'deleted') {
        return failure(
          new ServiceError(
            `Worktree '${options.name}' already exists`,
            'WORKTREE_ALREADY_EXISTS',
            409,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      // Check max worktrees limit
      const activeWorktrees = await this.listWorktrees(userId);
      if (activeWorktrees.success && activeWorktrees.data.length >= this.maxWorktrees) {
        return failure(
          new ServiceError(
            `Maximum number of worktrees (${this.maxWorktrees}) reached`,
            'MAX_WORKTREES_EXCEEDED',
            429,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      const worktreePath = this.getWorktreePath(options.name);
      const baseBranch = options.baseBranch || 'main';
      const newBranch = options.branch || `worktree/${options.name}`;

      // Ensure worktree base directory exists
      await fs.mkdir(this.worktreeBasePath, { recursive: true });

      // Create the worktree
      await this.executeGitCommand([
        'worktree', 'add', 
        '--track', 
        '-b', newBranch,
        worktreePath,
        baseBranch,
      ], {}, userId);

      // Get current commit
      const commit = await this.executeGitCommand(['rev-parse', 'HEAD'], { cwd: worktreePath }, userId);

      // Copy ignored files if requested
      if (options.copyIgnoredFiles) {
        await this.copyIgnoredFiles(worktreePath, userId);
      }

      const worktreeInfo: WorktreeInfo = {
        id: options.name,
        name: options.name,
        path: worktreePath,
        branch: newBranch,
        commit,
        status: 'active',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        createdBy: userId,
        description: options.description,
        metadata: options.metadata,
      };

      await this.storeWorktreeInfo(worktreeInfo);

      // Clear cache
      await this.clearCachePattern('worktrees:*');

      await this.auditLog('worktree', 'create_worktree', true, {
        userId,
        resource: options.name,
        metadata: { 
          worktreePath,
          branch: newBranch,
          baseBranch,
          options,
        },
      });

      return success(worktreeInfo, this.correlationId);
    } catch (error) {
      await this.auditLog('worktree', 'create_worktree', false, {
        userId,
        resource: options.name,
        error: error.message,
        metadata: { options },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to create worktree: ${error.message}`,
          'WORKTREE_CREATION_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * List all worktrees
   */
  async listWorktrees(userId?: string): Promise<Result<WorktreeInfo[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = 'worktrees:list';
      
      const worktrees = await this.withCache(
        cacheKey,
        async () => {
          const db = getDatabase();
          const result = await db.query(`
            SELECT * FROM worktree_status 
            WHERE status != 'deleted'
            ORDER BY created_at DESC
          `);

          return result.rows.map(row => ({
            id: row.id,
            name: row.name,
            path: row.path,
            branch: row.branch,
            commit: row.commit,
            status: row.status,
            createdAt: row.created_at,
            updatedAt: row.updated_at,
            createdBy: row.created_by,
            description: row.description,
            metadata: row.metadata || {},
          }));
        },
        60, // 1 minute cache
      );

      await this.auditLog('worktree', 'list_worktrees', true, {
        userId,
        metadata: { count: worktrees.length },
      });

      return success(worktrees, this.correlationId);
    } catch (error) {
      await this.auditLog('worktree', 'list_worktrees', false, {
        userId,
        error: error.message,
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to list worktrees: ${error.message}`,
          'WORKTREE_LIST_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get specific worktree info
   */
  async getWorktree(name: string, userId?: string): Promise<Result<WorktreeInfo | null>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = `worktrees:info:${name}`;
      
      const worktree = await this.withCache(
        cacheKey,
        async () => {
          return await this.getWorktreeInfoFromDb(name);
        },
        60, // 1 minute cache
      );

      return success(worktree, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get worktree info: ${error.message}`,
          'WORKTREE_GET_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Delete a worktree
   */
  async deleteWorktree(name: string, userId?: string, force = false): Promise<Result<void>> {
    try {
      this.checkAuthorization(userId, 'write');

      const worktree = await this.getWorktreeInfoFromDb(name);
      if (!worktree) {
        return failure(
          new ServiceError(
            `Worktree '${name}' not found`,
            'WORKTREE_NOT_FOUND',
            404,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      // Remove the worktree
      const args = ['worktree', 'remove'];
      if (force) {
        args.push('--force');
      }
      args.push(worktree.path);

      await this.executeGitCommand(args, {}, userId);

      // Update status in database
      worktree.status = 'deleted';
      worktree.updatedAt = new Date().toISOString();
      await this.storeWorktreeInfo(worktree);

      // Clear cache
      await this.clearCachePattern('worktrees:*');

      await this.auditLog('worktree', 'delete_worktree', true, {
        userId,
        resource: name,
        metadata: { force, worktreePath: worktree.path },
      });

      return success(undefined, this.correlationId);
    } catch (error) {
      await this.auditLog('worktree', 'delete_worktree', false, {
        userId,
        resource: name,
        error: error.message,
        metadata: { force },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to delete worktree: ${error.message}`,
          'WORKTREE_DELETE_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Merge a worktree branch
   */
  async mergeWorktree(
    name: string, 
    options: MergeOptions = {},
    userId?: string,
  ): Promise<Result<MergeResult>> {
    try {
      this.checkAuthorization(userId, 'write');

      const worktree = await this.getWorktreeInfoFromDb(name);
      if (!worktree) {
        return failure(
          new ServiceError(
            `Worktree '${name}' not found`,
            'WORKTREE_NOT_FOUND',
            404,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      const targetBranch = options.targetBranch || 'main';
      const strategy = options.strategy || 'merge';

      // Switch to target branch
      await this.executeGitCommand(['checkout', targetBranch], {}, userId);

      let mergeCommand: string[];
      switch (strategy) {
        case 'squash':
          mergeCommand = ['merge', '--squash', worktree.branch];
          break;
        case 'rebase':
          mergeCommand = ['rebase', worktree.branch];
          break;
        default:
          mergeCommand = ['merge', worktree.branch];
      }

      if (options.message) {
        mergeCommand.push('-m', options.message);
      }

      try {
        await this.executeGitCommand(mergeCommand, {}, userId);
        
        // Get the new commit hash
        const commit = await this.executeGitCommand(['rev-parse', 'HEAD'], {}, userId);

        const result: MergeResult = {
          success: true,
          targetBranch,
          commit,
          message: options.message,
        };

        // Auto-delete if requested
        if (options.autoDelete) {
          await this.deleteWorktree(name, userId, true);
        }

        await this.auditLog('worktree', 'merge_worktree', true, {
          userId,
          resource: name,
          metadata: { 
            targetBranch,
            strategy,
            commit,
            autoDelete: options.autoDelete,
          },
        });

        return success(result, this.correlationId);
      } catch (mergeError) {
        // Check for conflicts
        const statusOutput = await this.executeGitCommand(['status', '--porcelain'], {}, userId);
        const conflicts = statusOutput
          .split('\n')
          .filter(line => line.startsWith('UU'))
          .map(line => line.slice(3));

        const result: MergeResult = {
          success: false,
          targetBranch,
          conflicts,
          message: `Merge failed: ${mergeError.message}`,
        };

        // Update worktree status
        worktree.status = 'conflicted';
        worktree.updatedAt = new Date().toISOString();
        await this.storeWorktreeInfo(worktree);

        return success(result, this.correlationId);
      }
    } catch (error) {
      await this.auditLog('worktree', 'merge_worktree', false, {
        userId,
        resource: name,
        error: error.message,
        metadata: { options },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to merge worktree: ${error.message}`,
          'WORKTREE_MERGE_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * List branches
   */
  async listBranches(userId?: string, includeRemote = false): Promise<Result<BranchInfo[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = `worktrees:branches:${includeRemote}`;
      
      const branches = await this.withCache(
        cacheKey,
        async () => {
          const args = ['branch', '--format=%(refname:short)|%(objectname)|%(contents:subject)|%(authorname)|%(authordate:iso)'];
          if (includeRemote) {
            args.push('-a');
          }

          const output = await this.executeGitCommand(args, {}, userId);
          
          return output.split('\n')
            .filter(line => line.trim())
            .map(line => {
              const [name, commit, message, author, date] = line.split('|');
              return {
                name: name.trim(),
                commit,
                message,
                author,
                date,
                isRemote: name.startsWith('remotes/'),
                upstream: undefined, // Could be populated with git branch -vv
              };
            });
        },
        300, // 5 minutes cache
      );

      return success(branches, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to list branches: ${error.message}`,
          'BRANCH_LIST_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Copy ignored files from base repo to worktree
   */
  private async copyIgnoredFiles(worktreePath: string, userId?: string): Promise<void> {
    try {
      // Common ignored files to copy
      const filesToCopy = ['.env', '.env.local', '.env.development', 'node_modules/.env'];
      
      for (const file of filesToCopy) {
        const sourcePath = path.join(this.baseRepoPath, file);
        const targetPath = path.join(worktreePath, file);
        
        try {
          await fs.access(sourcePath);
          await fs.copyFile(sourcePath, targetPath);
        } catch {
          // File doesn't exist, skip
        }
      }
    } catch (error) {
      console.warn('Failed to copy ignored files:', error);
      // Don't throw - this is not critical
    }
  }

  /**
   * Cleanup old worktrees
   */
  private async cleanup(): Promise<void> {
    try {
      const cutoffDate = new Date();
      cutoffDate.setDate(cutoffDate.getDate() - this.cleanupAfterDays);

      const db = getDatabase();
      const result = await db.query(`
        SELECT * FROM worktree_status 
        WHERE status = 'inactive' 
        AND updated_at < $1
      `, [cutoffDate.toISOString()]);

      for (const row of result.rows) {
        try {
          await this.deleteWorktree(row.name, 'system', true);
        } catch (error) {
          console.warn(`Failed to cleanup worktree ${row.name}:`, error);
        }
      }
    } catch (error) {
      console.warn('Worktree cleanup failed:', error);
    }
  }

  /**
   * Health check for the Worktree service
   */
  async healthCheck(): Promise<{ healthy: boolean; details?: any }> {
    try {
      // Check if git is available
      await this.executeGitCommand(['--version']);
      
      // Check if base repo is a git repository
      await this.executeGitCommand(['status']);
      
      // Check if worktree base path is accessible
      await fs.access(this.worktreeBasePath);

      return {
        healthy: true,
        details: {
          baseRepoPath: this.baseRepoPath,
          worktreeBasePath: this.worktreeBasePath,
          maxWorktrees: this.maxWorktrees,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    } catch (error) {
      return {
        healthy: false,
        details: {
          error: error.message,
          baseRepoPath: this.baseRepoPath,
          worktreeBasePath: this.worktreeBasePath,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    }
  }
}

// Export factory function for dependency injection
export function createWorktreeService(
  config?: WorktreeConfig,
  cache?: CacheProvider,
  auditLogger?: AuditLogger,
): WorktreeService {
  return new WorktreeService(config, cache, auditLogger);
}