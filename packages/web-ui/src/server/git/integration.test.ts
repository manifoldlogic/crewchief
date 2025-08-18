import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { GitService, createGitServiceOptions } from './index.js';
import { promises as fs } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe('GitService Integration Tests', () => {
  let testDir: string;
  let gitService: GitService;

  beforeAll(async () => {
    // Create a temporary test directory
    testDir = path.join(__dirname, 'test-repo');
    
    try {
      await fs.mkdir(testDir, { recursive: true });
      
      // Initialize a test git repository
      const { execFile } = await import('child_process');
      const { promisify } = await import('util');
      const exec = promisify(execFile);
      
      // Initialize git repo
      await exec('git', ['init'], { cwd: testDir });
      await exec('git', ['config', 'user.name', 'Test User'], { cwd: testDir });
      await exec('git', ['config', 'user.email', 'test@example.com'], { cwd: testDir });
      
      // Create initial commit
      await fs.writeFile(path.join(testDir, 'README.md'), '# Test Repository\n');
      await exec('git', ['add', 'README.md'], { cwd: testDir });
      await exec('git', ['commit', '-m', 'Initial commit'], { cwd: testDir });
      
      // Create GitService instance
      const options = createGitServiceOptions(testDir, {
        config: {
          maxConcurrentOps: 2,
          timeoutMs: 10000,
          retryAttempts: 1,
          enableProgressTracking: false, // Disable for simpler testing
        },
        security: {
          allowedProtocols: ['file:', 'https:', 'ssh:'],
          allowedHosts: [],
          validateSslCerts: false,
          sanitizeUrls: false,
        },
        logger: {
          info: () => {},
          warn: () => {},
          error: () => {},
          debug: () => {},
        },
      });
      
      gitService = new GitService(options);
    } catch (error) {
      console.warn('Git not available for integration tests:', error);
      // Mark test as skipped if git is not available
      return;
    }
  });

  afterAll(async () => {
    if (testDir) {
      try {
        gitService?.clearQueue();
        await fs.rm(testDir, { recursive: true, force: true });
      } catch (error) {
        console.warn('Failed to cleanup test directory:', error);
      }
    }
  });

  it('should get repository status', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    const status = await gitService.getStatus();
    
    expect(status).toBeDefined();
    expect(status.summary).toBeDefined();
    expect(typeof status.summary.current).toBe('string');
    expect(Array.isArray(status.files)).toBe(true);
  });

  it('should list branches', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    const branches = await gitService.listBranches();
    
    expect(Array.isArray(branches)).toBe(true);
    expect(branches.length).toBeGreaterThan(0);
    
    const mainBranch = branches.find(b => b.current);
    expect(mainBranch).toBeDefined();
    expect(mainBranch?.name).toMatch(/(main|master)/);
  });

  it('should get commit log', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    const commits = await gitService.getLog(5);
    
    expect(Array.isArray(commits)).toBe(true);
    expect(commits.length).toBeGreaterThan(0);
    
    const firstCommit = commits[0];
    expect(firstCommit.hash).toBeDefined();
    expect(firstCommit.message).toBe('Initial commit');
    expect(firstCommit.author.name).toBe('Test User');
  });

  it('should create and manage files', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    // Create a new file
    const testFile = path.join(testDir, 'test.txt');
    await fs.writeFile(testFile, 'Hello, World!\n');

    // Stage the file directly (this will also add it if it's new)
    await gitService.addFiles(['test.txt']);

    // Check status after staging
    const statusAfter = await gitService.getStatus();
    expect(statusAfter.summary.staged).toBeGreaterThan(0);

    // Commit the file
    const commitResult = await gitService.commit('Add test file');
    expect(commitResult.commit).toBeDefined();
    expect(commitResult.summary).toBeDefined();
  });

  it('should create and checkout branches', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    const testBranch = 'test-feature';

    // Create a new branch
    await gitService.createBranch(testBranch);

    // List branches to verify it was created
    const branches = await gitService.listBranches();
    const newBranch = branches.find(b => b.name === testBranch);
    expect(newBranch).toBeDefined();
    expect(newBranch?.current).toBe(true);

    // Switch back to main
    await gitService.checkoutBranch('main');

    // Verify we're back on main
    const branchesAfter = await gitService.listBranches();
    const mainBranch = branchesAfter.find(b => b.current);
    expect(mainBranch?.name).toMatch(/(main|master)/);

    // Delete the test branch
    await gitService.deleteBranch(testBranch);

    // Verify it was deleted
    const finalBranches = await gitService.listBranches();
    const deletedBranch = finalBranches.find(b => b.name === testBranch);
    expect(deletedBranch).toBeUndefined();
  });

  it('should get diff information', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    // Modify an existing file
    const readmePath = path.join(testDir, 'README.md');
    await fs.writeFile(readmePath, '# Test Repository\n\nThis is a test.\n');

    // Get diff
    const diff = await gitService.getDiff('README.md');
    
    expect(Array.isArray(diff)).toBe(true);
    
    if (diff.length > 0) {
      const chunk = diff[0];
      expect(chunk.oldStart).toBeDefined();
      expect(chunk.newStart).toBeDefined();
      expect(Array.isArray(chunk.lines)).toBe(true);
    }
  });

  it('should handle queue operations', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    // Get initial queue status
    const initialStatus = gitService.getQueueStatus();
    expect(initialStatus.queued).toBe(0);
    expect(initialStatus.running).toBe(0);

    // Start multiple operations
    const operations = [
      gitService.getStatus(),
      gitService.listBranches(),
      gitService.getLog(5),
    ];

    const results = await Promise.all(operations);
    expect(results).toHaveLength(3);

    // Queue should be empty after completion
    const finalStatus = gitService.getQueueStatus();
    expect(finalStatus.queued).toBe(0);
    expect(finalStatus.running).toBe(0);
  });

  it('should validate security constraints', async () => {
    if (!gitService) {
      console.log('Skipping test - git not available');
      return;
    }

    // Test path validation (should reject paths outside base directory)
    await expect(
      gitService.addFiles(['../../../etc/passwd'])
    ).rejects.toThrow('Invalid file path');

    // Test that normal files work
    await expect(
      gitService.addFiles(['.'])
    ).resolves.not.toThrow();
  });
});

describe('GitService Configuration Tests', () => {
  it('should create service with default configuration', () => {
    const baseDir = process.cwd();
    const options = createGitServiceOptions(baseDir);
    
    expect(options.config.baseDir).toBe(baseDir);
    expect(options.config.maxConcurrentOps).toBeDefined();
    expect(options.security.allowedProtocols).toContain('https:');
    expect(options.network.retryAttempts).toBeGreaterThan(0);
  });

  it('should create service with custom configuration', () => {
    const baseDir = '/custom/path';
    const options = createGitServiceOptions(baseDir, {
      config: {
        maxConcurrentOps: 5,
        timeoutMs: 60000,
      },
      security: {
        allowedProtocols: ['https:'],
        allowedHosts: ['github.com'],
      },
    });
    
    expect(options.config.baseDir).toBe(baseDir);
    expect(options.config.maxConcurrentOps).toBe(5);
    expect(options.config.timeoutMs).toBe(60000);
    expect(options.security.allowedProtocols).toEqual(['https:']);
    expect(options.security.allowedHosts).toEqual(['github.com']);
  });

  it('should create service instance', () => {
    const options = createGitServiceOptions(process.cwd());
    const gitService = new GitService(options);
    
    expect(gitService).toBeDefined();
    expect(typeof gitService.getStatus).toBe('function');
    expect(typeof gitService.createWorktree).toBe('function');
    expect(typeof gitService.getQueueStatus).toBe('function');
  });
});