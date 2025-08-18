import { URL } from 'url';
import { promises as fs } from 'fs';
import path from 'path';
import { GitLogger, SecurityConfig, AuthConfig } from './types.js';

export class GitSecurityManager {
  private readonly config: SecurityConfig;
  private readonly logger: GitLogger;

  constructor(config: SecurityConfig, logger: GitLogger) {
    this.config = {
      allowedProtocols: ['https:', 'ssh:', 'git:'],
      allowedHosts: [],
      maxFileSize: 100 * 1024 * 1024, // 100MB
      sanitizeUrls: true,
      validateSslCerts: true,
      ...config,
    };
    this.logger = logger;
  }

  /**
   * Validates a git URL for security
   */
  validateGitUrl(url: string): boolean {
    try {
      if (this.config.sanitizeUrls) {
        url = this.sanitizeUrl(url);
      }

      const parsed = new URL(url);
      
      // Check protocol
      if (!this.config.allowedProtocols.includes(parsed.protocol)) {
        this.logger.warn('Rejected URL with disallowed protocol', { 
          url: this.redactUrl(url), 
          protocol: parsed.protocol 
        });
        return false;
      }

      // Check hostname if allowlist is configured
      if (this.config.allowedHosts.length > 0) {
        const hostname = parsed.hostname.toLowerCase();
        const isAllowed = this.config.allowedHosts.some(allowed => 
          hostname === allowed.toLowerCase() || 
          hostname.endsWith('.' + allowed.toLowerCase())
        );
        
        if (!isAllowed) {
          this.logger.warn('Rejected URL with disallowed host', { 
            url: this.redactUrl(url), 
            hostname 
          });
          return false;
        }
      }

      // Additional checks for suspicious patterns
      if (this.containsSuspiciousPatterns(url)) {
        this.logger.warn('Rejected URL with suspicious patterns', { 
          url: this.redactUrl(url) 
        });
        return false;
      }

      return true;
    } catch (error) {
      this.logger.error('Invalid URL format', { 
        url: this.redactUrl(url), 
        error: error instanceof Error ? error.message : String(error) 
      });
      return false;
    }
  }

  /**
   * Sanitizes a git URL by removing dangerous characters
   */
  sanitizeUrl(url: string): string {
    // Remove dangerous characters that could be used for command injection
    return url.replace(/[`${}\\;|&<>]/g, '');
  }

  /**
   * Validates file path for security
   */
  validatePath(filePath: string, baseDir: string): boolean {
    try {
      const resolvedPath = path.resolve(filePath);
      const resolvedBase = path.resolve(baseDir);
      
      // Ensure path is within base directory
      if (!resolvedPath.startsWith(resolvedBase)) {
        this.logger.warn('Rejected path outside base directory', { 
          path: filePath, 
          baseDir 
        });
        return false;
      }

      // Check for suspicious path patterns
      if (this.containsSuspiciousPathPatterns(filePath)) {
        this.logger.warn('Rejected path with suspicious patterns', { 
          path: filePath 
        });
        return false;
      }

      return true;
    } catch (error) {
      this.logger.error('Path validation error', { 
        path: filePath, 
        error: error instanceof Error ? error.message : String(error) 
      });
      return false;
    }
  }

  /**
   * Validates file size
   */
  async validateFileSize(filePath: string): Promise<boolean> {
    try {
      const stats = await fs.stat(filePath);
      if (stats.size > this.config.maxFileSize) {
        this.logger.warn('File exceeds maximum allowed size', { 
          path: filePath, 
          size: stats.size, 
          maxSize: this.config.maxFileSize 
        });
        return false;
      }
      return true;
    } catch (error) {
      this.logger.error('File size validation error', { 
        path: filePath, 
        error: error instanceof Error ? error.message : String(error) 
      });
      return false;
    }
  }

  /**
   * Sanitizes authentication configuration
   */
  sanitizeAuthConfig(auth: AuthConfig): AuthConfig {
    const sanitized = { ...auth };
    
    // Ensure sensitive fields are not logged
    if (sanitized.password) {
      delete (sanitized as any).password;
    }
    if (sanitized.token) {
      delete (sanitized as any).token;
    }
    if (sanitized.passphrase) {
      delete (sanitized as any).passphrase;
    }
    
    return sanitized;
  }

  /**
   * Validates SSH key path
   */
  async validateSshKey(keyPath: string): Promise<boolean> {
    try {
      const stats = await fs.stat(keyPath);
      
      // Check if file exists and is readable
      if (!stats.isFile()) {
        this.logger.warn('SSH key path is not a file', { keyPath });
        return false;
      }

      // Check file permissions (should not be world-readable)
      const mode = stats.mode & parseInt('777', 8);
      if (mode & parseInt('044', 8)) {
        this.logger.warn('SSH key has overly permissive permissions', { 
          keyPath, 
          mode: mode.toString(8) 
        });
        return false;
      }

      return true;
    } catch (error) {
      this.logger.error('SSH key validation error', { 
        keyPath, 
        error: error instanceof Error ? error.message : String(error) 
      });
      return false;
    }
  }

  /**
   * Creates environment variables for git operations without exposing credentials
   */
  createSecureEnv(auth?: AuthConfig): Record<string, string> {
    const env: Record<string, string> = {};
    
    if (auth?.type === 'https' && auth.token) {
      // Use credential helper to avoid exposing token in command line
      env.GIT_ASKPASS = 'echo';
      env.GIT_USERNAME = auth.username || 'token';
      env.GIT_PASSWORD = auth.token;
    }
    
    if (auth?.type === 'ssh' && auth.privateKeyPath) {
      env.GIT_SSH_COMMAND = `ssh -i "${auth.privateKeyPath}" -o StrictHostKeyChecking=no`;
    }

    // Disable credential helpers that might expose credentials
    env.GIT_TERMINAL_PROMPT = '0';
    
    return env;
  }

  /**
   * Redacts sensitive information from URLs for logging
   */
  private redactUrl(url: string): string {
    try {
      const parsed = new URL(url);
      if (parsed.password) {
        parsed.password = '[REDACTED]';
      }
      if (parsed.username && parsed.username !== 'git') {
        parsed.username = '[REDACTED]';
      }
      return parsed.toString();
    } catch {
      return '[INVALID_URL]';
    }
  }

  /**
   * Checks for suspicious patterns in URLs
   */
  private containsSuspiciousPatterns(url: string): boolean {
    const suspiciousPatterns = [
      /javascript:/i,
      /data:/i,
      /file:/i,
      /\.\.\//, // Path traversal
      /%2e%2e%2f/i, // Encoded path traversal
      /\$\{/i, // Template injection
      /`[^`]*`/i, // Command substitution
      /\$\([^)]*\)/i, // Command substitution
    ];

    return suspiciousPatterns.some(pattern => pattern.test(url));
  }

  /**
   * Checks for suspicious patterns in file paths
   */
  private containsSuspiciousPathPatterns(filePath: string): boolean {
    const suspiciousPatterns = [
      /\.\.\//,
      /\.\.\\/,
      /\/\.\./,
      /\\\.\./,
      /\0/, // Null bytes
      /[\x00-\x1f]/, // Control characters
    ];

    return suspiciousPatterns.some(pattern => pattern.test(filePath));
  }
}