import { WorktreeService } from '../git/worktrees'

export async function safeRemoveWorktree(wt: WorktreeService, worktreePath: string): Promise<void> {
  try {
    await wt.removeWorktree(worktreePath)
  } catch {
    // ignore
  }
}
