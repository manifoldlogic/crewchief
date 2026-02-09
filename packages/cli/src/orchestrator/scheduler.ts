import { RunManager } from './runManager'
import { resolveAgent } from '../agents/platforms'
import { busManager } from '../bus'
import { loadConfig } from '../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../git/worktrees'
import { TerminalProvider } from '../terminal/interface'

// ---------------------------------------------------------------------------
// SpawnOptions - configuration for the new spawnAgent() method
// ---------------------------------------------------------------------------

/**
 * Options controlling how an agent is spawned.
 *
 * By default (`useWorktree: false`), the agent runs in `process.cwd()` with
 * no configuration file required. Setting `useWorktree: true` loads the
 * CrewChief config and creates a dedicated git worktree for the agent.
 */
export interface SpawnOptions {
  /** When true, create a git worktree and load config. Default: false */
  useWorktree: boolean
  /** Agent name passed via --agent flag (e.g., 'code-review'). Optional. */
  agentName?: string
  /** Enable verbose logging. Optional. */
  verbose?: boolean
  /** Extra CLI arguments appended to the agent command. Optional. */
  extraArgs?: string
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Convert an arbitrary string to a URL/branch-safe slug.
 */
function slugify(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .replace(/-{2,}/g, '-')
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

export class Scheduler {
  constructor(
    private terminal: TerminalProvider,
    private runManager?: RunManager,
  ) {}

  // -------------------------------------------------------------------------
  // New method: spawnAgent (config-free by default)
  // -------------------------------------------------------------------------

  /**
   * Spawn an agent to work on a task.
   *
   * Branching logic:
   * - `useWorktree: false` (default) -- uses `process.cwd()`, no config needed
   * - `useWorktree: true` -- loads config, creates a git worktree
   *
   * @returns The run ID
   */
  async spawnAgent(task: string, platformName: string, options: SpawnOptions): Promise<string> {
    // 1. Resolve platform and agent
    const workingDirForAgentLookup = options.useWorktree ? undefined : process.cwd()
    const resolvedAgent = resolveAgent(platformName, options.agentName, workingDirForAgentLookup)

    // 2. Determine working directory (branch logic)
    let effectiveWorkingDir: string
    let branchName: string | null = null

    if (options.useWorktree) {
      const config = await loadConfig() // May throw if missing -- intentional
      const wt = new WorktreeService()
      const label = slugify(task)
      branchName = buildDeterministicBranchName({
        platform: platformName,
        taskDescription: label,
      })
      effectiveWorkingDir = await wt.createWorktree(
        branchName,
        config.repository.mainBranch,
        config.repository.worktreeBasePath,
      )
    } else {
      effectiveWorkingDir = process.cwd()
    }

    // 3. Generate label and create terminal window
    const label = `${slugify(task)}__${platformName}`
    const paneId = await this.terminal.createWindow({
      workingDirectory: effectiveWorkingDir,
      title: label,
      platform: platformName,
    })

    // 4. Record run with RunManager (7-param signature from AGENTDX.2005)
    const rm = this.runManager ?? new RunManager()
    const run = rm.createRun(
      platformName,
      task,
      paneId,
      effectiveWorkingDir,
      branchName,
      options.agentName || null,
      label,
    )

    // 5. Compute bus path for cross-process messaging
    const busPath = rm.getRunBusPath(run.id)

    // Start following BEFORE runCommand to avoid missing early messages
    busManager.startFollowing(run.id, busPath)

    // 6. Build and execute agent command
    let command = resolvedAgent.command
    if (options.extraArgs) {
      command = `${command} ${options.extraArgs}`
    }

    await this.terminal.runCommand(
      paneId,
      `cd ${JSON.stringify(effectiveWorkingDir)} && CREWCHIEF_BUS_PATH=${JSON.stringify(busPath)} ${command}`,
    )

    return run.id
  }
}
