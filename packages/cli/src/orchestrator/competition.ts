import { randomUUID } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { evaluateAndMaybeMerge } from './autoMerge'
import { RunManager } from './runManager'
import { Scheduler } from './scheduler'
import { Task } from './task.types'
import { runDefaultChecks } from '../evaluation/checks'
import { ensureDirSync, writeJsonSync, readJsonSync } from '../utils/fs'

export interface CompetitionParticipant {
  agentId: string // agent type id
  runId?: string
  worktreePath?: string
  score?: number
}

export interface Competition {
  id: string
  task: Task
  participants: CompetitionParticipant[]
  winner?: string // agentId
  createdAt: string
  evaluatedAt?: string
}

export class CompetitionManager {
  private baseDir: string
  private stateFile: string

  constructor(baseDir = path.join(process.cwd(), '.crewchief', 'competitions')) {
    this.baseDir = baseDir
    this.stateFile = path.join(this.baseDir, 'state.json')
    ensureDirSync(this.baseDir)
    if (!fs.existsSync(this.stateFile)) {
      writeJsonSync(this.stateFile, { competitions: [] as Competition[] })
    }
  }

  private loadAll(): Competition[] {
    const data = readJsonSync<{ competitions: Competition[] }>(this.stateFile)
    return data.competitions ?? []
  }

  private saveAll(list: Competition[]): void {
    writeJsonSync(this.stateFile, { competitions: list })
  }

  private competitionPath(id: string): string {
    return path.join(this.baseDir, `${id}.json`)
  }

  get(id: string): Competition | undefined {
    const p = this.competitionPath(id)
    if (!fs.existsSync(p)) return undefined
    return readJsonSync<Competition>(p)
  }

  list(): Competition[] {
    return this.loadAll()
  }

  start(description: string, agentIds: string[]): Competition {
    const task: Task = {
      id: randomUUID(),
      description,
      requirements: [],
      acceptanceCriteria: [],
    }
    const comp: Competition = {
      id: randomUUID(),
      task,
      participants: agentIds.map((a) => ({ agentId: a })),
      createdAt: new Date().toISOString(),
    }
    const all = this.loadAll()
    all.push(comp)
    this.saveAll(all)
    writeJsonSync(this.competitionPath(comp.id), comp)
    return comp
  }

  async assign(compId: string): Promise<Competition> {
    const comp = this.get(compId)
    if (!comp) throw new Error('Competition not found')
    const scheduler = new Scheduler()
    for (const p of comp.participants) {
      if (!p.runId) {
        const assignment = await scheduler.assignSingleAgent(comp.task, p.agentId)
        p.runId = assignment.runId
        p.worktreePath = assignment.worktreeId
      }
    }
    writeJsonSync(this.competitionPath(comp.id), comp)
    return comp
  }

  async evaluate(compId: string): Promise<Competition> {
    const comp = this.get(compId)
    if (!comp) throw new Error('Competition not found')
    for (const p of comp.participants) {
      if (!p.runId || !p.worktreePath) continue
      const runDir = path.join(process.cwd(), '.crewchief', 'runs', p.runId)
      const summary = await runDefaultChecks(p.worktreePath, runDir)
      p.score = summary.score
    }
    // pick winner by highest score
    const sorted = [...comp.participants].filter((p) => typeof p.score === 'number').sort((a, b) => b.score! - a.score!)
    comp.winner = sorted[0]?.agentId
    comp.evaluatedAt = new Date().toISOString()
    writeJsonSync(this.competitionPath(comp.id), comp)
    return comp
  }

  async finalize(
    compId: string,
  ): Promise<{ competition: Competition; merged?: boolean; score?: number; reason?: string }> {
    const comp = this.get(compId)
    if (!comp) throw new Error('Competition not found')
    if (!comp.winner) return { competition: comp, merged: false, reason: 'no winner' }
    const _rm = new RunManager()
    const winner = comp.participants.find((p) => p.agentId === comp.winner)
    if (!winner?.runId) return { competition: comp, merged: false, reason: 'no winner run' }
    const result = await evaluateAndMaybeMerge(winner.runId)
    return { competition: comp, merged: result.merged, score: result.score, reason: result.reason }
  }
}
