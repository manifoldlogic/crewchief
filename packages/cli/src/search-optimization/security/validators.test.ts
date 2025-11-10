import { describe, it, expect } from 'vitest'
import { validateVariantId } from './validators'

describe('Security Validators', () => {
  describe('validateVariantId', () => {
    it('accepts valid variant IDs', () => {
      expect(() => validateVariantId('variant-a-detailed')).not.toThrow()
      expect(() => validateVariantId('VARIANT_CONTROL')).not.toThrow()
      expect(() => validateVariantId('variant-123')).not.toThrow()
      expect(() => validateVariantId('a')).not.toThrow()
      expect(() => validateVariantId('ABC-123_xyz')).not.toThrow()
    })

    it('rejects path traversal attempts', () => {
      expect(() => validateVariantId('../etc/passwd')).toThrow('path traversal')
      expect(() => validateVariantId('variant/../etc')).toThrow('path traversal')
      expect(() => validateVariantId('..\\windows\\system32')).toThrow('path traversal')
      expect(() => validateVariantId('..')).toThrow('path traversal')
      expect(() => validateVariantId('variant/subdir')).toThrow('path traversal')
      expect(() => validateVariantId('variant\\subdir')).toThrow('path traversal')
    })

    it('rejects invalid characters', () => {
      expect(() => validateVariantId('variant@email.com')).toThrow('only alphanumeric')
      expect(() => validateVariantId('variant$money')).toThrow('only alphanumeric')
      expect(() => validateVariantId('variant<script>')).toThrow('only alphanumeric')
      expect(() => validateVariantId('variant;rm -rf')).toThrow('only alphanumeric')
      expect(() => validateVariantId('variant with spaces')).toThrow('only alphanumeric')
    })

    it('rejects too-long IDs', () => {
      const longId = 'a'.repeat(65)
      expect(() => validateVariantId(longId)).toThrow('max 64 characters')
    })

    it('rejects consecutive dashes or underscores', () => {
      expect(() => validateVariantId('variant--double')).toThrow('no consecutive dashes or underscores')
      expect(() => validateVariantId('variant__double')).toThrow('no consecutive dashes or underscores')
      expect(() => validateVariantId('variant---triple')).toThrow('no consecutive dashes or underscores')
    })

    it('accepts max length ID', () => {
      const maxId = 'a'.repeat(64)
      expect(() => validateVariantId(maxId)).not.toThrow()
    })
  })
})
