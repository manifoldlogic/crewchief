import type { Stats } from 'fs';
import type { Readable, Writable } from 'stream';

export interface FileSystemOptions {
  /** Maximum file size in bytes (default: 100MB) */
  maxFileSize?: number;
  /** Root directory to restrict operations to */
  rootDirectory: string;
  /** Whether to follow symbolic links (default: false) */
  followSymlinks?: boolean;
  /** Temporary directory for atomic operations */
  tempDirectory?: string;
  /** File watch debounce delay in milliseconds (default: 300) */
  watchDebounceMs?: number;
  /** Maximum number of concurrent operations (default: 10) */
  maxConcurrentOps?: number;
}

export interface FileMetadata {
  /** File path relative to root */
  path: string;
  /** File size in bytes */
  size: number;
  /** Last modified timestamp */
  mtime: Date;
  /** Creation timestamp */
  ctime: Date;
  /** Is directory */
  isDirectory: boolean;
  /** Is file */
  isFile: boolean;
  /** Is symbolic link */
  isSymbolicLink: boolean;
  /** File permissions (mode) */
  mode: number;
  /** MIME type */
  mimeType?: string;
  /** File extension */
  extension?: string;
}

export interface DirectoryEntry {
  /** Entry name */
  name: string;
  /** Entry path relative to root */
  path: string;
  /** Entry type */
  type: 'file' | 'directory' | 'symlink' | 'other';
  /** File size (if file) */
  size?: number;
  /** Last modified timestamp */
  mtime: Date;
  /** MIME type (if file) */
  mimeType?: string;
}

export interface ReadStreamOptions {
  /** Start position in bytes */
  start?: number;
  /** End position in bytes */
  end?: number;
  /** Encoding for text files */
  encoding?: BufferEncoding;
  /** Buffer size for streaming */
  bufferSize?: number;
}

export interface WriteStreamOptions {
  /** Encoding for text files */
  encoding?: BufferEncoding;
  /** File mode/permissions */
  mode?: number;
  /** Whether to append to existing file */
  append?: boolean;
  /** Buffer size for streaming */
  bufferSize?: number;
}

export interface WatchOptions {
  /** Whether to watch subdirectories recursively */
  recursive?: boolean;
  /** File patterns to ignore */
  ignored?: string | string[] | RegExp | ((path: string) => boolean);
  /** Debounce delay in milliseconds */
  debounceMs?: number;
  /** Whether to emit events for initial scan */
  ignoreInitial?: boolean;
}

export interface WatchEvent {
  /** Event type */
  type: 'add' | 'change' | 'unlink' | 'addDir' | 'unlinkDir';
  /** File path relative to root */
  path: string;
  /** File stats (if available) */
  stats?: Stats;
  /** Timestamp of event */
  timestamp: Date;
}

export interface ProgressInfo {
  /** Bytes processed */
  bytesProcessed: number;
  /** Total bytes (if known) */
  totalBytes?: number;
  /** Progress percentage (0-100) */
  percentage?: number;
  /** Estimated time remaining in milliseconds */
  estimatedTimeMs?: number;
}

export interface AtomicWriteOptions extends WriteStreamOptions {
  /** Whether to backup existing file */
  backup?: boolean;
  /** Backup file suffix */
  backupSuffix?: string;
  /** Whether to sync to disk before rename */
  fsync?: boolean;
}

export interface CopyOptions {
  /** Whether to overwrite existing files */
  overwrite?: boolean;
  /** Whether to preserve timestamps */
  preserveTimestamps?: boolean;
  /** Whether to preserve file mode/permissions */
  preserveMode?: boolean;
  /** Whether to copy recursively for directories */
  recursive?: boolean;
  /** File patterns to exclude */
  exclude?: string[];
  /** Progress callback */
  onProgress?: (progress: ProgressInfo) => void;
}

export interface GitIgnoreOptions {
  /** Whether to respect .gitignore files */
  respectGitignore?: boolean;
  /** Additional patterns to ignore */
  additionalIgnore?: string[];
  /** Whether to search parent directories for .gitignore */
  searchParents?: boolean;
}

export class FileSystemError extends Error {
  constructor(
    message: string,
    public code: string,
    public path?: string,
    public originalError?: Error,
  ) {
    super(message);
    this.name = 'FileSystemError';
  }
}

export class SecurityError extends FileSystemError {
  constructor(message: string, path?: string) {
    super(message, 'SECURITY_ERROR', path);
    this.name = 'SecurityError';
  }
}

export class FileSizeError extends FileSystemError {
  constructor(message: string, path?: string, public size?: number, public maxSize?: number) {
    super(message, 'FILE_SIZE_ERROR', path);
    this.name = 'FileSizeError';
  }
}

export class PermissionError extends FileSystemError {
  constructor(message: string, path?: string) {
    super(message, 'PERMISSION_ERROR', path);
    this.name = 'PermissionError';
  }
}