import { EventEmitter } from 'events';
import { GitOperation, GitOperationType, GitLogger, LockInfo, RetryOptions } from './types.js';

export interface QueueOptions {
  maxConcurrentOps?: number;
  defaultTimeoutMs?: number;
  lockTimeoutMs?: number;
  logger?: GitLogger;
}

export interface QueuedOperation {
  operation: GitOperation;
  execute: () => Promise<any>;
  resolve: (value: any) => void;
  reject: (error: any) => void;
  timeoutId?: NodeJS.Timeout;
  retryOptions?: RetryOptions;
  currentAttempt?: number;
}

export class GitOperationQueue extends EventEmitter {
  private readonly maxConcurrentOps: number;
  private readonly defaultTimeoutMs: number;
  private readonly lockTimeoutMs: number;
  private readonly logger?: GitLogger;
  
  private readonly queue: QueuedOperation[] = [];
  private readonly running: Map<string, QueuedOperation> = new Map();
  private readonly locks: Map<string, LockInfo> = new Map();
  
  private isProcessing = false;

  constructor(options: QueueOptions = {}) {
    super();
    this.maxConcurrentOps = options.maxConcurrentOps || 3;
    this.defaultTimeoutMs = options.defaultTimeoutMs || 300000; // 5 minutes
    this.lockTimeoutMs = options.lockTimeoutMs || 60000; // 1 minute
    this.logger = options.logger;
  }

  /**
   * Queues a git operation for execution
   */
  async enqueue<T>(
    operation: GitOperation,
    executor: () => Promise<T>,
    timeoutMs?: number,
    retryOptions?: RetryOptions,
  ): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      const queuedOp: QueuedOperation = {
        operation,
        execute: executor,
        resolve: resolve as (value: any) => void,
        reject,
        retryOptions,
        currentAttempt: 1,
      };

      // Set timeout if specified
      const timeout = timeoutMs || this.defaultTimeoutMs;
      if (timeout > 0) {
        queuedOp.timeoutId = setTimeout(() => {
          this.handleTimeout(queuedOp);
        }, timeout);
      }

      this.queue.push(queuedOp);
      
      this.logger?.debug('Operation queued', {
        operationId: operation.id,
        type: operation.type,
        queueLength: this.queue.length,
      });

      this.emit('queued', operation);
      this.processQueue();
    });
  }

  /**
   * Acquires a lock for exclusive operations
   */
  acquireLock(operationId: string, type: GitOperationType, resource?: string): boolean {
    const lockKey = resource || 'global';
    const existingLock = this.locks.get(lockKey);
    
    // Check if lock exists and hasn't expired
    if (existingLock && existingLock.expiresAt > new Date()) {
      this.logger?.debug('Lock acquisition failed - already locked', {
        operationId,
        lockKey,
        existingOperationId: existingLock.operationId,
      });
      return false;
    }

    // Acquire lock
    const lockInfo: LockInfo = {
      operationId,
      type,
      acquiredAt: new Date(),
      expiresAt: new Date(Date.now() + this.lockTimeoutMs),
    };

    this.locks.set(lockKey, lockInfo);
    
    this.logger?.debug('Lock acquired', {
      operationId,
      lockKey,
      expiresAt: lockInfo.expiresAt,
    });

    return true;
  }

  /**
   * Releases a lock
   */
  releaseLock(operationId: string, resource?: string): void {
    const lockKey = resource || 'global';
    const lock = this.locks.get(lockKey);
    
    if (lock && lock.operationId === operationId) {
      this.locks.delete(lockKey);
      this.logger?.debug('Lock released', { operationId, lockKey });
      
      // Process queue in case operations were waiting for this lock
      this.processQueue();
    }
  }

  /**
   * Cancels a queued or running operation
   */
  cancel(operationId: string): boolean {
    // Check running operations
    const runningOp = this.running.get(operationId);
    if (runningOp) {
      runningOp.operation.status = 'cancelled';
      if (runningOp.timeoutId) {
        clearTimeout(runningOp.timeoutId);
      }
      runningOp.reject(new Error('Operation cancelled'));
      this.running.delete(operationId);
      this.emit('cancelled', runningOp.operation);
      this.logger?.info('Running operation cancelled', { operationId });
      this.processQueue();
      return true;
    }

    // Check queued operations
    const queueIndex = this.queue.findIndex(op => op.operation.id === operationId);
    if (queueIndex >= 0) {
      const queuedOp = this.queue.splice(queueIndex, 1)[0];
      queuedOp.operation.status = 'cancelled';
      if (queuedOp.timeoutId) {
        clearTimeout(queuedOp.timeoutId);
      }
      queuedOp.reject(new Error('Operation cancelled'));
      this.emit('cancelled', queuedOp.operation);
      this.logger?.info('Queued operation cancelled', { operationId });
      return true;
    }

    return false;
  }

  /**
   * Gets current queue status
   */
  getStatus(): {
    queued: number;
    running: number;
    locks: number;
    operations: GitOperation[];
  } {
    const operations = [
      ...this.queue.map(op => op.operation),
      ...Array.from(this.running.values()).map(op => op.operation),
    ];

    return {
      queued: this.queue.length,
      running: this.running.size,
      locks: this.locks.size,
      operations,
    };
  }

  /**
   * Clears all queued operations
   */
  clear(): void {
    // Cancel all queued operations
    while (this.queue.length > 0) {
      const op = this.queue.shift()!;
      op.operation.status = 'cancelled';
      if (op.timeoutId) {
        clearTimeout(op.timeoutId);
      }
      op.reject(new Error('Queue cleared'));
    }

    this.logger?.info('Queue cleared');
  }

  /**
   * Processes the queue
   */
  private async processQueue(): Promise<void> {
    if (this.isProcessing) {
      return;
    }

    this.isProcessing = true;

    try {
      while (
        this.queue.length > 0 && 
        this.running.size < this.maxConcurrentOps
      ) {
        const queuedOp = this.queue.shift()!;
        
        // Check if operation requires a lock
        if (this.requiresLock(queuedOp.operation.type)) {
          if (!this.acquireLock(queuedOp.operation.id, queuedOp.operation.type)) {
            // Put back in queue and try later
            this.queue.unshift(queuedOp);
            break;
          }
        }

        // Start the operation
        this.executeOperation(queuedOp);
      }
    } finally {
      this.isProcessing = false;
    }
  }

  /**
   * Executes a queued operation
   */
  private async executeOperation(queuedOp: QueuedOperation): Promise<void> {
    const { operation } = queuedOp;
    
    operation.status = 'running';
    operation.startTime = new Date();
    this.running.set(operation.id, queuedOp);
    
    this.emit('started', operation);
    this.logger?.info('Operation started', {
      operationId: operation.id,
      type: operation.type,
    });

    try {
      const result = await queuedOp.execute();
      
      operation.status = 'completed';
      operation.endTime = new Date();
      operation.result = result;
      
      if (queuedOp.timeoutId) {
        clearTimeout(queuedOp.timeoutId);
      }
      
      queuedOp.resolve(result);
      this.emit('completed', operation);
      
      this.logger?.info('Operation completed', {
        operationId: operation.id,
        type: operation.type,
        duration: operation.endTime.getTime() - operation.startTime!.getTime(),
      });
    } catch (error) {
      await this.handleOperationError(queuedOp, error);
    } finally {
      this.running.delete(operation.id);
      this.releaseLock(operation.id);
      this.processQueue();
    }
  }

  /**
   * Handles operation errors with retry logic
   */
  private async handleOperationError(queuedOp: QueuedOperation, error: any): Promise<void> {
    const { operation, retryOptions } = queuedOp;
    const currentAttempt = queuedOp.currentAttempt || 1;

    this.logger?.warn('Operation failed', {
      operationId: operation.id,
      type: operation.type,
      attempt: currentAttempt,
      error: error instanceof Error ? error.message : String(error),
    });

    // Check if we should retry
    if (retryOptions && currentAttempt < retryOptions.attempts) {
      const delay = this.calculateRetryDelay(retryOptions, currentAttempt);
      
      this.logger?.info('Retrying operation', {
        operationId: operation.id,
        attempt: currentAttempt + 1,
        delay,
      });

      // Wait before retry
      await new Promise(resolve => setTimeout(resolve, delay));
      
      // Update attempt count and re-queue
      queuedOp.currentAttempt = currentAttempt + 1;
      this.queue.unshift(queuedOp);
      return;
    }

    // No more retries, fail the operation
    operation.status = 'failed';
    operation.endTime = new Date();
    operation.error = error instanceof Error ? error.message : String(error);
    
    if (queuedOp.timeoutId) {
      clearTimeout(queuedOp.timeoutId);
    }
    
    queuedOp.reject(error);
    this.emit('failed', operation);
  }

  /**
   * Handles operation timeout
   */
  private handleTimeout(queuedOp: QueuedOperation): void {
    const { operation } = queuedOp;
    
    this.logger?.warn('Operation timed out', {
      operationId: operation.id,
      type: operation.type,
    });

    this.cancel(operation.id);
  }

  /**
   * Calculates retry delay based on retry options
   */
  private calculateRetryDelay(retryOptions: RetryOptions, attempt: number): number {
    let delay = retryOptions.delay;
    
    if (retryOptions.backoff === 'exponential') {
      delay = retryOptions.delay * Math.pow(2, attempt - 1);
    } else if (retryOptions.backoff === 'linear') {
      delay = retryOptions.delay * attempt;
    }
    
    if (retryOptions.maxDelay) {
      delay = Math.min(delay, retryOptions.maxDelay);
    }
    
    return delay;
  }

  /**
   * Determines if an operation type requires exclusive locking
   */
  private requiresLock(type: GitOperationType): boolean {
    return [
      'merge',
      'worktree-add',
      'worktree-remove',
      'worktree-prune',
      'push',
      'commit',
    ].includes(type);
  }
}