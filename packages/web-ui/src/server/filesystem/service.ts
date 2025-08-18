import { promises as fs, constants as fsConstants } from 'fs';
import { createReadStream, createWriteStream } from 'fs';
import path from 'path';
import { Readable, Writable, pipeline } from 'stream';
import { promisify } from 'util';
import * as mimeTypes from 'mime-types';

import { PathSecurity, FileSizeValidator, OperationLimiter } from './security.js';
import { GitIgnoreHandler } from './gitignore.js';
import { FileSystemWatcher } from './watcher.js';
import type {
  FileSystemOptions,
  FileMetadata,
  DirectoryEntry,
  ReadStreamOptions,
  WriteStreamOptions,
  AtomicWriteOptions,
  CopyOptions,
  WatchOptions,
  ProgressInfo,
  FileSystemError,
  SecurityError,
  FileSizeError,
  PermissionError,
} from './types.js';

const pipelineAsync = promisify(pipeline);

/**
 * Secure file system service with comprehensive safety features
 */
export class FileSystemService {
  private readonly pathSecurity: PathSecurity;
  private readonly fileSizeValidator: FileSizeValidator;
  private readonly operationLimiter: OperationLimiter;
  private readonly gitIgnore: GitIgnoreHandler;
  private readonly watcher: FileSystemWatcher;
  private readonly tempDirectory: string;
  private readonly options: Required<FileSystemOptions>;

  constructor(options: FileSystemOptions) {
    this.options = {
      maxFileSize: 100 * 1024 * 1024, // 100MB default
      followSymlinks: false,
      tempDirectory: path.join(options.rootDirectory, '.tmp'),
      watchDebounceMs: 300,
      maxConcurrentOps: 10,
      ...options,
    };

    this.pathSecurity = new PathSecurity(this.options.rootDirectory);
    this.fileSizeValidator = new FileSizeValidator(this.options.maxFileSize);
    this.operationLimiter = new OperationLimiter(this.options.maxConcurrentOps);
    this.gitIgnore = new GitIgnoreHandler(this.options.rootDirectory);
    this.watcher = new FileSystemWatcher(this.options.rootDirectory);
    this.tempDirectory = this.pathSecurity.validatePath(this.options.tempDirectory);

    this.ensureTempDirectory();
  }

  // ========== FILE READING OPERATIONS ==========

  /**
   * Reads a file as text with encoding
   */
  async readFile(filePath: string, encoding: BufferEncoding = 'utf8'): Promise<string> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'read');
      
      // Check file size before reading
      const stats = await fs.stat(validatedPath);
      this.fileSizeValidator.validateSize(stats.size, filePath);
      
      return await fs.readFile(validatedPath, encoding);
    } finally {
      release();
    }
  }

  /**
   * Reads a file as a buffer
   */
  async readFileBuffer(filePath: string): Promise<Buffer> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'read');
      
      // Check file size before reading
      const stats = await fs.stat(validatedPath);
      this.fileSizeValidator.validateSize(stats.size, filePath);
      
      return await fs.readFile(validatedPath);
    } finally {
      release();
    }
  }

  /**
   * Creates a readable stream for a file
   */
  async createReadStream(
    filePath: string,
    options: ReadStreamOptions = {},
  ): Promise<Readable> {
    const validatedPath = await this.validateFileAccess(filePath, 'read');
    
    // Check file size
    const stats = await fs.stat(validatedPath);
    this.fileSizeValidator.validateSize(stats.size, filePath);
    
    const streamOptions: any = {
      encoding: options.encoding,
      start: options.start,
      end: options.end,
      highWaterMark: options.bufferSize || 64 * 1024, // 64KB default buffer
    };

    return createReadStream(validatedPath, streamOptions);
  }

  /**
   * Reads a file in chunks with progress callback
   */
  async readFileChunked(
    filePath: string,
    chunkSize = 64 * 1024,
    onProgress?: (progress: ProgressInfo) => void,
  ): Promise<Buffer[]> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'read');
      const stats = await fs.stat(validatedPath);
      const totalBytes = stats.size;
      
      this.fileSizeValidator.validateSize(totalBytes, filePath);
      
      const chunks: Buffer[] = [];
      let bytesRead = 0;
      const startTime = Date.now();
      
      const stream = createReadStream(validatedPath, { highWaterMark: chunkSize });
      
      return new Promise((resolve, reject) => {
        stream.on('data', (chunk: Buffer) => {
          chunks.push(chunk);
          bytesRead += chunk.length;
          
          if (onProgress) {
            const elapsed = Date.now() - startTime;
            const percentage = (bytesRead / totalBytes) * 100;
            const estimatedTimeMs = elapsed * (totalBytes / bytesRead) - elapsed;
            
            onProgress({
              bytesProcessed: bytesRead,
              totalBytes,
              percentage,
              estimatedTimeMs,
            });
          }
        });
        
        stream.on('end', () => resolve(chunks));
        stream.on('error', reject);
      });
    } finally {
      release();
    }
  }

  // ========== FILE WRITING OPERATIONS ==========

  /**
   * Writes text to a file
   */
  async writeFile(
    filePath: string,
    content: string,
    options: WriteStreamOptions = {},
  ): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'write');
      
      // Validate content size
      const contentBuffer = Buffer.from(content, options.encoding || 'utf8');
      this.fileSizeValidator.validateSize(contentBuffer.length, filePath);
      
      const writeOptions: any = {
        encoding: options.encoding || 'utf8',
        mode: options.mode,
      };
      
      if (options.append) {
        await fs.appendFile(validatedPath, content, writeOptions);
      } else {
        await fs.writeFile(validatedPath, content, writeOptions);
      }
    } finally {
      release();
    }
  }

  /**
   * Writes a buffer to a file
   */
  async writeFileBuffer(
    filePath: string,
    buffer: Buffer,
    options: WriteStreamOptions = {},
  ): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'write');
      
      // Validate buffer size
      this.fileSizeValidator.validateSize(buffer.length, filePath);
      
      const writeOptions: any = {
        mode: options.mode,
      };
      
      if (options.append) {
        await fs.appendFile(validatedPath, buffer, writeOptions);
      } else {
        await fs.writeFile(validatedPath, buffer, writeOptions);
      }
    } finally {
      release();
    }
  }

  /**
   * Creates a writable stream for a file
   */
  async createWriteStream(
    filePath: string,
    options: WriteStreamOptions = {},
  ): Promise<Writable> {
    const validatedPath = await this.validateFileAccess(filePath, 'write');
    
    const streamOptions: any = {
      encoding: options.encoding,
      mode: options.mode,
      flags: options.append ? 'a' : 'w',
      highWaterMark: options.bufferSize || 64 * 1024, // 64KB default buffer
    };

    return createWriteStream(validatedPath, streamOptions);
  }

  /**
   * Atomic write operation using temporary file
   */
  async writeFileAtomic(
    filePath: string,
    content: string | Buffer,
    options: AtomicWriteOptions = {},
  ): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'write');
      
      // Create backup if requested
      let backupPath: string | undefined;
      if (options.backup) {
        try {
          await fs.access(validatedPath);
          const suffix = options.backupSuffix || '.backup';
          backupPath = `${validatedPath}${suffix}`;
          await fs.copyFile(validatedPath, backupPath);
        } catch {
          // File doesn't exist, no backup needed
        }
      }
      
      // Create temporary file
      const tempName = this.pathSecurity.createTempFilename(validatedPath);
      const tempPath = path.join(this.tempDirectory, tempName);
      
      try {
        // Validate content size
        const contentBuffer = Buffer.isBuffer(content) 
          ? content 
          : Buffer.from(content, options.encoding || 'utf8');
        this.fileSizeValidator.validateSize(contentBuffer.length, filePath);
        
        // Write to temporary file
        await fs.writeFile(tempPath, contentBuffer, {
          mode: options.mode,
          encoding: options.encoding,
        });
        
        // Sync to disk if requested
        if (options.fsync) {
          const fd = await fs.open(tempPath, 'r+');
          try {
            await fd.sync();
          } finally {
            await fd.close();
          }
        }
        
        // Atomic move from temp to final location
        await fs.rename(tempPath, validatedPath);
        
      } catch (error) {
        // Clean up temp file on error
        try {
          await fs.unlink(tempPath);
        } catch {
          // Ignore cleanup errors
        }
        
        // Restore backup if it exists
        if (backupPath) {
          try {
            await fs.copyFile(backupPath, validatedPath);
          } catch {
            // Ignore restore errors
          }
        }
        
        throw error;
      }
      
      // Clean up backup file if write was successful
      if (backupPath) {
        try {
          await fs.unlink(backupPath);
        } catch {
          // Ignore cleanup errors
        }
      }
    } finally {
      release();
    }
  }

  // ========== DIRECTORY OPERATIONS ==========

  /**
   * Creates a directory with parent directories
   */
  async createDirectory(dirPath: string, mode = 0o755): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = this.pathSecurity.validatePath(dirPath);
      await fs.mkdir(validatedPath, { recursive: true, mode });
    } finally {
      release();
    }
  }

  /**
   * Lists directory contents with metadata
   */
  async listDirectory(
    dirPath: string,
    respectGitignore = true,
    additionalIgnore: string[] = [],
  ): Promise<DirectoryEntry[]> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(dirPath, 'read');
      
      // Ensure it's a directory
      const stats = await fs.stat(validatedPath);
      if (!stats.isDirectory()) {
        throw new FileSystemError(
          'Path is not a directory',
          'NOT_DIRECTORY',
          dirPath,
        );
      }
      
      const entries = await fs.readdir(validatedPath);
      const entryPromises = entries.map(async (name): Promise<DirectoryEntry | null> => {
        try {
          const entryPath = path.join(validatedPath, name);
          const entryStats = await fs.lstat(entryPath);
          
          // Check if should be ignored
          if (respectGitignore) {
            const isIgnored = await this.gitIgnore.isIgnored(entryPath, additionalIgnore);
            if (isIgnored) {
              return null;
            }
          }
          
          const type = entryStats.isDirectory() 
            ? 'directory' 
            : entryStats.isFile() 
            ? 'file' 
            : entryStats.isSymbolicLink() 
            ? 'symlink' 
            : 'other';
          
          const relativePath = this.pathSecurity.getRelativePath(entryPath);
          
          return {
            name,
            path: relativePath,
            type,
            size: entryStats.isFile() ? entryStats.size : undefined,
            mtime: entryStats.mtime,
            mimeType: entryStats.isFile() ? mimeTypes.lookup(name) || undefined : undefined,
          };
        } catch (error) {
          // Skip entries we can't read
          console.warn(`Warning: Could not read directory entry ${name}:`, error);
          return null;
        }
      });
      
      const results = await Promise.all(entryPromises);
      return results.filter((entry): entry is DirectoryEntry => entry !== null);
    } finally {
      release();
    }
  }

  /**
   * Removes a directory and its contents
   */
  async removeDirectory(dirPath: string, recursive = false): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(dirPath, 'write');
      
      if (recursive) {
        await fs.rm(validatedPath, { recursive: true, force: true });
      } else {
        await fs.rmdir(validatedPath);
      }
    } finally {
      release();
    }
  }

  // ========== FILE METADATA OPERATIONS ==========

  /**
   * Gets file metadata
   */
  async getFileMetadata(filePath: string): Promise<FileMetadata> {
    const validatedPath = await this.validateFileAccess(filePath, 'read');
    const stats = await fs.lstat(validatedPath);
    const relativePath = this.pathSecurity.getRelativePath(validatedPath);
    const extension = path.extname(filePath).toLowerCase();
    
    return {
      path: relativePath,
      size: stats.size,
      mtime: stats.mtime,
      ctime: stats.ctime,
      isDirectory: stats.isDirectory(),
      isFile: stats.isFile(),
      isSymbolicLink: stats.isSymbolicLink(),
      mode: stats.mode,
      mimeType: stats.isFile() ? mimeTypes.lookup(filePath) || undefined : undefined,
      extension: extension || undefined,
    };
  }

  /**
   * Checks if a file or directory exists
   */
  async exists(filePath: string): Promise<boolean> {
    try {
      const validatedPath = this.pathSecurity.validatePath(filePath);
      await fs.access(validatedPath);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Checks file permissions
   */
  async checkPermissions(
    filePath: string,
    mode: number = fsConstants.F_OK,
  ): Promise<boolean> {
    try {
      const validatedPath = this.pathSecurity.validatePath(filePath);
      await fs.access(validatedPath, mode);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Changes file permissions
   */
  async changePermissions(filePath: string, mode: number): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'write');
      await fs.chmod(validatedPath, mode);
    } finally {
      release();
    }
  }

  // ========== FILE WATCHING ==========

  /**
   * Starts watching a path for changes
   */
  async watchPath(filePath: string, options: WatchOptions = {}): Promise<void> {
    const validatedPath = this.pathSecurity.validatePath(filePath);
    const watchOptions = {
      ...options,
      debounceMs: options.debounceMs || this.options.watchDebounceMs,
    };
    
    await this.watcher.watch(validatedPath, watchOptions);
  }

  /**
   * Stops watching a path
   */
  async unwatchPath(filePath: string): Promise<void> {
    await this.watcher.unwatch(filePath);
  }

  /**
   * Gets the file system watcher instance
   */
  getWatcher(): FileSystemWatcher {
    return this.watcher;
  }

  // ========== UTILITY OPERATIONS ==========

  /**
   * Copies a file or directory
   */
  async copy(
    sourcePath: string,
    destPath: string,
    options: CopyOptions = {},
  ): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedSource = await this.validateFileAccess(sourcePath, 'read');
      const validatedDest = this.pathSecurity.validatePath(destPath);
      
      const sourceStats = await fs.stat(validatedSource);
      
      if (sourceStats.isDirectory() && options.recursive) {
        await this.copyDirectory(validatedSource, validatedDest, options);
      } else if (sourceStats.isFile()) {
        await this.copyFile(validatedSource, validatedDest, options);
      } else {
        throw new FileSystemError(
          'Cannot copy: source is not a file or directory',
          'INVALID_SOURCE',
          sourcePath,
        );
      }
    } finally {
      release();
    }
  }

  /**
   * Moves/renames a file or directory
   */
  async move(sourcePath: string, destPath: string): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedSource = await this.validateFileAccess(sourcePath, 'write');
      const validatedDest = this.pathSecurity.validatePath(destPath);
      
      await fs.rename(validatedSource, validatedDest);
    } finally {
      release();
    }
  }

  /**
   * Removes a file
   */
  async removeFile(filePath: string): Promise<void> {
    const release = await this.operationLimiter.acquire();
    try {
      const validatedPath = await this.validateFileAccess(filePath, 'write');
      await fs.unlink(validatedPath);
    } finally {
      release();
    }
  }

  /**
   * Gets service statistics
   */
  getStats(): {
    rootDirectory: string;
    maxFileSize: number;
    activeOperations: number;
    maxConcurrentOps: number;
    watcherStats: any;
  } {
    return {
      rootDirectory: this.pathSecurity.getRootDirectory(),
      maxFileSize: this.fileSizeValidator.getMaxSize(),
      activeOperations: this.operationLimiter.getActiveCount(),
      maxConcurrentOps: this.operationLimiter.getMaxConcurrent(),
      watcherStats: this.watcher.getStats(),
    };
  }

  /**
   * Cleanup resources
   */
  async cleanup(): Promise<void> {
    await this.watcher.unwatchAll();
    this.gitIgnore.clearCache();
    
    // Clean up temp directory
    try {
      const tempExists = await this.exists(this.tempDirectory);
      if (tempExists) {
        const tempContents = await fs.readdir(this.tempDirectory);
        for (const file of tempContents) {
          try {
            await fs.unlink(path.join(this.tempDirectory, file));
          } catch {
            // Ignore cleanup errors
          }
        }
      }
    } catch {
      // Ignore cleanup errors
    }
  }

  // ========== PRIVATE HELPER METHODS ==========

  /**
   * Validates file access for security and permissions
   */
  private async validateFileAccess(
    filePath: string,
    operation: 'read' | 'write',
  ): Promise<string> {
    // Validate filename if this is a write operation (for security)
    if (operation === 'write') {
      const filename = path.basename(filePath);
      this.pathSecurity.validateFilename(filename);
    }
    
    const validatedPath = await this.pathSecurity.validateSymlink(
      filePath,
      this.options.followSymlinks,
    );
    
    // Check if file exists for read operations
    if (operation === 'read') {
      try {
        await fs.access(validatedPath, fsConstants.F_OK);
      } catch (error) {
        throw new FileSystemError(
          `File not found: ${filePath}`,
          'ENOENT',
          filePath,
          error as Error,
        );
      }
    }
    
    // Check permissions
    const permission = operation === 'read' ? fsConstants.R_OK : fsConstants.W_OK;
    try {
      // For write operations on non-existent files, check parent directory
      if (operation === 'write') {
        try {
          await fs.access(validatedPath, fsConstants.F_OK);
          // File exists, check write permission
          await fs.access(validatedPath, permission);
        } catch (error) {
          // File doesn't exist, check parent directory write permission
          const parentDir = path.dirname(validatedPath);
          await fs.access(parentDir, fsConstants.W_OK);
        }
      } else {
        await fs.access(validatedPath, permission);
      }
    } catch (error) {
      throw new PermissionError(
        `Permission denied: cannot ${operation} ${filePath}`,
        filePath,
      );
    }
    
    return validatedPath;
  }

  /**
   * Ensures the temporary directory exists
   */
  private async ensureTempDirectory(): Promise<void> {
    try {
      await fs.mkdir(this.tempDirectory, { recursive: true, mode: 0o700 });
    } catch (error) {
      console.warn('Warning: Could not create temp directory:', error);
    }
  }

  /**
   * Copies a single file
   */
  private async copyFile(
    sourcePath: string,
    destPath: string,
    options: CopyOptions,
  ): Promise<void> {
    // Check if destination exists
    if (!options.overwrite) {
      try {
        await fs.access(destPath);
        throw new FileSystemError(
          'Destination file already exists',
          'EEXIST',
          destPath,
        );
      } catch (error) {
        if ((error as any).code !== 'ENOENT') {
          throw error;
        }
      }
    }
    
    // Copy the file
    await fs.copyFile(sourcePath, destPath);
    
    // Preserve timestamps and mode if requested
    if (options.preserveTimestamps || options.preserveMode) {
      const sourceStats = await fs.stat(sourcePath);
      
      if (options.preserveMode) {
        await fs.chmod(destPath, sourceStats.mode);
      }
      
      if (options.preserveTimestamps) {
        await fs.utimes(destPath, sourceStats.atime, sourceStats.mtime);
      }
    }
  }

  /**
   * Copies a directory recursively
   */
  private async copyDirectory(
    sourcePath: string,
    destPath: string,
    options: CopyOptions,
  ): Promise<void> {
    // Create destination directory
    await fs.mkdir(destPath, { recursive: true });
    
    // Get directory contents
    const entries = await fs.readdir(sourcePath);
    
    for (const entry of entries) {
      const sourceEntry = path.join(sourcePath, entry);
      const destEntry = path.join(destPath, entry);
      
      // Check exclusion patterns
      if (options.exclude?.some(pattern => entry.includes(pattern))) {
        continue;
      }
      
      const stats = await fs.lstat(sourceEntry);
      
      if (stats.isDirectory()) {
        await this.copyDirectory(sourceEntry, destEntry, options);
      } else if (stats.isFile()) {
        await this.copyFile(sourceEntry, destEntry, options);
        
        // Report progress
        if (options.onProgress) {
          options.onProgress({
            bytesProcessed: stats.size,
            totalBytes: stats.size,
            percentage: 100,
          });
        }
      }
    }
  }
}