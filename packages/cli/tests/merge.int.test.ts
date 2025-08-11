import { describe, it, expect, beforeAll } from 'vitest';
import path from 'node:path';
import fs from 'node:fs';
import simpleGit from 'simple-git';
import { GitMergeService } from '../src/git/merge';
import { initTempRepo } from './util';

describe('GitMergeService (integration)', () => {
  let dir = '';
  beforeAll(async () => {
    const r = await initTempRepo();
    dir = r.dir;
  });

  it('squash merges a feature branch into main', async () => {
    const git = simpleGit({ baseDir: dir });
    await git.checkoutLocalBranch('feature');
    fs.writeFileSync(path.join(dir, 'file.txt'), 'hello');
    await git.add(['.']);
    await git.commit('feat: add file');
    const svc = new GitMergeService(dir);
    const res = await svc.merge({ sourceBranch: 'feature', targetBranch: 'main', strategy: 'squash' });
    expect(res.success).toBe(true);
  });
});


