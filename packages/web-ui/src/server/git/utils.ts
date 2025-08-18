import { promises as fs } from 'fs';
import path from 'path';
import { GitServiceOptions, GitConfig, AuthConfig, NetworkConfig, SecurityConfig } from './types.js';

/**
 * Creates a default git service configuration
 */
export function createDefaultGitConfig(baseDir: string): GitConfig {
  return {
    baseDir,
    maxConcurrentOps: 3,
    timeoutMs: 300000, // 5 minutes
    retryAttempts: 3,
    retryDelayMs: 1000,
    maxRepoSizeMB: 1024, // 1GB
    enableProgressTracking: true,
  };
}

/**
 * Creates a default network configuration
 */
export function createDefaultNetworkConfig(): NetworkConfig {
  return {
    retryAttempts: 3,
    retryDelayMs: 1000,
    timeoutMs: 30000,
    offlineDetection: true,
  };
}

/**
 * Creates a default security configuration
 */
export function createDefaultSecurityConfig(): SecurityConfig {
  return {
    allowedProtocols: ['https:', 'ssh:', 'git:'],
    allowedHosts: [], // Empty means all hosts allowed
    maxFileSize: 100 * 1024 * 1024, // 100MB
    sanitizeUrls: true,
    validateSslCerts: true,
  };
}

/**
 * Creates auth configuration from environment variables
 */
export function createAuthFromEnv(): AuthConfig | undefined {
  // SSH configuration
  const sshKeyPath = process.env.GIT_SSH_KEY_PATH;
  if (sshKeyPath) {
    return {
      type: 'ssh',
      privateKeyPath: sshKeyPath,
      passphrase: process.env.GIT_SSH_PASSPHRASE,
    };
  }

  // Token-based authentication
  const token = process.env.GIT_TOKEN || process.env.GITHUB_TOKEN;
  if (token) {
    return {
      type: 'token',
      token,
      username: process.env.GIT_USERNAME || 'token',
    };
  }

  // HTTPS authentication
  const username = process.env.GIT_USERNAME;
  const password = process.env.GIT_PASSWORD;
  if (username && password) {
    return {
      type: 'https',
      username,
      password,
    };
  }

  return undefined;
}

/**
 * Validates a git configuration
 */
export async function validateGitConfig(config: GitConfig): Promise<string[]> {
  const errors: string[] = [];

  // Validate base directory
  try {
    const stats = await fs.stat(config.baseDir);
    if (!stats.isDirectory()) {
      errors.push('Base directory is not a valid directory');
    }
  } catch {
    errors.push('Base directory does not exist or is not accessible');
  }

  // Validate numeric values
  if (config.maxConcurrentOps && (config.maxConcurrentOps < 1 || config.maxConcurrentOps > 10)) {
    errors.push('maxConcurrentOps must be between 1 and 10');
  }

  if (config.timeoutMs && (config.timeoutMs < 1000 || config.timeoutMs > 3600000)) {
    errors.push('timeoutMs must be between 1 second and 1 hour');
  }

  if (config.retryAttempts && (config.retryAttempts < 0 || config.retryAttempts > 10)) {
    errors.push('retryAttempts must be between 0 and 10');
  }

  if (config.retryDelayMs && (config.retryDelayMs < 100 || config.retryDelayMs > 60000)) {
    errors.push('retryDelayMs must be between 100ms and 60 seconds');
  }

  if (config.maxRepoSizeMB && (config.maxRepoSizeMB < 1 || config.maxRepoSizeMB > 10240)) {
    errors.push('maxRepoSizeMB must be between 1MB and 10GB');
  }

  return errors;
}

/**
 * Validates an auth configuration
 */
export async function validateAuthConfig(auth: AuthConfig): Promise<string[]> {
  const errors: string[] = [];

  switch (auth.type) {
    case 'ssh':
      if (!auth.privateKeyPath) {
        errors.push('SSH auth requires privateKeyPath');
      } else {
        try {
          const stats = await fs.stat(auth.privateKeyPath);
          if (!stats.isFile()) {
            errors.push('SSH private key path is not a file');
          }
          
          // Check permissions (should not be world-readable)
          const mode = stats.mode & parseInt('777', 8);
          if (mode & parseInt('044', 8)) {
            errors.push('SSH private key has overly permissive permissions');
          }
        } catch {
          errors.push('SSH private key file does not exist or is not accessible');
        }
      }
      break;

    case 'https':
      if (!auth.username || !auth.password) {
        errors.push('HTTPS auth requires username and password');
      }
      break;

    case 'token':
      if (!auth.token) {
        errors.push('Token auth requires token');
      }
      if (auth.token && auth.token.length < 8) {
        errors.push('Token appears to be too short');
      }
      break;

    default:
      errors.push('Invalid auth type');
  }

  return errors;
}

/**
 * Creates a complete git service configuration with defaults
 */
export function createGitServiceOptions(
  baseDir: string,
  overrides?: Partial<GitServiceOptions>,
): GitServiceOptions {
  const config = createDefaultGitConfig(baseDir);
  const network = createDefaultNetworkConfig();
  const security = createDefaultSecurityConfig();
  const auth = createAuthFromEnv();

  return {
    config: { ...config, ...overrides?.config },
    network: { ...network, ...overrides?.network },
    security: { ...security, ...overrides?.security },
    auth: overrides?.auth || auth,
    logger: overrides?.logger,
  };
}

/**
 * Detects if we're in an offline environment
 */
export async function detectOfflineMode(): Promise<boolean> {
  try {
    // Try to resolve a well-known DNS name
    const { lookup } = await import('dns/promises');
    await lookup('github.com');
    return false;
  } catch {
    return true;
  }
}

/**
 * Estimates repository size
 */
export async function estimateRepoSize(repoPath: string): Promise<number> {
  try {
    const gitDir = path.join(repoPath, '.git');
    const stats = await getDirectorySize(gitDir);
    return Math.round(stats / (1024 * 1024)); // Convert to MB
  } catch {
    return 0;
  }
}

/**
 * Gets the total size of a directory recursively
 */
async function getDirectorySize(dirPath: string): Promise<number> {
  let totalSize = 0;

  try {
    const entries = await fs.readdir(dirPath, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name);

      if (entry.isDirectory()) {
        totalSize += await getDirectorySize(fullPath);
      } else if (entry.isFile()) {
        const stats = await fs.stat(fullPath);
        totalSize += stats.size;
      }
    }
  } catch {
    // Ignore errors (permission denied, etc.)
  }

  return totalSize;
}

/**
 * Checks if git is available on the system
 */
export async function checkGitAvailability(): Promise<{ available: boolean; version?: string; error?: string }> {
  try {
    const { execFile } = await import('child_process');
    const { promisify } = await import('util');
    const execFileAsync = promisify(execFile);

    const { stdout } = await execFileAsync('git', ['--version']);
    const version = stdout.trim().replace('git version ', '');

    return {
      available: true,
      version,
    };
  } catch (error) {
    return {
      available: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Parses a git URL to extract components
 */
export function parseGitUrl(url: string): {
  protocol: string;
  host: string;
  owner: string;
  repo: string;
  isValid: boolean;
} {
  try {
    // Handle SSH URLs (git@github.com:owner/repo.git)
    if (url.startsWith('git@')) {
      const match = url.match(/^git@([^:]+):([^/]+)\/(.+?)(?:\.git)?$/);
      if (match) {
        return {
          protocol: 'ssh',
          host: match[1],
          owner: match[2],
          repo: match[3],
          isValid: true,
        };
      }
    }

    // Handle HTTP(S) URLs
    const parsed = new URL(url);
    const pathParts = parsed.pathname.split('/').filter(Boolean);
    
    if (pathParts.length >= 2) {
      const owner = pathParts[0];
      const repo = pathParts[1].replace(/\.git$/, '');
      
      return {
        protocol: parsed.protocol.replace(':', ''),
        host: parsed.hostname,
        owner,
        repo,
        isValid: true,
      };
    }

    return {
      protocol: '',
      host: '',
      owner: '',
      repo: '',
      isValid: false,
    };
  } catch {
    return {
      protocol: '',
      host: '',
      owner: '',
      repo: '',
      isValid: false,
    };
  }
}

/**
 * Sanitizes sensitive information from git URLs for logging
 */
export function sanitizeGitUrl(url: string): string {
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
    // For SSH URLs or invalid URLs, just replace potential credentials
    return url.replace(/:([^@]+)@/, ':[REDACTED]@');
  }
}

/**
 * Creates a retry function with exponential backoff
 */
export function createRetryFunction<T>(
  fn: () => Promise<T>,
  maxAttempts: number,
  initialDelay: number,
  maxDelay = 30000,
): () => Promise<T> {
  return async (): Promise<T> => {
    let lastError: Error;
    
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        return await fn();
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        
        if (attempt === maxAttempts) {
          throw lastError;
        }
        
        const delay = Math.min(initialDelay * Math.pow(2, attempt - 1), maxDelay);
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
    
    throw lastError!;
  };
}

/**
 * Formats bytes to human readable string
 */
export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;
  
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }
  
  return `${size.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

/**
 * Formats duration to human readable string
 */
export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${ms}ms`;
  }
  
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) {
    return `${seconds}s`;
  }
  
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  
  if (minutes < 60) {
    return remainingSeconds > 0 ? `${minutes}m ${remainingSeconds}s` : `${minutes}m`;
  }
  
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  
  return remainingMinutes > 0 ? `${hours}h ${remainingMinutes}m` : `${hours}h`;
}