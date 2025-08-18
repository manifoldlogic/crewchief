/**
 * Secure File System Operations Module
 * 
 * This module provides comprehensive file system operations with security features:
 * - Path traversal attack prevention
 * - File size limits and validation
 * - Atomic operations with rollback
 * - File watching with debouncing
 * - .gitignore support
 * - Streaming for large files
 * - Permission handling
 * - Symbolic link security
 */

// Main service
export { FileSystemService } from './service.js';

// Security utilities
export { 
  PathSecurity, 
  FileSizeValidator, 
  OperationLimiter 
} from './security.js';

// GitIgnore support
export { GitIgnoreHandler } from './gitignore.js';

// File watching
export { 
  FileSystemWatcher, 
  createFileSystemWatcher 
} from './watcher.js';

// Types and interfaces
export type {
  FileSystemOptions,
  FileMetadata,
  DirectoryEntry,
  ReadStreamOptions,
  WriteStreamOptions,
  AtomicWriteOptions,
  CopyOptions,
  WatchOptions,
  WatchEvent,
  ProgressInfo,
  GitIgnoreOptions,
} from './types.js';

// Error classes
export {
  FileSystemError,
  SecurityError,
  FileSizeError,
  PermissionError,
} from './types.js';

// Import the service locally for factory functions
import { FileSystemService } from './service.js';

/**
 * Factory function to create a configured FileSystemService
 */
export function createFileSystemService(options: FileSystemOptions): FileSystemService {
  return new FileSystemService(options);
}

/**
 * Creates a FileSystemService for a project directory with sensible defaults
 */
export function createProjectFileSystemService(
  projectRoot: string,
  maxFileSize = 100 * 1024 * 1024, // 100MB
): FileSystemService {
  return new FileSystemService({
    rootDirectory: projectRoot,
    maxFileSize,
    followSymlinks: false,
    watchDebounceMs: 300,
    maxConcurrentOps: 10,
  });
}

/**
 * Security-focused FileSystemService with stricter limits
 */
export function createSecureFileSystemService(
  rootDirectory: string,
  maxFileSize = 10 * 1024 * 1024, // 10MB
): FileSystemService {
  return new FileSystemService({
    rootDirectory,
    maxFileSize,
    followSymlinks: false,
    watchDebounceMs: 500,
    maxConcurrentOps: 5,
  });
}

/**
 * Default export for convenience
 */
export { FileSystemService as default } from './service.js';