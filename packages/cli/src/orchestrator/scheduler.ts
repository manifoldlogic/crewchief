import { RunManager } from './runManager'
import { getAgentType } from '../agents/registry'
import { busManager } from '../bus'
import { loadConfig } from '../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../git/worktrees'
import { TerminalProvider } from '../terminal/interface'

export class Scheduler {
  constructor(private terminal: TerminalProvider) {}

  async assignSingleAgent(task: string, agentTypeId: string): Promise<string> {
    const config = await loadConfig()
    const wt = new WorktreeService()

    // Ensure branch name is unique if the task is generic, but deterministic if task is specific
    const branchName = buildDeterministicBranchName({ agentTypeId, taskDescription: task })

    // Create worktree first
    const worktreePath = await wt.createWorktree(
      branchName,
      config.repository.mainBranch,
      config.repository.worktreeBasePath,
    )

    // Create window/pane via provider
    // We use the provider to create a window/pane. For iTerm this makes a visual pane.
    // For headless this spawns a process logically.
    const paneId = await this.terminal.createWindow({
      workingDirectory: worktreePath,
      title: `${agentTypeId}: ${task.slice(0, 20)}...`,
    })

    const type = getAgentType(agentTypeId)
    if (!type) throw new Error(`Unknown agent type: ${agentTypeId}`)

    const rm = new RunManager()
    const run = rm.createRun(agentTypeId, task, paneId, worktreePath, branchName)

    // Compute bus path for cross-process messaging
    const busPath = rm.getRunBusPath(run.id)

    // Start following BEFORE runCommand to avoid missing early messages
    // LogFollower handles file-not-yet-existing gracefully
    busManager.startFollowing(run.id, busPath)

    // Determine execution command
    const exec = type.executionCommand.includes('scripts/mock-agent.js')
      ? `node ${JSON.stringify(process.cwd() + '/scripts/mock-agent.js')}`
      : type.executionCommand

    // Run the agent in the terminal pane/process
    // Note: HeadlessProvider runCommand spawns a process. ITermProvider sends keys to running shell.
    // For iTerm, we need to cd first if not already in cwd (createWindow might handle it)
    // But let's be safe and chain it.
    // Headless provider's runCommand uses `child_process.spawn(cmd, {shell: true})`, so chaining works there too.
    // Pass CREWCHIEF_BUS_PATH to agent environment for cross-process messaging
    await this.terminal.runCommand(
      paneId,
      `cd ${JSON.stringify(worktreePath)} && CREWCHIEF_BUS_PATH=${JSON.stringify(busPath)} ${exec}`,
    )

    return run.id
  }
}
