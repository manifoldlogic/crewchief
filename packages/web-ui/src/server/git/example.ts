/**
 * Example usage of GitService
 * 
 * This file demonstrates how to use the GitService class for various git operations
 * with proper error handling, progress tracking, and security considerations.
 */

import { GitService, createGitServiceOptions } from './index.js';
import type { GitProgress, WorktreeInfo, BranchInfo } from './types.js';

// ============================================================================
// Basic Setup
// ============================================================================

/**
 * Creates a GitService instance with secure defaults
 */
function createGitService(baseDir: string): GitService {
  const options = createGitServiceOptions(baseDir, {
    config: {
      maxConcurrentOps: 3,
      timeoutMs: 300000, // 5 minutes
      retryAttempts: 3,
      enableProgressTracking: true,
    },
    security: {
      allowedProtocols: ['https:', 'ssh:'],
      allowedHosts: ['github.com', 'gitlab.com', 'bitbucket.org'],
      sanitizeUrls: true,
      validateSslCerts: true,
    },
    logger: {
      info: (msg, meta) => console.log(`[GitService] ${msg}`, meta || ''),
      warn: (msg, meta) => console.warn(`[GitService] ${msg}`, meta || ''),
      error: (msg, meta) => console.error(`[GitService] ${msg}`, meta || ''),
      debug: (msg, meta) => console.debug(`[GitService] ${msg}`, meta || ''),
    },
  });

  return new GitService(options);
}

// ============================================================================
// Worktree Management Examples
// ============================================================================

/**
 * Example: Create and manage worktrees
 */
async function manageWorktrees(gitService: GitService) {
  console.log('=== Worktree Management ===');

  try {
    // List existing worktrees
    const existingWorktrees = await gitService.listWorktrees();
    console.log('Existing worktrees:', existingWorktrees);

    // Create a new worktree for feature development
    const worktreePath = './feature-worktree';
    const featureBranch = 'feature/new-feature';
    
    const progressCallback = (progress: GitProgress) => {
      console.log(`Progress: ${progress.stage} - ${progress.progress}%`);
    };

    const worktree = await gitService.createWorktree(
      worktreePath,
      featureBranch,
      progressCallback
    );
    
    console.log('Created worktree:', worktree);

    // Later: Remove the worktree when done
    // await gitService.removeWorktree(worktreePath);

    // Clean up orphaned worktrees
    const prunedPaths = await gitService.pruneWorktrees();
    if (prunedPaths.length > 0) {
      console.log('Pruned worktrees:', prunedPaths);
    }

  } catch (error) {
    console.error('Worktree operation failed:', error);
  }
}

/**
 * Example: Branch operations with error handling
 */
async function manageBranches(gitService: GitService) {
  console.log('=== Branch Management ===');

  try {
    // List all branches
    const branches = await gitService.listBranches(true); // Include remote branches
    console.log('Available branches:', branches.map(b => b.name));

    // Create a new feature branch
    const newBranch = 'feature/git-integration';
    await gitService.createBranch(newBranch, 'main');
    console.log(`Created branch: ${newBranch}`);

    // Switch to the new branch
    await gitService.checkoutBranch(newBranch);
    console.log(`Switched to branch: ${newBranch}`);

    // Later: Merge the branch back to main
    await gitService.checkoutBranch('main');
    const mergeResult = await gitService.mergeBranch(newBranch, { noFF: true });
    
    if (mergeResult.conflicts && mergeResult.conflicts.length > 0) {
      console.log('Merge conflicts detected:');
      mergeResult.conflicts.forEach(conflict => {
        console.log(`- ${conflict.file}: ${conflict.reason}`);
        if (conflict.sections) {
          conflict.sections.forEach(section => {
            console.log(`  ${section.type}: lines ${section.startLine}-${section.endLine}`);
          });
        }
      });
    } else {
      console.log('Merge completed successfully');
      
      // Clean up: delete the feature branch
      await gitService.deleteBranch(newBranch);
      console.log(`Deleted branch: ${newBranch}`);
    }

  } catch (error) {
    console.error('Branch operation failed:', error);
  }
}

// ============================================================================
// Commit and Push/Pull Examples
// ============================================================================

/**
 * Example: Make commits with proper staging
 */
async function makeCommits(gitService: GitService) {
  console.log('=== Commit Operations ===');

  try {
    // Check current status
    const status = await gitService.getStatus();
    console.log('Repository status:', status.summary);
    
    if (status.files.length > 0) {
      console.log('Modified files:');
      status.files.forEach(file => {
        const changes = [];
        if (file.staged) changes.push('staged');
        if (file.modified) changes.push('modified');
        if (file.created) changes.push('created');
        if (file.deleted) changes.push('deleted');
        console.log(`  ${file.path} (${changes.join(', ')})`);
      });
    }

    // Stage specific files
    const filesToCommit = ['src/server/git/service.ts', 'src/server/git/types.ts'];
    await gitService.addFiles(filesToCommit);
    console.log('Files staged for commit');

    // Make a commit
    const commitResult = await gitService.commit(
      'feat: implement git service with progress tracking\n\n- Add comprehensive GitService class\n- Include security validation\n- Support concurrent operations'
    );
    
    console.log('Commit created:', {
      hash: commitResult.commit,
      summary: commitResult.summary,
    });

  } catch (error) {
    console.error('Commit operation failed:', error);
  }
}

/**
 * Example: Push and pull with progress tracking
 */
async function syncWithRemote(gitService: GitService) {
  console.log('=== Remote Synchronization ===');

  const progressCallback = (progress: GitProgress) => {
    const percentage = progress.total 
      ? Math.round((progress.progress / progress.total) * 100)
      : progress.progress;
    console.log(`${progress.method}: ${progress.stage} ${percentage}%`);
    
    if (progress.remoteMessages && progress.remoteMessages.length > 0) {
      progress.remoteMessages.forEach(msg => console.log(`Remote: ${msg}`));
    }
  };

  try {
    // Fetch latest changes
    await gitService.fetch('origin', progressCallback);
    console.log('Fetch completed');

    // Pull changes to current branch
    const pullResult = await gitService.pull('origin', 'main', progressCallback);
    console.log('Pull result:', pullResult.summary);

    // Push local changes
    const pushResult = await gitService.push('origin', 'main', progressCallback);
    console.log('Push result:', pushResult.pushed);

  } catch (error) {
    console.error('Remote operation failed:', error);
    
    // Handle specific network errors
    if (error instanceof Error && error.message.includes('Network error')) {
      console.log('Network issue detected. Operations will be retried automatically.');
    }
  }
}

// ============================================================================
// Advanced Operations Examples
// ============================================================================

/**
 * Example: Clone a repository with progress tracking
 */
async function cloneRepository(gitService: GitService) {
  console.log('=== Repository Cloning ===');

  const repoUrl = 'https://github.com/example/repo.git';
  const targetPath = './cloned-repo';

  const progressCallback = (progress: GitProgress) => {
    console.log(`Clone progress: ${progress.stage} ${progress.progress}%`);
  };

  try {
    await gitService.clone(repoUrl, targetPath, {
      branch: 'main',
      depth: 1, // Shallow clone for faster download
      progressCallback,
    });
    
    console.log(`Repository cloned to: ${targetPath}`);

  } catch (error) {
    console.error('Clone failed:', error);
  }
}

/**
 * Example: Get detailed diff information
 */
async function analyzeDifferences(gitService: GitService) {
  console.log('=== Diff Analysis ===');

  try {
    // Get unstaged differences
    const unstagedDiff = await gitService.getDiff();
    console.log(`Unstaged changes: ${unstagedDiff.length} chunks`);

    // Get staged differences
    const stagedDiff = await gitService.getDiff(undefined, true);
    console.log(`Staged changes: ${stagedDiff.length} chunks`);

    // Analyze specific file
    const fileDiff = await gitService.getDiff('src/server/git/service.ts', false, 5);
    if (fileDiff.length > 0) {
      console.log('Changes in service.ts:');
      fileDiff.forEach((chunk, index) => {
        console.log(`  Chunk ${index + 1}: @@ -${chunk.oldStart},${chunk.oldLines} +${chunk.newStart},${chunk.newLines} @@`);
        console.log(`    ${chunk.lines.length} lines changed`);
        
        const addedLines = chunk.lines.filter(l => l.type === 'add').length;
        const deletedLines = chunk.lines.filter(l => l.type === 'delete').length;
        console.log(`    +${addedLines} -${deletedLines}`);
      });
    }

  } catch (error) {
    console.error('Diff analysis failed:', error);
  }
}

/**
 * Example: Get commit history with analysis
 */
async function analyzeHistory(gitService: GitService) {
  console.log('=== History Analysis ===');

  try {
    // Get recent commits
    const commits = await gitService.getLog(20);
    console.log(`Retrieved ${commits.length} commits`);

    // Analyze commit patterns
    const authors = new Map<string, number>();
    const commitsByDay = new Map<string, number>();

    commits.forEach(commit => {
      // Count commits by author
      const count = authors.get(commit.author.name) || 0;
      authors.set(commit.author.name, count + 1);

      // Count commits by day
      const day = commit.author.date.toISOString().split('T')[0];
      const dayCount = commitsByDay.get(day) || 0;
      commitsByDay.set(day, dayCount + 1);
    });

    console.log('Top contributors:');
    Array.from(authors.entries())
      .sort(([,a], [,b]) => b - a)
      .slice(0, 5)
      .forEach(([author, count]) => {
        console.log(`  ${author}: ${count} commits`);
      });

    console.log('Recent activity:');
    Array.from(commitsByDay.entries())
      .sort(([a], [b]) => b.localeCompare(a))
      .slice(0, 7)
      .forEach(([day, count]) => {
        console.log(`  ${day}: ${count} commits`);
      });

  } catch (error) {
    console.error('History analysis failed:', error);
  }
}

// ============================================================================
// Queue Management and Monitoring
// ============================================================================

/**
 * Example: Monitor operation queue
 */
async function monitorOperations(gitService: GitService) {
  console.log('=== Operation Monitoring ===');

  // Set up queue monitoring
  const monitorInterval = setInterval(() => {
    const status = gitService.getQueueStatus();
    if (status.queued > 0 || status.running > 0) {
      console.log(`Queue status: ${status.running} running, ${status.queued} queued, ${status.locks} locks`);
      
      status.operations.forEach(op => {
        const duration = op.startTime 
          ? Date.now() - op.startTime.getTime()
          : 0;
        console.log(`  ${op.type} (${op.status}) - ${duration}ms`);
      });
    }
  }, 1000);

  try {
    // Perform multiple operations concurrently
    const operations = [
      gitService.getStatus(),
      gitService.listBranches(),
      gitService.getLog(10),
    ];

    await Promise.all(operations);
    console.log('All operations completed');

  } catch (error) {
    console.error('Operation monitoring failed:', error);
  } finally {
    clearInterval(monitorInterval);
  }
}

/**
 * Example: Graceful error handling and recovery
 */
async function handleErrors(gitService: GitService) {
  console.log('=== Error Handling ===');

  try {
    // This might fail due to network issues
    await gitService.fetch('nonexistent-remote');
    
  } catch (error) {
    console.error('Expected error occurred:', error);
    
    // Check if operations are still working
    try {
      const status = await gitService.getStatus();
      console.log('Local operations still functional');
    } catch (localError) {
      console.error('Local operations also failing:', localError);
    }
  }

  // Clear failed operations from queue
  gitService.clearQueue();
  console.log('Queue cleared for fresh start');
}

// ============================================================================
// Main Example Function
// ============================================================================

/**
 * Main example demonstrating GitService usage
 */
export async function runGitServiceExample() {
  const baseDir = process.cwd(); // Use current directory
  const gitService = createGitService(baseDir);

  console.log('Starting GitService example...');
  console.log(`Working directory: ${baseDir}`);

  try {
    // Run examples in sequence
    await manageWorktrees(gitService);
    await manageBranches(gitService);
    await makeCommits(gitService);
    await syncWithRemote(gitService);
    await analyzeDifferences(gitService);
    await analyzeHistory(gitService);
    await monitorOperations(gitService);
    
    console.log('All examples completed successfully!');

  } catch (error) {
    console.error('Example failed:', error);
    await handleErrors(gitService);
  } finally {
    // Clean up
    gitService.clearQueue();
    console.log('GitService example finished');
  }
}

// ============================================================================
// Configuration Examples
// ============================================================================

/**
 * Example: Custom configuration for different environments
 */
export function createProductionGitService(baseDir: string): GitService {
  return new GitService(createGitServiceOptions(baseDir, {
    config: {
      maxConcurrentOps: 5,
      timeoutMs: 600000, // 10 minutes for production
      retryAttempts: 5,
      retryDelayMs: 2000,
      enableProgressTracking: false, // Disable for performance
    },
    security: {
      allowedProtocols: ['https:'], // Only HTTPS in production
      allowedHosts: ['github.com'], // Restrict to trusted hosts
      validateSslCerts: true,
      sanitizeUrls: true,
    },
    network: {
      retryAttempts: 5,
      retryDelayMs: 2000,
      timeoutMs: 60000,
      offlineDetection: true,
    },
  }));
}

/**
 * Example: Development configuration with debugging
 */
export function createDevelopmentGitService(baseDir: string): GitService {
  return new GitService(createGitServiceOptions(baseDir, {
    config: {
      maxConcurrentOps: 2,
      timeoutMs: 60000, // Shorter timeout for development
      retryAttempts: 2,
      enableProgressTracking: true,
    },
    security: {
      allowedProtocols: ['https:', 'ssh:', 'file:'], // Allow file:// for local testing
      allowedHosts: [], // Allow all hosts in development
      validateSslCerts: false, // Disable for self-signed certificates
    },
    logger: {
      info: (msg, meta) => console.log(`🔍 [Git] ${msg}`, meta || ''),
      warn: (msg, meta) => console.warn(`⚠️  [Git] ${msg}`, meta || ''),
      error: (msg, meta) => console.error(`❌ [Git] ${msg}`, meta || ''),
      debug: (msg, meta) => console.debug(`🐛 [Git] ${msg}`, meta || ''),
    },
  }));
}

// Run the example if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runGitServiceExample().catch(console.error);
}