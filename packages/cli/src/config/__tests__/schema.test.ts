import { describe, it, expect } from 'vitest'
import { WorktreeSchema } from '../schema'

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
