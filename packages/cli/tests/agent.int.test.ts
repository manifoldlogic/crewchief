import fs from 'node:fs'
import path from 'node:path'
import { describe, it, expect, beforeAll } from 'vitest'
import { WorktreeService } from '../src/git/worktrees'
import { RunManager } from '../src/orchestrator/runManager'
import { TmuxService } from '../src/tmux/tmux.service'

// These tests require tmux present; skip if not available
function hasTmux(): boolean {
  try {
    const { spawnSync } = require('node:child_process')
    return spawnSync('tmux', ['-V']).status === 0
  } catch {
    return false
  }
}

describe.skipIf(!hasTmux())('Agent lifecycle (integration)', () => {
  it('creates a run directory and logs on spawn via RunManager', async () => {
    const rm = new RunManager()
    const wt = new WorktreeService()
    const tmux = new TmuxService('crewchief')
    const dir = path.join(process.cwd(), '.crewchief')
    fs.existsSync(dir) || fs.mkdirSync(dir, { recursive: true })
    // Just validate RunManager baseline without actually spawning a real agent
    const run = rm.createRun('mock-agent', 'test', '%0', process.cwd(), 'branch')
    rm.appendLog(run.id, 'events.log', '{}')
    const runDir = rm.getRunDir(run.id)
    expect(fs.existsSync(runDir)).toBe(true)
  })
})
