import { EventEmitter } from 'events';
import chokidar, { FSWatcher } from 'chokidar';
import { PathSecurity } from './security.js';
import { GitIgnoreHandler } from './gitignore.js';
import type { WatchOptions, WatchEvent } from './types.js';

/**
 * Debounced event aggregator for file system events
 */
class EventDebouncer {
  private readonly timers = new Map<string, NodeJS.Timeout>();
  private readonly pendingEvents = new Map<string, WatchEvent>();

  constructor(
    private readonly debounceMs: number,
    private readonly emitCallback: (event: WatchEvent) => void,
  ) {}

  /**
   * Adds an event to be debounced
   */
  addEvent(event: WatchEvent): void {
    const key = `${event.type}:${event.path}`;
    
    // Clear existing timer for this path
    const existingTimer = this.timers.get(key);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    // Update pending event
    this.pendingEvents.set(key, event);

    // Set new timer
    const timer = setTimeout(() => {
      const pendingEvent = this.pendingEvents.get(key);
      if (pendingEvent) {
        this.emitCallback(pendingEvent);
        this.pendingEvents.delete(key);
      }
      this.timers.delete(key);
    }, this.debounceMs);

    this.timers.set(key, timer);
  }

  /**
   * Clears all pending events
   */
  clear(): void {
    for (const timer of this.timers.values()) {
      clearTimeout(timer);
    }
    this.timers.clear();
    this.pendingEvents.clear();
  }

  /**
   * Gets the number of pending events
   */
  getPendingCount(): number {
    return this.pendingEvents.size;
  }
}

/**
 * File system watcher with debouncing and .gitignore support
 */
export class FileSystemWatcher extends EventEmitter {
  private readonly pathSecurity: PathSecurity;
  private readonly gitIgnore: GitIgnoreHandler;
  private readonly watchers = new Map<string, FSWatcher>();
  private readonly debouncers = new Map<string, EventDebouncer>();
  private isWatching = false;

  constructor(rootDirectory: string) {
    super();
    this.pathSecurity = new PathSecurity(rootDirectory);
    this.gitIgnore = new GitIgnoreHandler(rootDirectory);
  }

  /**
   * Starts watching a path
   */
  async watch(
    watchPath: string,
    options: WatchOptions = {},
  ): Promise<void> {
    const validatedPath = this.pathSecurity.validatePath(watchPath);
    const watchKey = validatedPath;

    // Don't start multiple watchers for the same path
    if (this.watchers.has(watchKey)) {
      return;
    }

    const {
      recursive = true,
      ignored = [],
      debounceMs = 300,
      ignoreInitial = true,
    } = options;

    // Create debouncer for this watch path
    const debouncer = new EventDebouncer(
      debounceMs,
      (event: WatchEvent) => this.emit('change', event),
    );
    this.debouncers.set(watchKey, debouncer);

    // Prepare ignored patterns
    const ignoredPatterns = Array.isArray(ignored) ? ignored : [ignored];
    
    // Create chokidar watcher options
    const chokidarOptions = {
      ignored: (path: string) => this.shouldIgnoreSync(path, ignoredPatterns),
      persistent: true,
      ignoreInitial,
      followSymlinks: false, // Security: don't follow symlinks
      atomic: true, // Wait for write operations to complete
      usePolling: false, // Use native events when possible
      interval: 1000, // Polling interval if usePolling is true
      binaryInterval: 300, // Binary file polling interval
      awaitWriteFinish: {
        stabilityThreshold: 100,
        pollInterval: 100,
      },
    };

    // Create watcher
    const watcher = chokidar.watch(validatedPath, chokidarOptions);

    // Set up event handlers
    this.setupWatcherEvents(watcher, debouncer);

    // Store watcher
    this.watchers.set(watchKey, watcher);
    this.isWatching = true;

    // Wait for watcher to be ready
    return new Promise((resolve, reject) => {
      watcher.on('ready', resolve);
      watcher.on('error', reject);
    });
  }

  /**
   * Stops watching a specific path
   */
  async unwatch(watchPath: string): Promise<void> {
    const validatedPath = this.pathSecurity.validatePath(watchPath);
    const watchKey = validatedPath;

    const watcher = this.watchers.get(watchKey);
    if (watcher) {
      await watcher.close();
      this.watchers.delete(watchKey);
    }

    const debouncer = this.debouncers.get(watchKey);
    if (debouncer) {
      debouncer.clear();
      this.debouncers.delete(watchKey);
    }

    if (this.watchers.size === 0) {
      this.isWatching = false;
    }
  }

  /**
   * Stops all watchers
   */
  async unwatchAll(): Promise<void> {
    const closePromises = Array.from(this.watchers.values()).map(w => w.close());
    await Promise.all(closePromises);
    
    this.watchers.clear();
    
    for (const debouncer of this.debouncers.values()) {
      debouncer.clear();
    }
    this.debouncers.clear();
    
    this.isWatching = false;
  }

  /**
   * Checks if currently watching any paths
   */
  isActive(): boolean {
    return this.isWatching && this.watchers.size > 0;
  }

  /**
   * Gets list of watched paths
   */
  getWatchedPaths(): string[] {
    return Array.from(this.watchers.keys());
  }

  /**
   * Gets watcher statistics
   */
  getStats(): { 
    watchedPaths: number; 
    pendingEvents: number; 
    cacheStats: { ignoreEntries: number; fileEntries: number } 
  } {
    const pendingEvents = Array.from(this.debouncers.values())
      .reduce((sum, debouncer) => sum + debouncer.getPendingCount(), 0);

    return {
      watchedPaths: this.watchers.size,
      pendingEvents,
      cacheStats: this.gitIgnore.getCacheStats(),
    };
  }

  /**
   * Sets up event handlers for a chokidar watcher
   */
  private setupWatcherEvents(watcher: FSWatcher, debouncer: EventDebouncer): void {
    const createEvent = (type: WatchEvent['type'], path: string, stats?: any): WatchEvent => ({
      type,
      path: this.pathSecurity.getRelativePath(path),
      stats,
      timestamp: new Date(),
    });

    watcher.on('add', (path, stats) => {
      debouncer.addEvent(createEvent('add', path, stats));
    });

    watcher.on('change', (path, stats) => {
      debouncer.addEvent(createEvent('change', path, stats));
    });

    watcher.on('unlink', (path) => {
      debouncer.addEvent(createEvent('unlink', path));
    });

    watcher.on('addDir', (path, stats) => {
      debouncer.addEvent(createEvent('addDir', path, stats));
    });

    watcher.on('unlinkDir', (path) => {
      debouncer.addEvent(createEvent('unlinkDir', path));
    });

    watcher.on('error', (error) => {
      this.emit('error', error);
    });
  }

  /**
   * Determines if a path should be ignored (synchronous version for chokidar)
   */
  private shouldIgnoreSync(
    filePath: string, 
    additionalPatterns: (string | RegExp | ((path: string) => boolean))[],
  ): boolean {
    try {
      // Check if path is within our root directory
      if (!this.pathSecurity.isWithinRoot(filePath)) {
        return true;
      }

      // Check additional patterns
      for (const pattern of additionalPatterns) {
        if (typeof pattern === 'string') {
          if (filePath.includes(pattern)) {
            return true;
          }
        } else if (pattern instanceof RegExp) {
          if (pattern.test(filePath)) {
            return true;
          }
        } else if (typeof pattern === 'function') {
          if (pattern(filePath)) {
            return true;
          }
        }
      }

      // For basic patterns, do simple checks
      const relativePath = this.pathSecurity.getRelativePath(filePath);
      const basicIgnorePatterns = [
        'node_modules',
        '.git',
        '.DS_Store',
        'Thumbs.db',
        '.tmp',
        '.temp',
      ];

      return basicIgnorePatterns.some(pattern => relativePath.includes(pattern));
    } catch (error) {
      // If we can't determine, err on the side of caution and ignore
      return true;
    }
  }

  /**
   * Determines if a path should be ignored (async version for full .gitignore support)
   */
  private async shouldIgnore(
    filePath: string, 
    additionalPatterns: (string | RegExp | ((path: string) => boolean))[],
  ): Promise<boolean> {
    try {
      // First check sync patterns
      if (this.shouldIgnoreSync(filePath, additionalPatterns)) {
        return true;
      }

      // Check .gitignore rules
      return await this.gitIgnore.isIgnored(filePath);
    } catch (error) {
      // If we can't determine, err on the side of caution and ignore
      console.warn(`Warning: Could not determine ignore status for ${filePath}:`, error);
      return true;
    }
  }
}

/**
 * Factory function to create a file system watcher
 */
export function createFileSystemWatcher(rootDirectory: string): FileSystemWatcher {
  return new FileSystemWatcher(rootDirectory);
}