import path from 'node:path';
import simpleGit, { SimpleGit } from 'simple-git';
import { ensureDirSync } from '../utils/fs';

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
}


