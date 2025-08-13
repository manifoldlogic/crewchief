import { randomUUID } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { ensureDirSync, writeJsonSync, readJsonSync } from '../utils/fs'

export interface PersistedRun {
  id: string
  agentTypeId: string
  task: string
  paneId: string
  worktreePath: string
  branchName?: string
  status: 'running' | 'closed' | 'failed'
  startedAt: string
}

export class RunManager {
  private baseDir: string
  private stateFile: string

  constructor(baseDir = path.join(process.cwd(), '.crewchief', 'runs')) {
    this.baseDir = baseDir
    this.stateFile = path.join(this.baseDir, 'state.json')
    ensureDirSync(this.baseDir)
    if (!fs.existsSync(this.stateFile)) {
      writeJsonSync(this.stateFile, { runs: [] as PersistedRun[] })
    }
  }

  private loadAll(): PersistedRun[] {
    const data = readJsonSync<{ runs: PersistedRun[] }>(this.stateFile)
    return data.runs ?? []
  }

  private saveAll(runs: PersistedRun[]): void {
    writeJsonSync(this.stateFile, { runs })
  }

  createRun(
    agentTypeId: string,
    task: string,
    paneId: string,
    worktreePath: string,
    branchName?: string,
  ): PersistedRun {
    const run: PersistedRun = {
      id: randomUUID(),
      agentTypeId,
      task,
      paneId,
      worktreePath,
      branchName,
      status: 'running',
      startedAt: new Date().toISOString(),
    }
    const runs = this.loadAll()
    runs.push(run)
    this.saveAll(runs)
    ensureDirSync(this.getRunDir(run.id))
    return run
  }

  listRuns(): PersistedRun[] {
    return this.loadAll()
  }

  getRun(runId: string): PersistedRun | undefined {
    return this.loadAll().find((r) => r.id === runId)
  }

  getRunByAgentType(agentTypeId: string): PersistedRun | undefined {
    // Convenience for mock agent-id equal to type id
    return this.loadAll().find((r) => r.agentTypeId === agentTypeId && r.status === 'running')
  }

  updateRun(runId: string, patch: Partial<PersistedRun>): PersistedRun | undefined {
    const runs = this.loadAll()
    const idx = runs.findIndex((r) => r.id === runId)
    if (idx === -1) return undefined
    runs[idx] = { ...runs[idx], ...patch }
    this.saveAll(runs)
    return runs[idx]
  }

  getRunDir(runId: string): string {
    return path.join(this.baseDir, runId)
  }

  appendLog(runId: string, fileName: string, line: string): void {
    const dir = this.getRunDir(runId)
    ensureDirSync(dir)
    fs.appendFileSync(path.join(dir, fileName), line + '\n')
  }
}
