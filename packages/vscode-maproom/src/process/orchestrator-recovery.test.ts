/**
 * Integration tests for ProcessOrchestrator crash recovery
 *
 * Tests that verify the crash recovery functionality is properly integrated
 */

import { describe, it, expect } from 'vitest'

describe('ProcessOrchestrator crash recovery integration', () => {
  describe('public API', () => {
    it('should export CrashRecovery class', async () => {
      const { CrashRecovery } = await import('./recovery.js')
      expect(CrashRecovery).toBeDefined()
      expect(typeof CrashRecovery).toBe('function')
    })
  })

  describe('recovery module', () => {
    it('should create CrashRecovery instances', async () => {
      const { CrashRecovery } = await import('./recovery.js')
      const recovery = new CrashRecovery()
      expect(recovery).toBeDefined()
      expect(recovery.getState()).toBe('CLOSED')
    })

    it('should provide handleCrash method', async () => {
      const { CrashRecovery } = await import('./recovery.js')
      const recovery = new CrashRecovery()
      expect(recovery.handleCrash).toBeDefined()
      expect(typeof recovery.handleCrash).toBe('function')
    })

    it('should provide reset method', async () => {
      const { CrashRecovery } = await import('./recovery.js')
      const recovery = new CrashRecovery()
      expect(recovery.reset).toBeDefined()
      expect(typeof recovery.reset).toBe('function')
    })

    it('should provide dispose method', async () => {
      const { CrashRecovery } = await import('./recovery.js')
      const recovery = new CrashRecovery()
      expect(recovery.dispose).toBeDefined()
      expect(typeof recovery.dispose).toBe('function')
    })
  })
})
