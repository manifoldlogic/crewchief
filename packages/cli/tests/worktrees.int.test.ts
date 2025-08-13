import fs from 'node:fs'
import { describe, it, expect, beforeAll } from 'vitest'
import { initTempRepo } from './util'
import { WorktreeService } from '../src/git/worktrees'

describe('WorktreeService (integration)', () => {
  let repoDir = ''
  beforeAll(async () => {
    const { dir } = await initTempRepo()
    repoDir = dir
  })

  it('creates and lists worktrees', async () => {
    const wt = new WorktreeService(repoDir)
    const base = '.crewchief/worktrees'
    await wt.initRepository(base)
    const name = 'test-wt'
    const p = await wt.createWorktree(name, 'main', base)
    expect(fs.existsSync(p)).toBe(true)
    const list = await wt.listWorktrees()
    const target = fs.realpathSync(p)
    const has = list.some((w) => {
      try {
        return fs.realpathSync(w.path) === target
      } catch {
        return false
      }
    })
    expect(has).toBe(true)
  })
})
