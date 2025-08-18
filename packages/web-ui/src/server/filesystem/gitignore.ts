import path from 'path';
import { promises as fs } from 'fs';
import ignore, { Ignore } from 'ignore';
import { PathSecurity } from './security.js';
import { FileSystemError } from './types.js';

/**
 * GitIgnore handler for respecting .gitignore files
 */
export class GitIgnoreHandler {
  private readonly pathSecurity: PathSecurity;
  private readonly ignoreCache = new Map<string, Ignore>();
  private readonly fileCache = new Map<string, { mtime: number; patterns: string[] }>();

  constructor(rootDirectory: string) {
    this.pathSecurity = new PathSecurity(rootDirectory);
  }

  /**
   * Checks if a path should be ignored based on .gitignore rules
   */
  async isIgnored(filePath: string, additionalPatterns: string[] = []): Promise<boolean> {
    try {
      const validatedPath = this.pathSecurity.validatePath(filePath);
      const relativePath = this.pathSecurity.getRelativePath(validatedPath);
      
      // Get ignore patterns for this path
      const ignoreInstance = await this.getIgnoreForPath(validatedPath, additionalPatterns);
      
      // Check if the path is ignored
      return ignoreInstance.ignores(relativePath);
    } catch (error) {
      // If we can't read .gitignore files, don't ignore by default
      console.warn(`Warning: Could not check .gitignore for ${filePath}:`, error);
      return false;
    }
  }

  /**
   * Filters a list of paths based on .gitignore rules
   */
  async filterIgnored(
    paths: string[], 
    additionalPatterns: string[] = [],
  ): Promise<string[]> {
    const filtered: string[] = [];
    
    for (const filePath of paths) {
      try {
        const isIgnored = await this.isIgnored(filePath, additionalPatterns);
        if (!isIgnored) {
          filtered.push(filePath);
        }
      } catch (error) {
        // Include file if we can't determine ignore status
        filtered.push(filePath);
      }
    }
    
    return filtered;
  }

  /**
   * Gets ignore patterns for a specific path (considering parent .gitignore files)
   */
  private async getIgnoreForPath(
    filePath: string, 
    additionalPatterns: string[] = [],
  ): Promise<Ignore> {
    const cacheKey = `${filePath}:${additionalPatterns.join(',')}`;
    
    // Check cache
    if (this.ignoreCache.has(cacheKey)) {
      return this.ignoreCache.get(cacheKey)!;
    }

    const ignoreInstance = ignore();
    const rootPath = this.pathSecurity.getRootDirectory();
    const allPatterns: string[] = [...additionalPatterns];

    // Collect .gitignore files from root to the target directory
    const gitignoreFiles = await this.collectGitIgnoreFiles(filePath);
    
    // Load patterns from all .gitignore files
    for (const gitignoreFile of gitignoreFiles) {
      try {
        const patterns = await this.loadGitIgnoreFile(gitignoreFile);
        allPatterns.push(...patterns);
      } catch (error) {
        // Ignore errors reading .gitignore files
        console.warn(`Warning: Could not read .gitignore file ${gitignoreFile}:`, error);
      }
    }

    // Add default patterns that should always be ignored
    allPatterns.push(
      '.git/',
      '.gitignore',
      'node_modules/',
      '.DS_Store',
      'Thumbs.db',
      '*.tmp',
      '*.temp',
      '.env.local',
      '.env.*.local',
    );

    // Add all patterns to ignore instance
    if (allPatterns.length > 0) {
      ignoreInstance.add(allPatterns);
    }

    // Cache the ignore instance
    this.ignoreCache.set(cacheKey, ignoreInstance);
    
    // Clean cache periodically
    if (this.ignoreCache.size > 100) {
      this.cleanCache();
    }

    return ignoreInstance;
  }

  /**
   * Collects .gitignore files from root directory up to the target path
   */
  private async collectGitIgnoreFiles(targetPath: string): Promise<string[]> {
    const gitignoreFiles: string[] = [];
    const rootPath = this.pathSecurity.getRootDirectory();
    
    let currentDir = path.dirname(targetPath);
    
    // Walk up from target directory to root
    while (this.pathSecurity.isWithinRoot(currentDir)) {
      const gitignorePath = path.join(currentDir, '.gitignore');
      
      try {
        await fs.access(gitignorePath);
        gitignoreFiles.unshift(gitignorePath); // Add to beginning for correct precedence
      } catch {
        // .gitignore doesn't exist in this directory, continue
      }
      
      // Stop if we've reached the root
      if (currentDir === rootPath) {
        break;
      }
      
      const parentDir = path.dirname(currentDir);
      if (parentDir === currentDir) {
        break; // Reached filesystem root
      }
      
      currentDir = parentDir;
    }

    return gitignoreFiles;
  }

  /**
   * Loads patterns from a .gitignore file with caching
   */
  private async loadGitIgnoreFile(gitignorePath: string): Promise<string[]> {
    try {
      const stats = await fs.stat(gitignorePath);
      const mtime = stats.mtime.getTime();
      
      // Check cache
      const cached = this.fileCache.get(gitignorePath);
      if (cached && cached.mtime === mtime) {
        return cached.patterns;
      }

      // Read and parse .gitignore file
      const content = await fs.readFile(gitignorePath, 'utf8');
      const patterns = this.parseGitIgnoreContent(content);
      
      // Cache the patterns
      this.fileCache.set(gitignorePath, { mtime, patterns });
      
      return patterns;
    } catch (error) {
      throw new FileSystemError(
        `Failed to load .gitignore file: ${(error as Error).message}`,
        'GITIGNORE_READ_ERROR',
        gitignorePath,
        error as Error,
      );
    }
  }

  /**
   * Parses .gitignore file content into patterns
   */
  private parseGitIgnoreContent(content: string): string[] {
    return content
      .split('\n')
      .map(line => line.trim())
      .filter(line => line && !line.startsWith('#')) // Remove empty lines and comments
      .map(line => {
        // Handle patterns that start with !
        if (line.startsWith('!')) {
          return line; // Keep negation patterns as-is
        }
        
        // Handle directory patterns
        if (line.endsWith('/')) {
          return line;
        }
        
        // Handle glob patterns
        return line;
      });
  }

  /**
   * Cleans the cache to prevent memory leaks
   */
  private cleanCache(): void {
    // Keep only the most recently used 50 entries
    const entries = Array.from(this.ignoreCache.entries());
    const toKeep = entries.slice(-50);
    
    this.ignoreCache.clear();
    toKeep.forEach(([key, value]) => {
      this.ignoreCache.set(key, value);
    });

    // Also clean file cache
    const fileEntries = Array.from(this.fileCache.entries());
    const filesToKeep = fileEntries.slice(-50);
    
    this.fileCache.clear();
    filesToKeep.forEach(([key, value]) => {
      this.fileCache.set(key, value);
    });
  }

  /**
   * Clears all caches
   */
  clearCache(): void {
    this.ignoreCache.clear();
    this.fileCache.clear();
  }

  /**
   * Gets cache statistics
   */
  getCacheStats(): { ignoreEntries: number; fileEntries: number } {
    return {
      ignoreEntries: this.ignoreCache.size,
      fileEntries: this.fileCache.size,
    };
  }
}