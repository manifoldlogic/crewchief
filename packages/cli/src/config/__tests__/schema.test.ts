import { describe, it, expect } from 'vitest'
import { WorktreeSchema, TerminalSchema, TmuxSchema } from '../schema'

describe('WorktreeSchema', () => {
  describe('autoScanOnWorktreeUse field', () => {
    it('accepts true value', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: true,
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.autoScanOnWorktreeUse).toBe(true)
      }
    })

    it('accepts false value', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: false,
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.autoScanOnWorktreeUse).toBe(false)
      }
    })

    it('defaults to false when field is omitted', () => {
      const result = WorktreeSchema.safeParse({})
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.autoScanOnWorktreeUse).toBe(false)
      }
    })

    it('defaults to false when field is undefined', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: undefined,
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.autoScanOnWorktreeUse).toBe(false)
      }
    })

    it('rejects string "true"', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: 'true',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
        expect(result.error.issues[0].received).toBe('string')
      }
    })

    it('rejects string "false"', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: 'false',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
      }
    })

    it('rejects empty string', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: '',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
      }
    })

    it('rejects number 1', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: 1,
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
        expect(result.error.issues[0].received).toBe('number')
      }
    })

    it('rejects number 0', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: 0,
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
        expect(result.error.issues[0].received).toBe('number')
      }
    })

    it('rejects null', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: null,
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
        expect(result.error.issues[0].received).toBe('null')
      }
    })

    it('rejects object', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: {},
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
        expect(result.error.issues[0].received).toBe('object')
      }
    })

    it('rejects array', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: [],
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
        expect(result.error.issues[0].received).toBe('array')
      }
    })

    it('rejects non-empty array', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: [true],
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues[0].code).toBe('invalid_type')
        expect(result.error.issues[0].expected).toBe('boolean')
        expect(result.error.issues[0].received).toBe('array')
      }
    })
  })

  describe('TypeScript type inference', () => {
    it('infers correct type from schema', () => {
      const parsed = WorktreeSchema.parse({
        autoScanOnWorktreeUse: true,
      })

      // TypeScript compiler will catch if type inference is broken
      const value: boolean = parsed.autoScanOnWorktreeUse
      expect(value).toBe(true)
    })

    it('infers default value type', () => {
      const parsed = WorktreeSchema.parse({})

      // TypeScript knows this is boolean due to .default(false)
      const value: boolean = parsed.autoScanOnWorktreeUse
      expect(value).toBe(false)
    })
  })

  describe('integration with other WorktreeSchema fields', () => {
    it('works with all fields populated', () => {
      const result = WorktreeSchema.safeParse({
        copyIgnoredFiles: ['.env', '.secret'],
        copyFromPath: '/custom/path',
        overwriteStrategy: 'backup',
        autoScanOnWorktreeUse: true,
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.copyIgnoredFiles).toEqual(['.env', '.secret'])
        expect(result.data.copyFromPath).toBe('/custom/path')
        expect(result.data.overwriteStrategy).toBe('backup')
        expect(result.data.autoScanOnWorktreeUse).toBe(true)
      }
    })

    it('applies default when other fields are present', () => {
      const result = WorktreeSchema.safeParse({
        copyFromPath: '/custom/path',
        overwriteStrategy: 'skip',
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.copyFromPath).toBe('/custom/path')
        expect(result.data.overwriteStrategy).toBe('skip')
        expect(result.data.autoScanOnWorktreeUse).toBe(false)
      }
    })

    it('validates autoScanOnWorktreeUse independently of other fields', () => {
      const result = WorktreeSchema.safeParse({
        copyFromPath: '/valid/path',
        autoScanOnWorktreeUse: 'invalid',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        // Should only have one error for autoScanOnWorktreeUse
        const autoScanErrors = result.error.issues.filter((issue) => issue.path.includes('autoScanOnWorktreeUse'))
        expect(autoScanErrors.length).toBeGreaterThan(0)
      }
    })
  })

  describe('edge cases', () => {
    it('handles explicit false with other defaults', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: false,
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.autoScanOnWorktreeUse).toBe(false)
        expect(result.data.copyFromPath).toBe('.')
        expect(result.data.overwriteStrategy).toBe('skip')
      }
    })

    it('handles explicit true with other defaults', () => {
      const result = WorktreeSchema.safeParse({
        autoScanOnWorktreeUse: true,
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.autoScanOnWorktreeUse).toBe(true)
        expect(result.data.copyFromPath).toBe('.')
        expect(result.data.overwriteStrategy).toBe('skip')
      }
    })
  })
})

describe('TmuxSchema', () => {
  it('accepts custom session name', () => {
    const result = TmuxSchema.safeParse({ sessionName: 'custom-session' })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.sessionName).toBe('custom-session')
    }
  })

  it('defaults sessionName to "crewchief"', () => {
    const result = TmuxSchema.safeParse({})
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.sessionName).toBe('crewchief')
    }
  })

  it('rejects non-string sessionName', () => {
    const result = TmuxSchema.safeParse({ sessionName: 123 })
    expect(result.success).toBe(false)
  })
})

describe('TerminalSchema', () => {
  describe('backend enum', () => {
    it('accepts "iterm" backend', () => {
      const result = TerminalSchema.safeParse({ backend: 'iterm' })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('iterm')
      }
    })

    it('accepts "tmux" backend', () => {
      const result = TerminalSchema.safeParse({ backend: 'tmux' })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('tmux')
      }
    })

    it('accepts "headless" backend', () => {
      const result = TerminalSchema.safeParse({ backend: 'headless' })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('headless')
      }
    })

    it('accepts "auto" backend', () => {
      const result = TerminalSchema.safeParse({ backend: 'auto' })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('auto')
      }
    })

    it('defaults backend to "auto"', () => {
      const result = TerminalSchema.safeParse({})
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('auto')
      }
    })

    it('rejects unknown backend values', () => {
      const result = TerminalSchema.safeParse({ backend: 'wezterm' })
      expect(result.success).toBe(false)
    })
  })

  describe('tmux field', () => {
    it('accepts tmux config with custom session name', () => {
      const result = TerminalSchema.safeParse({
        backend: 'tmux',
        tmux: { sessionName: 'my-session' },
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.tmux?.sessionName).toBe('my-session')
      }
    })

    it('defaults tmux sessionName when tmux object is empty', () => {
      const result = TerminalSchema.safeParse({
        backend: 'tmux',
        tmux: {},
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.tmux?.sessionName).toBe('crewchief')
      }
    })

    it('tmux field is optional', () => {
      const result = TerminalSchema.safeParse({ backend: 'tmux' })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.tmux).toBeUndefined()
      }
    })
  })

  describe('iterm field', () => {
    it('accepts iterm config with session name', () => {
      const result = TerminalSchema.safeParse({
        backend: 'iterm',
        iterm: { sessionName: 'my-iterm-session' },
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.iterm?.sessionName).toBe('my-iterm-session')
      }
    })

    it('iterm field is optional', () => {
      const result = TerminalSchema.safeParse({ backend: 'iterm' })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.iterm).toBeUndefined()
      }
    })
  })

  describe('backward compatibility', () => {
    it('existing iterm config continues working', () => {
      const result = TerminalSchema.safeParse({
        backend: 'iterm',
        iterm: { sessionName: 'crewchief' },
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('iterm')
        expect(result.data.iterm?.sessionName).toBe('crewchief')
      }
    })

    it('existing auto config continues working', () => {
      const result = TerminalSchema.safeParse({ backend: 'auto' })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('auto')
      }
    })

    it('empty config defaults correctly', () => {
      const result = TerminalSchema.safeParse({})
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('auto')
        expect(result.data.iterm).toBeUndefined()
        expect(result.data.tmux).toBeUndefined()
      }
    })

    it('iterm-only config (no tmux) still parses', () => {
      const result = TerminalSchema.safeParse({
        backend: 'iterm',
        iterm: { sessionName: 'test' },
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.backend).toBe('iterm')
        expect(result.data.tmux).toBeUndefined()
      }
    })
  })
})
