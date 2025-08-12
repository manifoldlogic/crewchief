import path from 'node:path';
import simpleGit, { SimpleGit } from 'simple-git';
import { ensureDirSync, removeDirSync } from '../utils/fs';
import fs from 'node:fs';

export interface WorktreeListItem {
  path: string;
  branch?: string;
}

export class WorktreeService {
  private git: SimpleGit;
  private cwd: string;

  constructor(cwd: string = process.cwd()) {
    this.cwd = cwd;
    this.git = simpleGit({ baseDir: cwd });
  }

  async initRepository(storagePath: string): Promise<void> {
    ensureDirSync(path.join(this.cwd, storagePath));
  }

  async createWorktree(name: string, baseBranch: string, basePath: string): Promise<string> {
    const wtPath = path.join(this.cwd, basePath, name);
    ensureDirSync(wtPath);
    await this.git.fetch();
    await this.git.raw(['worktree', 'add', '-B', name, wtPath, baseBranch]);
    return wtPath;
  }

  async listWorktrees(): Promise<WorktreeListItem[]> {
    const out = await this.git.raw(['worktree', 'list', '--porcelain']);
    const lines = out.split('\n');
    const items: WorktreeListItem[] = [];
    let current: Partial<WorktreeListItem> = {};
    for (const line of lines) {
      if (line.startsWith('worktree ')) {
        if (current.path) items.push(current as WorktreeListItem);
        current = { path: line.replace('worktree ', '').trim() };
      } else if (line.startsWith('branch ')) {
        current.branch = line.replace('branch refs/heads/', '').trim();
      }
    }
    if (current.path) items.push(current as WorktreeListItem);
    return items;
  }

  async pruneWorktrees(opts?: { mode?: 'stale' | 'all'; keepDir?: boolean }): Promise<void> {
    if (!opts || opts.mode === 'stale') {
      await this.git.raw(['worktree', 'prune']);
      return;
    }
    if (opts.mode === 'all') {
      const list = await this.listWorktrees();
      // Use real paths so symlinks do not bypass the protection
      const cwdReal = safeRealpath(this.cwd);
      for (const item of list) {
        const p = path.resolve(item.path);
        const pReal = safeRealpath(p);
        // Skip if current working directory is the same as, or inside, this worktree
        const rel = path.relative(pReal, cwdReal);
        const isCwdInside = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
        if (isCwdInside) continue; // never remove current working tree
        try {
          // Force remove to handle unmerged/untracked files in the worktree
          await this.git.raw(['worktree', 'remove', '--force', p]);
          // Delete the directory unless --keep-dir was specified
          if (!opts.keepDir) {
            removeDirSync(p);
          }
        } catch {
          // ignore failures, continue best-effort
        }
      }
    }
  }

  async removeWorktree(worktreePath: string): Promise<void> {
    // Guard against deleting the current worktree (or its ancestor) even if asked
    const targetPath = path.resolve(worktreePath);
    const targetReal = safeRealpath(targetPath);
    const cwdReal = safeRealpath(this.cwd);
    const rel = path.relative(targetReal, cwdReal);
    const isCwdInsideTarget = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
    if (isCwdInsideTarget) {
      throw new Error('Refusing to remove the current worktree. Change directories and try again.');
    }
    await this.git.raw(['worktree', 'remove', '--force', targetPath]);
  }
}

function safeRealpath(p: string): string {
  try {
    return fs.realpathSync(p);
  } catch {
    return path.resolve(p);
  }
}

// Deterministic branch naming per spec: derive from agent id, optional task, and timestamp
export function buildDeterministicBranchName(params: {
  agentTypeId: string;
  taskDescription?: string;
  now?: Date;
}): string {
  const safeId = slugify(params.agentTypeId);
  const taskPart = params.taskDescription ? '-' + slugify(params.taskDescription).slice(0, 32) : '';
  const d = params.now ?? new Date();
  const ts =
    d.getUTCFullYear().toString() +
    pad2(d.getUTCMonth() + 1) +
    pad2(d.getUTCDate()) +
    pad2(d.getUTCHours()) +
    pad2(d.getUTCMinutes()) +
    pad2(d.getUTCSeconds());
  return `cc-${safeId}${taskPart}-${ts}`;
}

function pad2(n: number): string {
  return n < 10 ? `0${n}` : `${n}`;
}

function slugify(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .replace(/-{2,}/g, '-');
}


