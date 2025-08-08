import { TmuxService } from '../tmux/tmux.service';
import { WorktreeService } from '../git/worktrees';

export async function safeClosePane(tmux: TmuxService, paneId: string): Promise<void> {
  try {
    tmux.closePane(paneId);
  } catch {
    // ignore
  }
}

export async function safeRemoveWorktree(wt: WorktreeService, worktreePath: string): Promise<void> {
  try {
    await wt.removeWorktree(worktreePath);
  } catch {
    // ignore
  }
}


