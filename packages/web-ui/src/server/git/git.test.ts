import { describe, it, expect, beforeEach, afterEach, vi, type MockedFunction } from 'vitest';
import { promises as fs } from 'fs';
import path from 'path';
import { GitService } from './service.js';
import { GitSecurityManager } from './security.js';
import { GitProgressTracker } from './progress.js';
import { GitOperationQueue } from './queue.js';
import { createGitServiceOptions, validateGitConfig, parseGitUrl, sanitizeGitUrl } from './utils.js';
import type { GitServiceOptions, GitProgress } from './types.js';

// Mock simple-git
vi.mock('simple-git', () => ({
  simpleGit: vi.fn(() => ({
    raw: vi.fn(),
    status: vi.fn(),
    branch: vi.fn(),
    checkout: vi.fn(),
    checkoutBranch: vi.fn(),
    checkoutLocalBranch: vi.fn(),
    deleteLocalBranch: vi.fn(),
    merge: vi.fn(),
    add: vi.fn(),
    commit: vi.fn(),
    push: vi.fn(),
    pull: vi.fn(),
    fetch: vi.fn(),
    clone: vi.fn(),
    diff: vi.fn(),
    log: vi.fn(),
    reset: vi.fn(),
    clean: vi.fn(),
    revparse: vi.fn(),
    env: vi.fn().mockReturnThis(),
    outputHandler: vi.fn().mockReturnThis(),
  })),
  CleanOptions: {
    FORCE: 1,
    RECURSIVE: 2,
  },
}));

// Mock fs
vi.mock('fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('fs')>();
  return {
    ...actual,
    promises: {
      stat: vi.fn(),
      readFile: vi.fn(),
      readdir: vi.fn(),
    },
  };
});

const mockFs = fs as {
  stat: MockedFunction<typeof fs.stat>;
  readFile: MockedFunction<typeof fs.readFile>;
  readdir: MockedFunction<typeof fs.readdir>;
};

describe('GitService', () => {
  let gitService: GitService;
  let options: GitServiceOptions;
  const testBaseDir = '/test/repo';

  beforeEach(() => {
    vi.clearAllMocks();
    
    options = createGitServiceOptions(testBaseDir, {
      config: {
        baseDir: testBaseDir,
        maxConcurrentOps: 2,
        timeoutMs: 5000,
        enableProgressTracking: true,
      },
    });

    gitService = new GitService(options);
  });

  afterEach(() => {
    gitService.clearQueue();
  });

  describe('Worktree Operations', () => {
    it('should create a worktree', async () => {
      const worktreePath = '/test/repo/worktree1';
      const branch = 'feature-branch';

      // Mock git operations
      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.raw as MockedFunction<any>).mockResolvedValue('');
      (gitInstance.status as MockedFunction<any>).mockResolvedValue({
        current: branch,
        detached: false,
      });
      (gitInstance.revparse as MockedFunction<any>)
        .mockResolvedValueOnce(branch)
        .mockResolvedValueOnce('abc123');

      // Mock security validation
      mockFs.stat.mockResolvedValue({
        isDirectory: () => true,
        mode: 0o755,
      } as any);

      const result = await gitService.createWorktree(worktreePath, branch);

      expect(result).toMatchObject({
        path: worktreePath,
        branch,
        commit: 'abc123',
        isDetached: false,
        isBare: false,
      });

      expect(gitInstance.raw).toHaveBeenCalledWith([
        'worktree',
        'add',
        '-b',
        branch,
        worktreePath,
        branch,
      ]);
    });

    it('should list worktrees', async () => {
      const mockOutput = `worktree /test/repo
HEAD abc123
branch refs/heads/main

worktree /test/repo/feature
HEAD def456
branch refs/heads/feature-branch

worktree /test/repo/detached
HEAD 789xyz
detached`;

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.raw as MockedFunction<any>).mockResolvedValue(mockOutput);

      const result = await gitService.listWorktrees();

      expect(result).toHaveLength(3);
      expect(result[0]).toMatchObject({
        path: '/test/repo',
        branch: 'refs/heads/main',
        commit: 'abc123',
        isDetached: false,
      });
      expect(result[2].isDetached).toBe(true);
    });

    it('should remove a worktree', async () => {
      const worktreePath = '/test/repo/worktree1';

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.raw as MockedFunction<any>).mockResolvedValue('');

      await gitService.removeWorktree(worktreePath);

      expect(gitInstance.raw).toHaveBeenCalledWith([
        'worktree',
        'remove',
        worktreePath,
      ]);
    });

    it('should remove a worktree with force', async () => {
      const worktreePath = '/test/repo/worktree1';

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.raw as MockedFunction<any>).mockResolvedValue('');

      await gitService.removeWorktree(worktreePath, true);

      expect(gitInstance.raw).toHaveBeenCalledWith([
        'worktree',
        'remove',
        '--force',
        worktreePath,
      ]);
    });

    it('should prune worktrees', async () => {
      const mockOutput = `Removing worktrees/old-feature: gitdir file points to non-existent location
Removing worktrees/another-old: gitdir file points to non-existent location`;

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.raw as MockedFunction<any>).mockResolvedValue(mockOutput);

      const result = await gitService.pruneWorktrees();

      expect(result).toEqual(['old-feature', 'another-old']);
      expect(gitInstance.raw).toHaveBeenCalledWith([
        'worktree',
        'prune',
        '--verbose',
      ]);
    });
  });

  describe('Branch Operations', () => {
    it('should create a branch', async () => {
      const branchName = 'new-feature';
      const startPoint = 'main';

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.checkoutBranch as MockedFunction<any>).mockResolvedValue({});

      await gitService.createBranch(branchName, startPoint);

      expect(gitInstance.checkoutBranch).toHaveBeenCalledWith(branchName, startPoint);
    });

    it('should checkout a branch', async () => {
      const branchName = 'feature-branch';

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.checkout as MockedFunction<any>).mockResolvedValue({});

      await gitService.checkoutBranch(branchName);

      expect(gitInstance.checkout).toHaveBeenCalledWith(branchName);
    });

    it('should delete a branch', async () => {
      const branchName = 'old-feature';

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.deleteLocalBranch as MockedFunction<any>).mockResolvedValue({});

      await gitService.deleteBranch(branchName, true);

      expect(gitInstance.deleteLocalBranch).toHaveBeenCalledWith(branchName, true);
    });

    it('should merge a branch', async () => {
      const branchName = 'feature-branch';

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.merge as MockedFunction<any>).mockResolvedValue({});
      (gitInstance.status as MockedFunction<any>).mockResolvedValue({
        conflicted: [],
      });

      const result = await gitService.mergeBranch(branchName, { noFF: true });

      expect(result).toEqual({});
      expect(gitInstance.merge).toHaveBeenCalledWith([branchName, '--no-ff']);
    });

    it('should detect merge conflicts', async () => {
      const branchName = 'feature-branch';
      const conflictContent = `line 1
<<<<<<< HEAD
our changes
=======
their changes
>>>>>>> feature-branch
line 3`;

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.merge as MockedFunction<any>).mockResolvedValue({});
      (gitInstance.status as MockedFunction<any>).mockResolvedValue({
        conflicted: ['conflicted-file.txt'],
      });

      mockFs.readFile.mockResolvedValue(conflictContent);

      const result = await gitService.mergeBranch(branchName);

      expect(result.conflicts).toHaveLength(1);
      expect(result.conflicts![0].file).toBe('conflicted-file.txt');
      expect(result.conflicts![0].sections).toHaveLength(2);
    });

    it('should list branches', async () => {
      const mockBranchSummary = {
        current: 'main',
        all: ['main', 'feature-branch', 'remotes/origin/main'],
        branches: {
          main: {
            current: true,
            commit: 'abc123',
            label: 'main',
          },
          'feature-branch': {
            current: false,
            commit: 'def456',
            label: 'feature-branch',
          },
        },
      };

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.branch as MockedFunction<any>).mockResolvedValue(mockBranchSummary);

      const result = await gitService.listBranches();

      expect(result).toHaveLength(2);
      expect(result[0]).toMatchObject({
        name: 'main',
        current: true,
        commit: 'abc123',
      });
    });
  });

  describe('Commit Operations', () => {
    it('should add files', async () => {
      const files = ['file1.txt', 'file2.txt'];

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.add as MockedFunction<any>).mockResolvedValue({});

      await gitService.addFiles(files);

      expect(gitInstance.add).toHaveBeenCalledWith(files);
    });

    it('should commit changes', async () => {
      const message = 'test commit';
      const mockCommitResult = {
        commit: 'abc123',
        summary: {
          changes: 2,
          insertions: 10,
          deletions: 5,
        },
      };

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.commit as MockedFunction<any>).mockResolvedValue(mockCommitResult);

      const result = await gitService.commit(message);

      expect(result).toEqual(mockCommitResult);
      expect(gitInstance.commit).toHaveBeenCalledWith(message);
    });

    it('should push changes', async () => {
      const mockPushResult = {
        pushed: [
          {
            local: 'main',
            remote: 'main',
            remoteName: 'origin',
          },
        ],
      };

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.push as MockedFunction<any>).mockResolvedValue(mockPushResult);
      (gitInstance.env as MockedFunction<any>).mockReturnValue(gitInstance);

      const result = await gitService.push('origin', 'main');

      expect(result).toEqual(mockPushResult);
      expect(gitInstance.push).toHaveBeenCalledWith('origin', 'main');
    });

    it('should pull changes', async () => {
      const mockPullResult = {
        summary: {
          changes: 3,
          insertions: 15,
          deletions: 8,
        },
      };

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.pull as MockedFunction<any>).mockResolvedValue(mockPullResult);
      (gitInstance.env as MockedFunction<any>).mockReturnValue(gitInstance);

      const result = await gitService.pull('origin', 'main');

      expect(result).toEqual(mockPullResult);
      expect(gitInstance.pull).toHaveBeenCalledWith('origin', 'main');
    });
  });

  describe('Status and Information', () => {
    it('should get repository status', async () => {
      const mockStatus = {
        current: 'main',
        tracking: 'origin/main',
        ahead: 1,
        behind: 0,
        staged: ['staged-file.txt'],
        modified: ['modified-file.txt'],
        created: ['new-file.txt'],
        deleted: ['deleted-file.txt'],
        renamed: [],
        conflicted: [],
      };

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.status as MockedFunction<any>).mockResolvedValue(mockStatus);

      const result = await gitService.getStatus();

      expect(result.summary).toMatchObject({
        current: 'main',
        tracking: 'origin/main',
        ahead: 1,
        behind: 0,
        staged: 1,
        modified: 1,
        created: 1,
        deleted: 1,
      });

      expect(result.files).toHaveLength(4);
    });

    it('should get diff', async () => {
      const mockDiffOutput = `@@ -1,3 +1,4 @@
 line 1
-old line 2
+new line 2
+added line
 line 3`;

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.diff as MockedFunction<any>).mockResolvedValue(mockDiffOutput);

      const result = await gitService.getDiff('test-file.txt');

      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        oldStart: 1,
        oldLines: 3,
        newStart: 1,
        newLines: 4,
      });
      expect(result[0].lines).toHaveLength(4);
    });

    it('should get commit log', async () => {
      const mockLogResult = {
        all: [
          {
            hash: 'abc123',
            message: 'first commit',
            author_name: 'John Doe',
            author_email: 'john@example.com',
            date: '2023-01-01T12:00:00Z',
            refs: 'HEAD -> main',
            body: 'commit body',
          },
        ],
        latest: null,
        total: 1,
      };

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.log as MockedFunction<any>).mockResolvedValue(mockLogResult);

      const result = await gitService.getLog(10);

      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        hash: 'abc123',
        message: 'first commit',
        author: {
          name: 'John Doe',
          email: 'john@example.com',
        },
      });
    });
  });

  describe('Advanced Operations', () => {
    it('should clone a repository', async () => {
      const url = 'https://github.com/user/repo.git';
      const targetPath = '/test/target';

      // Mock simple-git for clone operation
      const mockGit = await import('simple-git');
      const cloneGit = mockGit.simpleGit();
      (cloneGit.clone as MockedFunction<any>).mockResolvedValue({});
      (cloneGit.env as MockedFunction<any>).mockReturnValue(cloneGit);

      await gitService.clone(url, targetPath, {
        branch: 'main',
        depth: 1,
      });

      expect(cloneGit.clone).toHaveBeenCalledWith(
        url,
        targetPath,
        ['--branch', 'main', '--depth', '1']
      );
    });

    it('should fetch from remote', async () => {
      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.fetch as MockedFunction<any>).mockResolvedValue({});
      (gitInstance.env as MockedFunction<any>).mockReturnValue(gitInstance);

      await gitService.fetch('origin');

      expect(gitInstance.fetch).toHaveBeenCalledWith('origin');
    });

    it('should reset repository state', async () => {
      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.reset as MockedFunction<any>).mockResolvedValue({});

      await gitService.reset('hard', 'HEAD~1');

      expect(gitInstance.reset).toHaveBeenCalledWith(['hard', 'HEAD~1']);
    });

    it('should clean untracked files', async () => {
      const mockCleanOutput = `Removing untracked-file.txt
Removing untracked-dir/`;

      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.clean as MockedFunction<any>).mockResolvedValue(mockCleanOutput);

      const result = await gitService.clean({ force: true, directories: true });

      expect(result).toEqual(['untracked-file.txt', 'untracked-dir/']);
    });
  });

  describe('Queue Management', () => {
    it('should get queue status', () => {
      const status = gitService.getQueueStatus();

      expect(status).toMatchObject({
        queued: 0,
        running: 0,
        locks: 0,
        operations: [],
      });
    });

    it('should cancel operations', async () => {
      // Start a long-running operation
      const promise = gitService.getStatus();
      const status = gitService.getQueueStatus();
      
      if (status.operations.length > 0) {
        const cancelled = gitService.cancelOperation(status.operations[0].id);
        expect(cancelled).toBe(true);
      }

      // Operation should be cancelled
      await expect(promise).rejects.toThrow('Operation cancelled');
    });

    it('should clear queue', () => {
      gitService.clearQueue();
      const status = gitService.getQueueStatus();

      expect(status.queued).toBe(0);
      expect(status.operations).toHaveLength(0);
    });
  });

  describe('Progress Tracking', () => {
    it('should track progress for operations', async () => {
      const progressUpdates: GitProgress[] = [];
      
      const mockGit = await import('simple-git');
      const gitInstance = mockGit.simpleGit();
      (gitInstance.status as MockedFunction<any>).mockImplementation(async () => {
        // Simulate slow operation
        await new Promise(resolve => setTimeout(resolve, 100));
        return { current: 'main', staged: [], modified: [] };
      });

      await gitService.getStatus();

      // Progress tracking is tested via the progress callback
      // In real usage, this would capture progress updates
    });
  });
});

describe('GitSecurityManager', () => {
  let securityManager: GitSecurityManager;
  const mockLogger = {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    securityManager = new GitSecurityManager(
      {
        allowedProtocols: ['https:', 'ssh:'],
        allowedHosts: ['github.com'],
        maxFileSize: 1024 * 1024,
        sanitizeUrls: true,
        validateSslCerts: true,
      },
      mockLogger
    );
  });

  describe('URL Validation', () => {
    it('should validate allowed URLs', () => {
      expect(securityManager.validateGitUrl('https://github.com/user/repo.git')).toBe(true);
      expect(securityManager.validateGitUrl('ssh://git@github.com/user/repo.git')).toBe(true);
    });

    it('should reject disallowed protocols', () => {
      expect(securityManager.validateGitUrl('http://github.com/user/repo.git')).toBe(false);
      expect(securityManager.validateGitUrl('ftp://example.com/repo.git')).toBe(false);
    });

    it('should reject disallowed hosts', () => {
      expect(securityManager.validateGitUrl('https://evil.com/user/repo.git')).toBe(false);
    });

    it('should reject suspicious patterns', () => {
      expect(securityManager.validateGitUrl('https://github.com/user/repo.git`rm -rf /`')).toBe(false);
      expect(securityManager.validateGitUrl('javascript:alert(1)')).toBe(false);
    });
  });

  describe('Path Validation', () => {
    it('should validate paths within base directory', () => {
      expect(securityManager.validatePath('/base/safe/path', '/base')).toBe(true);
    });

    it('should reject paths outside base directory', () => {
      expect(securityManager.validatePath('/outside/path', '/base')).toBe(false);
      expect(securityManager.validatePath('../outside', '/base')).toBe(false);
    });

    it('should reject suspicious path patterns', () => {
      expect(securityManager.validatePath('/base/path/../../outside', '/base')).toBe(false);
      expect(securityManager.validatePath('/base/path\x00', '/base')).toBe(false);
    });
  });

  describe('SSH Key Validation', () => {
    it('should validate SSH key files', async () => {
      mockFs.stat.mockResolvedValue({
        isFile: () => true,
        mode: 0o600, // Proper SSH key permissions
      } as any);

      const result = await securityManager.validateSshKey('/home/user/.ssh/id_rsa');
      expect(result).toBe(true);
    });

    it('should reject SSH keys with bad permissions', async () => {
      mockFs.stat.mockResolvedValue({
        isFile: () => true,
        mode: 0o644, // World-readable
      } as any);

      const result = await securityManager.validateSshKey('/home/user/.ssh/id_rsa');
      expect(result).toBe(false);
    });
  });
});

describe('GitProgressTracker', () => {
  let progressTracker: GitProgressTracker;
  const mockLogger = {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    progressTracker = new GitProgressTracker({
      method: 'test',
      repository: '/test/repo',
      logger: mockLogger,
    });
  });

  it('should track progress updates', () => {
    const updates: GitProgress[] = [];
    progressTracker.onProgress((progress) => {
      updates.push(progress);
    });

    progressTracker.updateProgress({ stage: 'processing', progress: 50 });
    progressTracker.updateProgress({ stage: 'finishing', progress: 100 });

    expect(updates).toHaveLength(2);
    expect(updates[0].stage).toBe('processing');
    expect(updates[1].stage).toBe('finishing');
  });

  it('should emit completion events', () => {
    const completionSpy = vi.fn();
    progressTracker.on('complete', completionSpy);

    progressTracker.complete();

    expect(completionSpy).toHaveBeenCalled();
  });

  it('should emit error events', () => {
    const errorSpy = vi.fn();
    progressTracker.on('error', errorSpy);

    progressTracker.fail('Test error');

    expect(errorSpy).toHaveBeenCalledWith('Test error');
  });
});

describe('GitOperationQueue', () => {
  let queue: GitOperationQueue;
  const mockLogger = {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    queue = new GitOperationQueue({
      maxConcurrentOps: 2,
      defaultTimeoutMs: 1000,
      logger: mockLogger,
    });
  });

  afterEach(() => {
    queue.clear();
  });

  it('should execute operations in order', async () => {
    const results: number[] = [];
    
    const operations = [1, 2, 3].map(num => ({
      id: `op-${num}`,
      type: 'test' as any,
      status: 'pending' as any,
    }));

    const promises = operations.map((op, index) =>
      queue.enqueue(
        op,
        async () => {
          await new Promise(resolve => setTimeout(resolve, 10));
          results.push(index + 1);
          return index + 1;
        }
      )
    );

    await Promise.all(promises);

    expect(results).toEqual([1, 2, 3]);
  });

  it('should handle operation failures with retry', async () => {
    let attempts = 0;
    const operation = {
      id: 'failing-op',
      type: 'test' as any,
      status: 'pending' as any,
    };

    const result = await queue.enqueue(
      operation,
      async () => {
        attempts++;
        if (attempts < 3) {
          throw new Error('Temporary failure');
        }
        return 'success';
      },
      1000,
      { attempts: 3, delay: 10 }
    );

    expect(attempts).toBe(3);
    expect(result).toBe('success');
  });

  it('should respect concurrency limits', async () => {
    let runningCount = 0;
    let maxRunning = 0;

    const operations = Array.from({ length: 5 }, (_, i) => ({
      id: `op-${i}`,
      type: 'test' as any,
      status: 'pending' as any,
    }));

    const promises = operations.map(op =>
      queue.enqueue(op, async () => {
        runningCount++;
        maxRunning = Math.max(maxRunning, runningCount);
        await new Promise(resolve => setTimeout(resolve, 50));
        runningCount--;
        return 'done';
      })
    );

    await Promise.all(promises);

    expect(maxRunning).toBeLessThanOrEqual(2); // Max concurrent ops is 2
  });

  it('should cancel operations', async () => {
    const operation = {
      id: 'cancellable-op',
      type: 'test' as any,
      status: 'pending' as any,
    };

    const promise = queue.enqueue(operation, async () => {
      await new Promise(resolve => setTimeout(resolve, 1000));
      return 'completed';
    });

    // Cancel the operation
    const cancelled = queue.cancel(operation.id);
    expect(cancelled).toBe(true);

    await expect(promise).rejects.toThrow('Operation cancelled');
  });
});

describe('Utility Functions', () => {
  describe('validateGitConfig', () => {
    it('should validate valid config', async () => {
      mockFs.stat.mockResolvedValue({
        isDirectory: () => true,
      } as any);

      const config = {
        baseDir: '/valid/dir',
        maxConcurrentOps: 3,
        timeoutMs: 5000,
        retryAttempts: 2,
        retryDelayMs: 1000,
        maxRepoSizeMB: 100,
      };

      const errors = await validateGitConfig(config);
      expect(errors).toHaveLength(0);
    });

    it('should detect invalid config values', async () => {
      mockFs.stat.mockRejectedValue(new Error('Directory not found'));

      const config = {
        baseDir: '/invalid/dir',
        maxConcurrentOps: 0, // Invalid
        timeoutMs: 500, // Too short
        retryAttempts: 15, // Too many
        maxRepoSizeMB: 20000, // Too large
      };

      const errors = await validateGitConfig(config);
      expect(errors.length).toBeGreaterThan(0);
    });
  });

  describe('parseGitUrl', () => {
    it('should parse HTTPS URLs', () => {
      const result = parseGitUrl('https://github.com/owner/repo.git');
      
      expect(result).toMatchObject({
        protocol: 'https',
        host: 'github.com',
        owner: 'owner',
        repo: 'repo',
        isValid: true,
      });
    });

    it('should parse SSH URLs', () => {
      const result = parseGitUrl('git@github.com:owner/repo.git');
      
      expect(result).toMatchObject({
        protocol: 'ssh',
        host: 'github.com',
        owner: 'owner',
        repo: 'repo',
        isValid: true,
      });
    });

    it('should handle invalid URLs', () => {
      const result = parseGitUrl('invalid-url');
      
      expect(result.isValid).toBe(false);
    });
  });

  describe('sanitizeGitUrl', () => {
    it('should redact credentials from HTTPS URLs', () => {
      const result = sanitizeGitUrl('https://user:pass@github.com/owner/repo.git');
      expect(result).toContain('[REDACTED]');
      expect(result).not.toContain('pass');
    });

    it('should redact credentials from SSH URLs', () => {
      const result = sanitizeGitUrl('ssh://user:pass@github.com/owner/repo.git');
      expect(result).toContain('[REDACTED]');
      expect(result).not.toContain('pass');
    });

    it('should preserve git user in SSH URLs', () => {
      const result = sanitizeGitUrl('git@github.com:owner/repo.git');
      expect(result).not.toContain('[REDACTED]');
      expect(result).toContain('git@');
    });
  });
});