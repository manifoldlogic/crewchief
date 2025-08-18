import { promises as fs } from 'fs';
import path from 'path';
import { v4 as uuidv4 } from 'uuid';
import { simpleGit, SimpleGit, CleanOptions, ResetMode } from 'simple-git';
import type { 
  BranchSummary, 
  StatusResult, 
  DiffResult, 
  LogResult,
  RemoteWithRefs,
  PullResult,
  PushResult,
  CommitResult,
} from 'simple-git';

import { GitOperationQueue } from './queue.js';
import { GitProgressTracker } from './progress.js';
import { GitSecurityManager } from './security.js';
import {
  GitServiceOptions,
  GitConfig,
  WorktreeInfo,
  BranchInfo,
  CommitInfo,
  FileStatus,
  MergeConflict,
  ConflictSection,
  DiffChunk,
  DiffLine,
  GitOperation,
  GitOperationType,
  ProgressCallback,
  AuthConfig,
  NetworkConfig,
  SecurityConfig,
  GitLogger,
  RetryOptions,
} from './types.js';

export class GitService {
  private readonly config: GitConfig;
  private readonly auth?: AuthConfig;
  private readonly network: NetworkConfig;
  private readonly security: SecurityConfig;
  private readonly logger: GitLogger;
  private readonly queue: GitOperationQueue;
  private readonly securityManager: GitSecurityManager;
  private readonly git: SimpleGit;

  constructor(options: GitServiceOptions) {
    this.config = {
      maxConcurrentOps: 3,
      timeoutMs: 300000, // 5 minutes
      retryAttempts: 3,
      retryDelayMs: 1000,
      maxRepoSizeMB: 1024, // 1GB
      enableProgressTracking: true,
      ...options.config,
    };

    this.auth = options.auth;
    
    this.network = {
      retryAttempts: 3,
      retryDelayMs: 1000,
      timeoutMs: 30000,
      offlineDetection: true,
      ...options.network,
    };

    this.security = {
      allowedProtocols: ['https:', 'ssh:', 'git:'],
      allowedHosts: [],
      maxFileSize: 100 * 1024 * 1024, // 100MB
      sanitizeUrls: true,
      validateSslCerts: true,
      ...options.security,
    };

    this.logger = options.logger || this.createDefaultLogger();
    
    this.queue = new GitOperationQueue({
      maxConcurrentOps: this.config.maxConcurrentOps,
      defaultTimeoutMs: this.config.timeoutMs,
      logger: this.logger,
    });

    this.securityManager = new GitSecurityManager(this.security, this.logger);
    
    // Initialize simple-git with security and configuration
    this.git = simpleGit({
      baseDir: this.config.baseDir,
      binary: 'git',
      maxConcurrentProcesses: this.config.maxConcurrentOps,
      timeout: {
        block: this.config.timeoutMs || 300000,
      },
      config: this.buildGitConfig(),
    });

    this.setupErrorHandling();
  }

  // ============================================================================
  // Worktree Operations
  // ============================================================================

  /**
   * Creates a new git worktree
   */
  async createWorktree(
    worktreePath: string, 
    branch?: string, 
    progressCallback?: ProgressCallback,
  ): Promise<WorktreeInfo> {
    const operation = this.createOperation('worktree-add');
    
    return this.queue.enqueue(
      operation,
      async () => {
        // For worktrees, allow relative paths within or adjacent to base directory
        const fullPath = path.resolve(this.config.baseDir, worktreePath);
        const parentDir = path.dirname(this.config.baseDir);
        if (!this.securityManager.validatePath(fullPath, parentDir)) {
          throw new Error('Invalid worktree path');
        }

        const tracker = new GitProgressTracker({
          method: 'worktree-add',
          repository: this.config.baseDir,
          logger: this.logger,
        });

        if (progressCallback) {
          tracker.onProgress(progressCallback);
        }

        try {
          tracker.updateProgress({ stage: 'creating_worktree', progress: 25 });
          
          const args = ['worktree', 'add'];
          if (branch) {
            args.push('-b', branch);
          }
          args.push(worktreePath);
          if (branch) {
            args.push(branch);
          }

          await this.git.raw(args);
          
          tracker.updateProgress({ stage: 'validating_worktree', progress: 75 });
          
          const worktreeInfo = await this.getWorktreeInfo(worktreePath);
          
          tracker.complete();
          
          this.logger.info('Worktree created successfully', {
            path: worktreePath,
            branch: worktreeInfo.branch,
          });
          
          return worktreeInfo;
        } catch (error) {
          tracker.fail(error instanceof Error ? error : new Error(String(error)));
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getRetryOptions(),
    );
  }

  /**
   * Lists all git worktrees
   */
  async listWorktrees(): Promise<WorktreeInfo[]> {
    const operation = this.createOperation('worktree-list');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const result = await this.git.raw(['worktree', 'list', '--porcelain']);
          return this.parseWorktreeList(result);
        } catch (error) {
          this.logger.error('Failed to list worktrees', {
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  /**
   * Removes a git worktree
   */
  async removeWorktree(worktreePath: string, force = false): Promise<void> {
    const operation = this.createOperation('worktree-remove');
    
    return this.queue.enqueue(
      operation,
      async () => {
        // For worktrees, allow relative paths within or adjacent to base directory  
        const fullPath = path.resolve(this.config.baseDir, worktreePath);
        const parentDir = path.dirname(this.config.baseDir);
        if (!this.securityManager.validatePath(fullPath, parentDir)) {
          throw new Error('Invalid worktree path');
        }

        try {
          const args = ['worktree', 'remove'];
          if (force) {
            args.push('--force');
          }
          args.push(worktreePath);
          
          await this.git.raw(args);
          
          this.logger.info('Worktree removed successfully', { path: worktreePath });
        } catch (error) {
          this.logger.error('Failed to remove worktree', {
            path: worktreePath,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getRetryOptions(),
    );
  }

  /**
   * Prunes worktrees
   */
  async pruneWorktrees(): Promise<string[]> {
    const operation = this.createOperation('worktree-prune');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const result = await this.git.raw(['worktree', 'prune', '--verbose']);
          const prunedPaths = this.parsePruneOutput(result);
          
          this.logger.info('Worktrees pruned successfully', {
            count: prunedPaths.length,
            paths: prunedPaths,
          });
          
          return prunedPaths;
        } catch (error) {
          this.logger.error('Failed to prune worktrees', {
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  // ============================================================================
  // Branch Operations
  // ============================================================================

  /**
   * Creates a new branch
   */
  async createBranch(name: string, startPoint?: string): Promise<void> {
    const operation = this.createOperation('branch-create');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          if (startPoint) {
            await this.git.checkoutBranch(name, startPoint);
          } else {
            await this.git.checkoutLocalBranch(name);
          }
          
          this.logger.info('Branch created successfully', { name, startPoint });
        } catch (error) {
          this.logger.error('Failed to create branch', {
            name,
            startPoint,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  /**
   * Checks out a branch
   */
  async checkoutBranch(name: string): Promise<void> {
    const operation = this.createOperation('checkout');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          await this.git.checkout(name);
          this.logger.info('Branch checked out successfully', { name });
        } catch (error) {
          this.logger.error('Failed to checkout branch', {
            name,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  /**
   * Deletes a branch
   */
  async deleteBranch(name: string, force = false): Promise<void> {
    const operation = this.createOperation('branch-delete');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          await this.git.deleteLocalBranch(name, force);
          this.logger.info('Branch deleted successfully', { name, force });
        } catch (error) {
          this.logger.error('Failed to delete branch', {
            name,
            force,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  /**
   * Merges a branch
   */
  async mergeBranch(
    branch: string, 
    options?: { noFF?: boolean; squash?: boolean },
  ): Promise<{ conflicts?: MergeConflict[] }> {
    const operation = this.createOperation('merge');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const mergeOptions: string[] = [];
          
          if (options?.noFF) {
            mergeOptions.push('--no-ff');
          }
          if (options?.squash) {
            mergeOptions.push('--squash');
          }

          const result = await this.git.merge([branch, ...mergeOptions]);
          
          // Check for conflicts
          const status = await this.git.status();
          if (status.conflicted.length > 0) {
            const conflicts = await this.parseConflicts(status.conflicted);
            this.logger.warn('Merge completed with conflicts', {
              branch,
              conflicts: conflicts.length,
            });
            return { conflicts };
          }
          
          this.logger.info('Branch merged successfully', { branch });
          return {};
        } catch (error) {
          this.logger.error('Failed to merge branch', {
            branch,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getRetryOptions(),
    );
  }

  /**
   * Lists branches
   */
  async listBranches(includeRemote = false): Promise<BranchInfo[]> {
    const operation = this.createOperation('branch-list');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const summary: BranchSummary = includeRemote 
            ? await this.git.branch(['-a']) 
            : await this.git.branch();
          
          return this.parseBranchSummary(summary);
        } catch (error) {
          this.logger.error('Failed to list branches', {
            includeRemote,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  // ============================================================================
  // Commit Operations
  // ============================================================================

  /**
   * Adds files to staging area
   */
  async addFiles(files: string[] | '.'): Promise<void> {
    const operation = this.createOperation('add');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          if (Array.isArray(files)) {
            for (const file of files) {
              // Allow relative paths and don't validate '.' and '*' patterns
              if (file !== '.' && file !== '*' && !file.startsWith('./') && !file.startsWith('*')) {
                const fullPath = path.resolve(this.config.baseDir, file);
                if (!this.securityManager.validatePath(fullPath, this.config.baseDir)) {
                  throw new Error(`Invalid file path: ${file}`);
                }
              }
            }
            await this.git.add(files);
          } else {
            await this.git.add(files);
          }
          
          this.logger.info('Files added to staging area', { files });
        } catch (error) {
          this.logger.error('Failed to add files', {
            files,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  /**
   * Commits changes
   */
  async commit(message: string, files?: string[]): Promise<CommitResult> {
    const operation = this.createOperation('commit');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          let result: CommitResult;
          
          if (files) {
            await this.addFiles(files);
          }
          
          result = await this.git.commit(message);
          
          this.logger.info('Changes committed successfully', {
            message,
            commit: result.commit,
            summary: result.summary,
          });
          
          return result;
        } catch (error) {
          this.logger.error('Failed to commit changes', {
            message,
            files,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getRetryOptions(),
    );
  }

  /**
   * Pushes changes to remote
   */
  async push(
    remote = 'origin',
    branch?: string,
    progressCallback?: ProgressCallback,
  ): Promise<PushResult> {
    const operation = this.createOperation('push');
    
    return this.queue.enqueue(
      operation,
      async () => {
        const tracker = new GitProgressTracker({
          method: 'push',
          repository: this.config.baseDir,
          logger: this.logger,
        });

        if (progressCallback) {
          tracker.onProgress(progressCallback);
        }

        try {
          tracker.updateProgress({ stage: 'starting_push', progress: 10 });
          
          // Configure progress tracking for simple-git
          const git = this.git.env(this.securityManager.createSecureEnv(this.auth));
          
          if (this.config.enableProgressTracking) {
            git.outputHandler(tracker.createTransferHandler().progress);
          }

          const result = branch 
            ? await git.push(remote, branch)
            : await git.push();
          
          tracker.complete();
          
          this.logger.info('Changes pushed successfully', {
            remote,
            branch,
            pushed: result.pushed,
          });
          
          return result;
        } catch (error) {
          tracker.fail(error instanceof Error ? error : new Error(String(error)));
          
          if (this.isNetworkError(error)) {
            throw new Error(`Network error while pushing: ${error instanceof Error ? error.message : error}`);
          }
          
          this.logger.error('Failed to push changes', {
            remote,
            branch,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getNetworkRetryOptions(),
    );
  }

  /**
   * Pulls changes from remote
   */
  async pull(
    remote = 'origin',
    branch?: string,
    progressCallback?: ProgressCallback,
  ): Promise<PullResult> {
    const operation = this.createOperation('pull');
    
    return this.queue.enqueue(
      operation,
      async () => {
        const tracker = new GitProgressTracker({
          method: 'pull',
          repository: this.config.baseDir,
          logger: this.logger,
        });

        if (progressCallback) {
          tracker.onProgress(progressCallback);
        }

        try {
          tracker.updateProgress({ stage: 'starting_pull', progress: 10 });
          
          const git = this.git.env(this.securityManager.createSecureEnv(this.auth));
          
          if (this.config.enableProgressTracking) {
            git.outputHandler(tracker.createTransferHandler().progress);
          }

          const result = branch 
            ? await git.pull(remote, branch)
            : await git.pull();
          
          tracker.complete();
          
          this.logger.info('Changes pulled successfully', {
            remote,
            branch,
            summary: result.summary,
          });
          
          return result;
        } catch (error) {
          tracker.fail(error instanceof Error ? error : new Error(String(error)));
          
          if (this.isNetworkError(error)) {
            throw new Error(`Network error while pulling: ${error instanceof Error ? error.message : error}`);
          }
          
          this.logger.error('Failed to pull changes', {
            remote,
            branch,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getNetworkRetryOptions(),
    );
  }

  // ============================================================================
  // Status and Information
  // ============================================================================

  /**
   * Gets repository status
   */
  async getStatus(): Promise<{ files: FileStatus[]; summary: any }> {
    const operation = this.createOperation('status');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const status: StatusResult = await this.git.status();
          const files = this.parseStatusResult(status);
          
          return {
            files,
            summary: {
              current: status.current,
              tracking: status.tracking,
              ahead: status.ahead,
              behind: status.behind,
              staged: status.staged.length,
              modified: status.modified.length,
              created: status.created.length,
              deleted: status.deleted.length,
              renamed: status.renamed.length,
              conflicted: status.conflicted.length,
            },
          };
        } catch (error) {
          this.logger.error('Failed to get status', {
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  /**
   * Gets file diff
   */
  async getDiff(
    file?: string,
    staged = false,
    contextLines = 3,
  ): Promise<DiffChunk[]> {
    const operation = this.createOperation('diff');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const options: string[] = [`--unified=${contextLines}`];
          
          if (staged) {
            options.push('--staged');
          }
          
          if (file && file !== '.' && file !== '*' && !file.startsWith('./')) {
            const fullPath = path.resolve(this.config.baseDir, file);
            if (!this.securityManager.validatePath(fullPath, this.config.baseDir)) {
              throw new Error('Invalid file path');
            }
          }
          
          const result = file
            ? await this.git.diff([...options, '--', file])
            : await this.git.diff(options);
          
          return this.parseDiffOutput(result);
        } catch (error) {
          this.logger.error('Failed to get diff', {
            file,
            staged,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  /**
   * Gets commit log
   */
  async getLog(
    maxCount = 50,
    from?: string,
    to?: string,
  ): Promise<CommitInfo[]> {
    const operation = this.createOperation('log');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const options: any = {
            maxCount,
            format: {
              hash: '%H',
              date: '%ai',
              message: '%s',
              author_name: '%an',
              author_email: '%ae',
              refs: '%D',
            },
          };
          
          if (from && to) {
            options.from = from;
            options.to = to;
          }
          
          const log: LogResult = await this.git.log(options);
          return this.parseLogResult(log);
        } catch (error) {
          this.logger.error('Failed to get log', {
            maxCount,
            from,
            to,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  // ============================================================================
  // Advanced Operations
  // ============================================================================

  /**
   * Clones a repository
   */
  async clone(
    url: string,
    targetPath: string,
    options?: {
      branch?: string;
      depth?: number;
      progressCallback?: ProgressCallback;
    },
  ): Promise<void> {
    const operation = this.createOperation('clone');
    
    return this.queue.enqueue(
      operation,
      async () => {
        if (!this.securityManager.validateGitUrl(url)) {
          throw new Error('Invalid or insecure git URL');
        }
        
        // For clone, allow paths within or adjacent to base directory  
        const fullPath = path.resolve(this.config.baseDir, targetPath);
        const parentDir = path.dirname(this.config.baseDir);
        if (!this.securityManager.validatePath(fullPath, parentDir)) {
          throw new Error('Invalid target path');
        }

        const tracker = new GitProgressTracker({
          method: 'clone',
          repository: url,
          logger: this.logger,
        });

        if (options?.progressCallback) {
          tracker.onProgress(options.progressCallback);
        }

        try {
          tracker.updateProgress({ stage: 'initializing', progress: 5 });
          
          const cloneOptions: string[] = [];
          
          if (options?.branch) {
            cloneOptions.push('--branch', options.branch);
          }
          
          if (options?.depth) {
            cloneOptions.push('--depth', options.depth.toString());
          }

          const git = simpleGit().env(this.securityManager.createSecureEnv(this.auth));
          
          if (this.config.enableProgressTracking) {
            git.outputHandler(tracker.createCloneHandler().progress);
          }

          await git.clone(url, targetPath, cloneOptions);
          
          tracker.complete();
          
          this.logger.info('Repository cloned successfully', {
            url: this.securityManager.validateGitUrl(url) ? '[VALID_URL]' : '[INVALID_URL]',
            targetPath,
            branch: options?.branch,
            depth: options?.depth,
          });
        } catch (error) {
          tracker.fail(error instanceof Error ? error : new Error(String(error)));
          
          if (this.isNetworkError(error)) {
            throw new Error(`Network error while cloning: ${error instanceof Error ? error.message : error}`);
          }
          
          this.logger.error('Failed to clone repository', {
            url: this.securityManager.validateGitUrl(url) ? '[VALID_URL]' : '[INVALID_URL]',
            targetPath,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs! * 2, // Clone operations may take longer
      this.getNetworkRetryOptions(),
    );
  }

  /**
   * Fetches from remote
   */
  async fetch(remote = 'origin', progressCallback?: ProgressCallback): Promise<void> {
    const operation = this.createOperation('fetch');
    
    return this.queue.enqueue(
      operation,
      async () => {
        const tracker = new GitProgressTracker({
          method: 'fetch',
          repository: this.config.baseDir,
          logger: this.logger,
        });

        if (progressCallback) {
          tracker.onProgress(progressCallback);
        }

        try {
          tracker.updateProgress({ stage: 'starting_fetch', progress: 10 });
          
          const git = this.git.env(this.securityManager.createSecureEnv(this.auth));
          
          if (this.config.enableProgressTracking) {
            git.outputHandler(tracker.createTransferHandler().progress);
          }

          await git.fetch(remote);
          
          tracker.complete();
          
          this.logger.info('Fetch completed successfully', { remote });
        } catch (error) {
          tracker.fail(error instanceof Error ? error : new Error(String(error)));
          
          if (this.isNetworkError(error)) {
            throw new Error(`Network error while fetching: ${error instanceof Error ? error.message : error}`);
          }
          
          this.logger.error('Failed to fetch', {
            remote,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getNetworkRetryOptions(),
    );
  }

  /**
   * Resets repository state
   */
  async reset(mode: 'soft' | 'mixed' | 'hard' = 'mixed', commit = 'HEAD'): Promise<void> {
    const operation = this.createOperation('reset');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const resetMode = mode as ResetMode;
          await this.git.reset([resetMode, commit]);
          
          this.logger.info('Reset completed successfully', { mode, commit });
        } catch (error) {
          this.logger.error('Failed to reset', {
            mode,
            commit,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
      this.config.timeoutMs,
      this.getRetryOptions(),
    );
  }

  /**
   * Cleans untracked files
   */
  async clean(options: { dryRun?: boolean; directories?: boolean; force?: boolean } = {}): Promise<string[]> {
    const operation = this.createOperation('clean');
    
    return this.queue.enqueue(
      operation,
      async () => {
        try {
          const cleanOptions: CleanOptions = {
            force: options.force || false,
            dryRun: options.dryRun || false,
            recursive: options.directories || false,
          };
          
          const result = await this.git.clean(CleanOptions.FORCE | CleanOptions.RECURSIVE, cleanOptions);
          
          // Parse clean output to get list of cleaned files
          const cleanedFiles = result.split('\n')
            .filter(line => line.startsWith('Removing '))
            .map(line => line.substring(9)); // Remove "Removing " prefix
          
          this.logger.info('Clean completed successfully', {
            options,
            cleaned: cleanedFiles.length,
          });
          
          return cleanedFiles;
        } catch (error) {
          this.logger.error('Failed to clean', {
            options,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      },
    );
  }

  // ============================================================================
  // Queue Management
  // ============================================================================

  /**
   * Gets current operation queue status
   */
  getQueueStatus() {
    return this.queue.getStatus();
  }

  /**
   * Cancels an operation
   */
  cancelOperation(operationId: string): boolean {
    return this.queue.cancel(operationId);
  }

  /**
   * Clears the operation queue
   */
  clearQueue(): void {
    this.queue.clear();
  }

  // ============================================================================
  // Private Helper Methods
  // ============================================================================

  private createOperation(type: GitOperationType): GitOperation {
    return {
      id: uuidv4(),
      type,
      status: 'pending',
    };
  }

  private getRetryOptions(): RetryOptions {
    return {
      attempts: this.config.retryAttempts || 3,
      delay: this.config.retryDelayMs || 1000,
      backoff: 'exponential',
      maxDelay: 10000,
    };
  }

  private getNetworkRetryOptions(): RetryOptions {
    return {
      attempts: this.network.retryAttempts,
      delay: this.network.retryDelayMs,
      backoff: 'exponential',
      maxDelay: 30000,
    };
  }

  private isNetworkError(error: any): boolean {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return /network|timeout|connection|dns|refused/i.test(errorMessage);
  }

  private buildGitConfig(): string[] {
    const config: string[] = [];
    
    if (this.auth?.type === 'https' && this.auth.token) {
      config.push('credential.helper=store');
    }
    
    // Disable SSL verification if configured
    if (!this.security.validateSslCerts) {
      config.push('http.sslVerify=false');
    }
    
    return config;
  }

  private setupErrorHandling(): void {
    this.queue.on('failed', (operation: GitOperation) => {
      this.logger.error('Git operation failed', {
        operationId: operation.id,
        type: operation.type,
        error: operation.error,
      });
    });

    this.queue.on('completed', (operation: GitOperation) => {
      this.logger.debug('Git operation completed', {
        operationId: operation.id,
        type: operation.type,
        duration: operation.endTime && operation.startTime 
          ? operation.endTime.getTime() - operation.startTime.getTime()
          : undefined,
      });
    });
  }

  private createDefaultLogger(): GitLogger {
    return {
      info: (message: string, meta?: any) => console.log('[GitService]', message, meta),
      warn: (message: string, meta?: any) => console.warn('[GitService]', message, meta),
      error: (message: string, meta?: any) => console.error('[GitService]', message, meta),
      debug: (message: string, meta?: any) => console.debug('[GitService]', message, meta),
    };
  }

  // ============================================================================
  // Parsing Helpers
  // ============================================================================

  private async getWorktreeInfo(worktreePath: string): Promise<WorktreeInfo> {
    try {
      const git = simpleGit(worktreePath);
      const status = await git.status();
      const branch = await git.revparse(['--abbrev-ref', 'HEAD']);
      const commit = await git.revparse(['HEAD']);
      
      return {
        path: worktreePath,
        branch: branch.trim(),
        commit: commit.trim(),
        isDetached: status.detached,
        isBare: false, // Worktrees are never bare
      };
    } catch (error) {
      throw new Error(`Failed to get worktree info: ${error instanceof Error ? error.message : error}`);
    }
  }

  private parseWorktreeList(output: string): WorktreeInfo[] {
    const worktrees: WorktreeInfo[] = [];
    const lines = output.split('\n').filter(line => line.trim());
    
    let currentWorktree: Partial<WorktreeInfo> = {};
    
    for (const line of lines) {
      if (line.startsWith('worktree ')) {
        if (currentWorktree.path) {
          worktrees.push(currentWorktree as WorktreeInfo);
        }
        currentWorktree = { path: line.substring(9) };
      } else if (line.startsWith('HEAD ')) {
        currentWorktree.commit = line.substring(4);
      } else if (line.startsWith('branch ')) {
        currentWorktree.branch = line.substring(7);
        currentWorktree.isDetached = false;
      } else if (line === 'detached') {
        currentWorktree.isDetached = true;
        currentWorktree.branch = 'HEAD';
      } else if (line === 'bare') {
        currentWorktree.isBare = true;
      } else if (line.startsWith('prunable ')) {
        currentWorktree.isPrunable = true;
        currentWorktree.reason = line.substring(9);
      }
    }
    
    if (currentWorktree.path) {
      worktrees.push(currentWorktree as WorktreeInfo);
    }
    
    return worktrees;
  }

  private parsePruneOutput(output: string): string[] {
    return output
      .split('\n')
      .filter(line => line.includes('Removing worktrees'))
      .map(line => {
        const match = line.match(/Removing worktrees\/(.+):/);
        return match ? match[1] : '';
      })
      .filter(path => path.length > 0);
  }

  private parseBranchSummary(summary: BranchSummary): BranchInfo[] {
    const branches: BranchInfo[] = [];
    
    Object.entries(summary.branches).forEach(([name, info]) => {
      branches.push({
        name,
        current: name === summary.current,
        commit: info.commit,
        label: info.label,
        upstream: info.upstream,
      });
    });
    
    return branches;
  }

  private parseStatusResult(status: StatusResult): FileStatus[] {
    const files: FileStatus[] = [];
    
    // Helper function to create file status
    const createFileStatus = (path: string, index: string, workingDir: string): FileStatus => ({
      path,
      index,
      working_dir: workingDir,
      staged: index !== ' ',
      modified: workingDir === 'M' || index === 'M',
      created: workingDir === 'A' || index === 'A',
      deleted: workingDir === 'D' || index === 'D',
      renamed: workingDir === 'R' || index === 'R',
      conflicted: index === 'U' || workingDir === 'U',
    });
    
    status.staged.forEach(file => {
      files.push(createFileStatus(file, 'A', ' '));
    });
    
    status.modified.forEach(file => {
      files.push(createFileStatus(file, ' ', 'M'));
    });
    
    status.created.forEach(file => {
      files.push(createFileStatus(file, ' ', 'A'));
    });
    
    status.deleted.forEach(file => {
      files.push(createFileStatus(file, ' ', 'D'));
    });
    
    status.renamed.forEach(file => {
      files.push(createFileStatus(file.to || file as any, 'R', ' '));
    });
    
    status.conflicted.forEach(file => {
      files.push(createFileStatus(file, 'U', 'U'));
    });
    
    return files;
  }

  private async parseConflicts(conflictedFiles: string[]): Promise<MergeConflict[]> {
    const conflicts: MergeConflict[] = [];
    
    for (const file of conflictedFiles) {
      try {
        const content = await fs.readFile(path.join(this.config.baseDir, file), 'utf8');
        const sections = this.parseConflictSections(content);
        
        conflicts.push({
          file,
          reason: 'merge_conflict',
          content,
          sections,
        });
      } catch (error) {
        conflicts.push({
          file,
          reason: 'file_read_error',
        });
      }
    }
    
    return conflicts;
  }

  private parseConflictSections(content: string): ConflictSection[] {
    const sections: ConflictSection[] = [];
    const lines = content.split('\n');
    let inConflict = false;
    let currentSection: Partial<ConflictSection> | null = null;
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      
      if (line.startsWith('<<<<<<<')) {
        inConflict = true;
        currentSection = {
          type: 'ours',
          startLine: i + 1,
          content: '',
        };
      } else if (line.startsWith('=======') && inConflict) {
        if (currentSection) {
          currentSection.endLine = i;
          sections.push(currentSection as ConflictSection);
        }
        currentSection = {
          type: 'theirs',
          startLine: i + 1,
          content: '',
        };
      } else if (line.startsWith('>>>>>>>') && inConflict) {
        if (currentSection) {
          currentSection.endLine = i;
          sections.push(currentSection as ConflictSection);
        }
        inConflict = false;
        currentSection = null;
      } else if (inConflict && currentSection) {
        currentSection.content += line + '\n';
      }
    }
    
    return sections;
  }

  private parseDiffOutput(diffOutput: string): DiffChunk[] {
    const chunks: DiffChunk[] = [];
    const lines = diffOutput.split('\n');
    let currentChunk: Partial<DiffChunk> | null = null;
    
    for (const line of lines) {
      if (line.startsWith('@@')) {
        if (currentChunk) {
          chunks.push(currentChunk as DiffChunk);
        }
        
        const match = line.match(/@@ -(\d+),?(\d+)? \+(\d+),?(\d+)? @@(.*)/);
        if (match) {
          currentChunk = {
            oldStart: parseInt(match[1], 10),
            oldLines: parseInt(match[2] || '1', 10),
            newStart: parseInt(match[3], 10),
            newLines: parseInt(match[4] || '1', 10),
            header: match[5]?.trim() || '',
            lines: [],
          };
        }
      } else if (currentChunk && (line.startsWith('+') || line.startsWith('-') || line.startsWith(' '))) {
        const type = line.startsWith('+') ? 'add' : line.startsWith('-') ? 'delete' : 'context';
        const content = line.substring(1);
        
        currentChunk.lines!.push({
          type,
          content,
        });
      }
    }
    
    if (currentChunk) {
      chunks.push(currentChunk as DiffChunk);
    }
    
    return chunks;
  }

  private parseLogResult(log: LogResult): CommitInfo[] {
    return log.all.map(commit => ({
      hash: commit.hash,
      message: commit.message,
      author: {
        name: commit.author_name,
        email: commit.author_email,
        date: new Date(commit.date),
      },
      committer: {
        name: commit.author_name, // simple-git doesn't separate committer
        email: commit.author_email,
        date: new Date(commit.date),
      },
      refs: commit.refs ? commit.refs.split(', ') : undefined,
      body: commit.body,
    }));
  }
}