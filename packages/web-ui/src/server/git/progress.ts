import { EventEmitter } from 'events';
import { GitProgress, ProgressCallback, GitLogger } from './types.js';

export interface ProgressOptions {
  method: string;
  repository?: string;
  updateIntervalMs?: number;
  logger?: GitLogger;
}

export class GitProgressTracker extends EventEmitter {
  private readonly method: string;
  private readonly repository?: string;
  private readonly updateIntervalMs: number;
  private readonly logger?: GitLogger;
  private readonly callbacks: Set<ProgressCallback> = new Set();
  
  private currentProgress: GitProgress;
  private lastUpdateTime = 0;

  constructor(options: ProgressOptions) {
    super();
    this.method = options.method;
    this.repository = options.repository;
    this.updateIntervalMs = options.updateIntervalMs || 100; // 10 FPS
    this.logger = options.logger;
    
    this.currentProgress = {
      stage: 'initializing',
      progress: 0,
      method: this.method,
      repository: this.repository,
      remoteMessages: [],
    };
  }

  /**
   * Updates progress and notifies callbacks
   */
  updateProgress(update: Partial<GitProgress>): void {
    const now = Date.now();
    
    // Throttle updates to avoid overwhelming callbacks
    if (now - this.lastUpdateTime < this.updateIntervalMs) {
      return;
    }
    
    this.currentProgress = {
      ...this.currentProgress,
      ...update,
    };
    
    this.lastUpdateTime = now;
    
    // Notify all callbacks
    this.callbacks.forEach(callback => {
      try {
        callback(this.currentProgress);
      } catch (error) {
        this.logger?.error('Progress callback error', {
          error: error instanceof Error ? error.message : String(error),
        });
      }
    });
    
    // Emit event for EventEmitter interface
    this.emit('progress', this.currentProgress);
    
    this.logger?.debug('Progress updated', {
      method: this.method,
      stage: this.currentProgress.stage,
      progress: this.currentProgress.progress,
      total: this.currentProgress.total,
    });
  }

  /**
   * Registers a progress callback
   */
  onProgress(callback: ProgressCallback): () => void {
    this.callbacks.add(callback);
    
    // Return unsubscribe function
    return () => {
      this.callbacks.delete(callback);
    };
  }

  /**
   * Marks operation as completed
   */
  complete(): void {
    this.updateProgress({
      stage: 'completed',
      progress: 100,
      total: 100,
    });
    
    this.emit('complete', this.currentProgress);
    this.logger?.info('Git operation completed', {
      method: this.method,
      repository: this.repository,
    });
  }

  /**
   * Marks operation as failed
   */
  fail(error: Error | string): void {
    const errorMessage = error instanceof Error ? error.message : error;
    
    this.updateProgress({
      stage: 'failed',
    });
    
    this.emit('error', errorMessage);
    this.logger?.error('Git operation failed', {
      method: this.method,
      repository: this.repository,
      error: errorMessage,
    });
  }

  /**
   * Gets current progress
   */
  getProgress(): GitProgress {
    return { ...this.currentProgress };
  }

  /**
   * Creates a progress handler for simple-git operations
   */
  createSimpleGitHandler(): (method: string, stage: string, progress: number) => void {
    return (method: string, stage: string, progress: number) => {
      this.updateProgress({
        stage,
        progress,
        method,
      });
    };
  }

  /**
   * Creates a progress handler for clone operations
   */
  createCloneHandler(): {
    progress: (method: string, stage: string, progress: number) => void;
  } {
    return {
      progress: (method: string, stage: string, progress: number) => {
        // Parse clone-specific progress information
        let total: number | undefined;
        let normalizedProgress = progress;
        
        // Clone operations often report progress > 100, normalize it
        if (stage.includes('Receiving objects') || stage.includes('Resolving deltas')) {
          const match = stage.match(/(\d+)\/(\d+)/);
          if (match) {
            const current = parseInt(match[1], 10);
            total = parseInt(match[2], 10);
            normalizedProgress = Math.round((current / total) * 100);
          }
        }
        
        this.updateProgress({
          stage: this.parseCloneStage(stage),
          progress: Math.min(normalizedProgress, 100),
          total,
          method,
        });
      },
    };
  }

  /**
   * Creates a progress handler for push/pull operations
   */
  createTransferHandler(): {
    progress: (method: string, stage: string, progress: number) => void;
  } {
    return {
      progress: (method: string, stage: string, progress: number) => {
        this.updateProgress({
          stage: this.parseTransferStage(stage),
          progress: Math.min(progress, 100),
          method,
        });
      },
    };
  }

  /**
   * Parses clone stage messages to user-friendly format
   */
  private parseCloneStage(stage: string): string {
    if (stage.includes('Cloning into')) return 'initializing';
    if (stage.includes('remote: Enumerating')) return 'enumerating_objects';
    if (stage.includes('remote: Counting')) return 'counting_objects';
    if (stage.includes('remote: Compressing')) return 'compressing_objects';
    if (stage.includes('Receiving objects')) return 'receiving_objects';
    if (stage.includes('Resolving deltas')) return 'resolving_deltas';
    if (stage.includes('Checking out files')) return 'checking_out';
    return 'processing';
  }

  /**
   * Parses transfer stage messages to user-friendly format
   */
  private parseTransferStage(stage: string): string {
    if (stage.includes('Enumerating')) return 'enumerating_objects';
    if (stage.includes('Counting')) return 'counting_objects';
    if (stage.includes('Compressing')) return 'compressing_objects';
    if (stage.includes('Writing objects')) return 'writing_objects';
    if (stage.includes('Resolving deltas')) return 'resolving_deltas';
    if (stage.includes('Updating')) return 'updating_refs';
    return 'transferring';
  }

  /**
   * Creates a combined progress tracker for complex operations
   */
  static createCombined(
    trackers: GitProgressTracker[],
    weights?: number[],
  ): GitProgressTracker {
    const combined = new GitProgressTracker({
      method: 'combined',
      repository: trackers[0]?.repository,
    });

    const normalizedWeights = weights || trackers.map(() => 1 / trackers.length);
    const totalWeight = normalizedWeights.reduce((sum, weight) => sum + weight, 0);

    const updateCombinedProgress = () => {
      let totalProgress = 0;
      let allCompleted = true;
      let anyFailed = false;

      trackers.forEach((tracker, index) => {
        const progress = tracker.getProgress();
        const weight = normalizedWeights[index] / totalWeight;
        
        totalProgress += progress.progress * weight;
        
        if (progress.stage !== 'completed') {
          allCompleted = false;
        }
        
        if (progress.stage === 'failed') {
          anyFailed = true;
        }
      });

      if (anyFailed) {
        combined.fail('One or more operations failed');
      } else if (allCompleted) {
        combined.complete();
      } else {
        combined.updateProgress({
          stage: 'processing',
          progress: Math.round(totalProgress),
        });
      }
    };

    // Subscribe to all trackers
    trackers.forEach(tracker => {
      tracker.on('progress', updateCombinedProgress);
      tracker.on('complete', updateCombinedProgress);
      tracker.on('error', updateCombinedProgress);
    });

    return combined;
  }
}