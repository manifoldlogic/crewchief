import type { BranchSummary, StatusResult, DiffResult, RemoteWithRefs } from 'simple-git';

export interface GitConfig {
  baseDir: string;
  maxConcurrentOps?: number;
  timeoutMs?: number;
  retryAttempts?: number;
  retryDelayMs?: number;
  maxRepoSizeMB?: number;
  enableProgressTracking?: boolean;
}

export interface WorktreeInfo {
  path: string;
  branch: string;
  commit: string;
  isDetached: boolean;
  isBare: boolean;
  isPrunable?: boolean;
  reason?: string;
}

export interface BranchInfo {
  name: string;
  current: boolean;
  commit: string;
  label: string;
  upstream?: string;
  ahead?: number;
  behind?: number;
}

export interface CommitInfo {
  hash: string;
  message: string;
  author: {
    name: string;
    email: string;
    date: Date;
  };
  committer: {
    name: string;
    email: string;
    date: Date;
  };
  refs?: string[];
  body?: string;
}

export interface FileStatus {
  path: string;
  index: string;
  working_dir: string;
  staged: boolean;
  modified: boolean;
  created: boolean;
  deleted: boolean;
  renamed: boolean;
  conflicted: boolean;
}

export interface MergeConflict {
  file: string;
  reason: string;
  content?: string;
  sections?: ConflictSection[];
}

export interface ConflictSection {
  type: 'ours' | 'theirs' | 'base';
  startLine: number;
  endLine: number;
  content: string;
}

export interface DiffChunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  header: string;
  lines: DiffLine[];
}

export interface DiffLine {
  type: 'add' | 'delete' | 'context';
  content: string;
  lineNumber?: {
    old?: number;
    new?: number;
  };
}

export interface GitProgress {
  stage: string;
  progress: number;
  total?: number;
  method: string;
  repository?: string;
  remoteMessages?: string[];
}

export interface GitOperation {
  id: string;
  type: GitOperationType;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  startTime?: Date;
  endTime?: Date;
  progress?: GitProgress;
  error?: string;
  result?: any;
}

export type GitOperationType = 
  | 'clone'
  | 'fetch'
  | 'pull'
  | 'push'
  | 'commit'
  | 'checkout'
  | 'merge'
  | 'worktree-add'
  | 'worktree-remove'
  | 'worktree-prune'
  | 'status'
  | 'diff'
  | 'log'
  | 'branch-create'
  | 'branch-delete';

export interface AuthConfig {
  type: 'ssh' | 'https' | 'token';
  username?: string;
  password?: string;
  token?: string;
  privateKeyPath?: string;
  passphrase?: string;
}

export interface NetworkConfig {
  retryAttempts: number;
  retryDelayMs: number;
  timeoutMs: number;
  offlineDetection: boolean;
}

export interface SecurityConfig {
  allowedProtocols: string[];
  allowedHosts: string[];
  maxFileSize: number;
  sanitizeUrls: boolean;
  validateSslCerts: boolean;
}

export interface GitServiceOptions {
  config: GitConfig;
  auth?: AuthConfig;
  network?: NetworkConfig;
  security?: SecurityConfig;
  logger?: GitLogger;
}

export interface GitLogger {
  info(message: string, meta?: any): void;
  warn(message: string, meta?: any): void;
  error(message: string, meta?: any): void;
  debug(message: string, meta?: any): void;
}

export interface ProgressCallback {
  (progress: GitProgress): void;
}

export interface RetryOptions {
  attempts: number;
  delay: number;
  backoff?: 'linear' | 'exponential';
  maxDelay?: number;
}

export interface LockInfo {
  operationId: string;
  type: GitOperationType;
  acquiredAt: Date;
  expiresAt: Date;
}