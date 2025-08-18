import path from 'path';
import { promises as fs } from 'fs';
import { SecurityError, FileSizeError } from './types.js';

/**
 * Security utilities for path validation and sanitization
 */
export class PathSecurity {
  private readonly rootPath: string;
  private readonly normalizedRoot: string;

  constructor(rootDirectory: string) {
    this.rootPath = path.resolve(rootDirectory);
    this.normalizedRoot = path.normalize(this.rootPath);
  }

  /**
   * Validates and normalizes a path to ensure it's within the root directory
   */
  validatePath(inputPath: string): string {
    if (!inputPath || typeof inputPath !== 'string') {
      throw new SecurityError('Invalid path: path must be a non-empty string');
    }

    // Remove null bytes and other dangerous characters
    const sanitized = this.sanitizePath(inputPath);
    
    // Resolve the path relative to root
    const resolvedPath = path.resolve(this.rootPath, sanitized);
    const normalizedPath = path.normalize(resolvedPath);

    // Ensure the resolved path is within the root directory
    if (!this.isWithinRoot(normalizedPath)) {
      throw new SecurityError(
        `Path traversal attempt detected: ${inputPath} resolves outside root directory`,
        inputPath,
      );
    }

    return normalizedPath;
  }

  /**
   * Gets a path relative to the root directory
   */
  getRelativePath(absolutePath: string): string {
    const validated = this.validatePath(absolutePath);
    return path.relative(this.rootPath, validated);
  }

  /**
   * Checks if a path is within the root directory
   */
  isWithinRoot(targetPath: string): boolean {
    const normalizedTarget = path.normalize(path.resolve(targetPath));
    return normalizedTarget.startsWith(this.normalizedRoot + path.sep) || 
           normalizedTarget === this.normalizedRoot;
  }

  /**
   * Sanitizes a path by removing dangerous characters
   */
  private sanitizePath(inputPath: string): string {
    // Remove null bytes
    let sanitized = inputPath.replace(/\0/g, '');
    
    // Remove or replace other dangerous characters
    sanitized = sanitized.replace(/[<>:"|?*]/g, '');
    
    // Normalize path separators
    sanitized = sanitized.replace(/[/\\]+/g, path.sep);
    
    // Remove leading/trailing whitespace
    sanitized = sanitized.trim();
    
    // Ensure we don't have an empty path
    if (!sanitized) {
      throw new SecurityError('Path becomes empty after sanitization');
    }
    
    return sanitized;
  }

  /**
   * Validates that a filename is safe (no path traversal, dangerous chars)
   */
  validateFilename(filename: string): string {
    if (!filename || typeof filename !== 'string') {
      throw new SecurityError('Invalid filename: must be a non-empty string');
    }

    // Check for path separators (indicates path traversal attempt)
    if (filename.includes('/') || filename.includes('\\')) {
      throw new SecurityError('Filename cannot contain path separators', filename);
    }

    // Check for dangerous filenames
    const dangerous = ['.', '..', 'CON', 'PRN', 'AUX', 'NUL'];
    const upper = filename.toUpperCase();
    if (dangerous.includes(upper) || dangerous.some(d => upper.startsWith(d + '.'))) {
      throw new SecurityError(`Dangerous filename not allowed: ${filename}`, filename);
    }

    // Remove dangerous characters
    const sanitized = filename.replace(/[<>:"|?*\x00-\x1f]/g, '');
    
    if (!sanitized || sanitized !== filename) {
      throw new SecurityError(`Filename contains invalid characters: ${filename}`, filename);
    }

    return sanitized;
  }

  /**
   * Creates a secure temporary filename
   */
  createTempFilename(originalPath: string, suffix = '.tmp'): string {
    const basename = path.basename(originalPath);
    const timestamp = Date.now();
    const random = Math.random().toString(36).substring(2, 8);
    return `${basename}.${timestamp}.${random}${suffix}`;
  }

  /**
   * Checks if a path might be a symbolic link to outside the root
   */
  async validateSymlink(symlinkPath: string, followSymlinks = false): Promise<string> {
    const validated = this.validatePath(symlinkPath);
    
    try {
      const stats = await fs.lstat(validated);
      
      if (stats.isSymbolicLink()) {
        if (!followSymlinks) {
          throw new SecurityError('Symbolic links are not allowed', symlinkPath);
        }
        
        // Resolve the symlink and validate the target
        const target = await fs.readlink(validated);
        const resolvedTarget = path.resolve(path.dirname(validated), target);
        
        if (!this.isWithinRoot(resolvedTarget)) {
          throw new SecurityError(
            'Symbolic link points outside root directory',
            symlinkPath,
          );
        }
        
        return resolvedTarget;
      }
      
      return validated;
    } catch (error) {
      if (error instanceof SecurityError) {
        throw error;
      }
      
      // If file doesn't exist, just return the validated path
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
        return validated;
      }
      
      throw new SecurityError(
        `Failed to validate path: ${(error as Error).message}`,
        symlinkPath,
        error as Error,
      );
    }
  }

  /**
   * Gets the root directory
   */
  getRootDirectory(): string {
    return this.rootPath;
  }
}

/**
 * File size validation utility
 */
export class FileSizeValidator {
  constructor(private readonly maxSize: number) {}

  /**
   * Validates file size before operations
   */
  validateSize(size: number, filePath?: string): void {
    if (size > this.maxSize) {
      throw new FileSizeError(
        `File size ${size} bytes exceeds maximum allowed size ${this.maxSize} bytes`,
        filePath,
        size,
        this.maxSize,
      );
    }
  }

  /**
   * Gets the maximum allowed file size
   */
  getMaxSize(): number {
    return this.maxSize;
  }
}

/**
 * Operation rate limiter to prevent resource exhaustion
 */
export class OperationLimiter {
  private activeOperations = 0;

  constructor(private readonly maxConcurrent: number) {}

  /**
   * Acquires a slot for an operation
   */
  async acquire(): Promise<() => void> {
    while (this.activeOperations >= this.maxConcurrent) {
      await new Promise(resolve => setTimeout(resolve, 10));
    }
    
    this.activeOperations++;
    
    return () => {
      this.activeOperations--;
    };
  }

  /**
   * Gets current operation count
   */
  getActiveCount(): number {
    return this.activeOperations;
  }

  /**
   * Gets maximum concurrent operations
   */
  getMaxConcurrent(): number {
    return this.maxConcurrent;
  }
}