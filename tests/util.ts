import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import simpleGit, { SimpleGit } from 'simple-git';

export function makeTempDir(prefix = 'cc-test-'): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

export async function initTempRepo(): Promise<{ dir: string; git: SimpleGit }> {
  const dir = makeTempDir();
  const git = simpleGit({ baseDir: dir });
  await git.init();
  await git.addConfig('user.email', 'test@example.com');
  await git.addConfig('user.name', 'Test User');
  fs.writeFileSync(path.join(dir, 'README.md'), '# temp\n');
  await git.add(['.']);
  await git.commit('init');
  const status = await git.status();
  if (status.current !== 'main') {
    try {
      await git.checkoutLocalBranch('main');
    } catch {
      await git.checkout('main');
    }
  }
  return { dir, git };
}


