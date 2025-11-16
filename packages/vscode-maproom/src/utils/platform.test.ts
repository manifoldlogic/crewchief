/**
 * Tests for platform detection utilities
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import {
  detectPlatform,
  getBinaryExtension,
  isWindows,
  isMacOS,
  isLinux,
  getPathSeparator,
  PlatformError,
  type PlatformId,
} from './platform.js'

describe('platform utilities', () => {
  // Store original values to restore after tests
  const originalPlatform = process.platform
  const originalArch = process.arch

  afterEach(() => {
    // Restore original values
    Object.defineProperty(process, 'platform', { value: originalPlatform })
    Object.defineProperty(process, 'arch', { value: originalArch })
  })

  describe('detectPlatform', () => {
    it('should detect darwin-x64', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      const platform = detectPlatform()
      expect(platform).toBe('darwin-x64')
    })

    it('should detect darwin-arm64', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      Object.defineProperty(process, 'arch', { value: 'arm64' })

      const platform = detectPlatform()
      expect(platform).toBe('darwin-arm64')
    })

    it('should detect linux-x64', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      const platform = detectPlatform()
      expect(platform).toBe('linux-x64')
    })

    it('should detect linux-arm64', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 'arm64' })

      const platform = detectPlatform()
      expect(platform).toBe('linux-arm64')
    })

    it('should detect win32-x64', () => {
      Object.defineProperty(process, 'platform', { value: 'win32' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      const platform = detectPlatform()
      expect(platform).toBe('win32-x64')
    })

    it('should normalize amd64 architecture to x64', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 'amd64' })

      const platform = detectPlatform()
      expect(platform).toBe('linux-x64')
    })

    it('should normalize aarch64 architecture to arm64', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 'aarch64' })

      const platform = detectPlatform()
      expect(platform).toBe('linux-arm64')
    })

    it('should throw PlatformError for unsupported platform', () => {
      Object.defineProperty(process, 'platform', { value: 'freebsd' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      expect(() => detectPlatform()).toThrow(PlatformError)
      expect(() => detectPlatform()).toThrow(/Unsupported platform: freebsd/)
    })

    it('should throw PlatformError for unsupported architecture', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 'ia32' })

      expect(() => detectPlatform()).toThrow(PlatformError)
      expect(() => detectPlatform()).toThrow(/Unsupported architecture: ia32/)
    })

    it('should throw PlatformError with platform and arch details', () => {
      Object.defineProperty(process, 'platform', { value: 'sunos' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      try {
        detectPlatform()
        expect.fail('Should have thrown PlatformError')
      } catch (error) {
        expect(error).toBeInstanceOf(PlatformError)
        if (error instanceof PlatformError) {
          expect(error.platform).toBe('sunos')
          expect(error.arch).toBe('x64')
        }
      }
    })
  })

  describe('getBinaryExtension', () => {
    it('should return .exe for Windows', () => {
      Object.defineProperty(process, 'platform', { value: 'win32' })
      expect(getBinaryExtension()).toBe('.exe')
    })

    it('should return empty string for macOS', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      expect(getBinaryExtension()).toBe('')
    })

    it('should return empty string for Linux', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      expect(getBinaryExtension()).toBe('')
    })
  })

  describe('isWindows', () => {
    it('should return true on Windows', () => {
      Object.defineProperty(process, 'platform', { value: 'win32' })
      expect(isWindows()).toBe(true)
    })

    it('should return false on macOS', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      expect(isWindows()).toBe(false)
    })

    it('should return false on Linux', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      expect(isWindows()).toBe(false)
    })
  })

  describe('isMacOS', () => {
    it('should return true on macOS', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      expect(isMacOS()).toBe(true)
    })

    it('should return false on Windows', () => {
      Object.defineProperty(process, 'platform', { value: 'win32' })
      expect(isMacOS()).toBe(false)
    })

    it('should return false on Linux', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      expect(isMacOS()).toBe(false)
    })
  })

  describe('isLinux', () => {
    it('should return true on Linux', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      expect(isLinux()).toBe(true)
    })

    it('should return false on Windows', () => {
      Object.defineProperty(process, 'platform', { value: 'win32' })
      expect(isLinux()).toBe(false)
    })

    it('should return false on macOS', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      expect(isLinux()).toBe(false)
    })
  })

  describe('getPathSeparator', () => {
    it('should return backslash for Windows', () => {
      Object.defineProperty(process, 'platform', { value: 'win32' })
      expect(getPathSeparator()).toBe('\\')
    })

    it('should return forward slash for macOS', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      expect(getPathSeparator()).toBe('/')
    })

    it('should return forward slash for Linux', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      expect(getPathSeparator()).toBe('/')
    })
  })

  describe('PlatformError', () => {
    it('should create error with message, platform, and arch', () => {
      const error = new PlatformError('Test error', 'linux', 'ia32')
      expect(error.message).toBe('Test error')
      expect(error.platform).toBe('linux')
      expect(error.arch).toBe('ia32')
      expect(error.name).toBe('PlatformError')
    })

    it('should be instanceof Error', () => {
      const error = new PlatformError('Test error', 'darwin', 'x64')
      expect(error).toBeInstanceOf(Error)
    })
  })
})
