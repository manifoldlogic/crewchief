/**
 * Tests for PreFlightValidator
 */

import type { ChildProcess } from 'child_process'
import { mkdir, writeFile, rm } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { PreFlightValidator } from './pre-flight-validator.js'
import type { VariantEnvironment } from './types.js'

// Store mock functions at module level
const mockConnect = vi.fn()
const mockQuery = vi.fn()
const mockEnd = vi.fn()

// Mock pg module
vi.mock('pg', () => {
  return {
    Client: vi.fn().mockImplementation(() => {
      return {
        connect: mockConnect,
        query: mockQuery,
        end: mockEnd,
      }
    }),
  }
})

// Mock child_process spawn
vi.mock('child_process', () => {
  return {
    spawn: vi.fn(),
  }
})

describe('PreFlightValidator', () => {
  let validator: PreFlightValidator
  let testDir: string

  beforeEach(async () => {
    validator = new PreFlightValidator('postgresql://test:test@localhost:5432/test')

    // Create a temporary directory for tests
    testDir = join(tmpdir(), `crewchief-test-${Date.now()}`)
    await mkdir(testDir, { recursive: true })
  })

  afterEach(async () => {
    // Clean up test directory
    try {
      await rm(testDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }

    // Reset all mock implementations and calls
    mockConnect.mockReset()
    mockQuery.mockReset()
    mockEnd.mockReset()
    vi.clearAllMocks()
  })

  describe('checkDatabaseConnection', () => {
    it('should return true for valid connection', async () => {
      mockConnect.mockResolvedValue(undefined)
      mockQuery.mockResolvedValue({ rows: [{ '?column?': 1 }] } as unknown as ReturnType<typeof mockQuery>)
      mockEnd.mockResolvedValue(undefined)

      const result = await validator.checkDatabaseConnection()

      expect(result).toBe(true)
      expect(mockConnect).toHaveBeenCalled()
      expect(mockQuery).toHaveBeenCalledWith('SELECT 1')
      expect(mockEnd).toHaveBeenCalled()
    })

    it('should return false for connection failure', async () => {
      mockConnect.mockRejectedValue(new Error('Connection refused'))

      const result = await validator.checkDatabaseConnection()

      expect(result).toBe(false)
      expect(mockConnect).toHaveBeenCalled()
    })

    it('should return false when database URL is not configured', async () => {
      const validatorNoUrl = new PreFlightValidator('')

      const result = await validatorNoUrl.checkDatabaseConnection()

      expect(result).toBe(false)
    })

    it('should handle query failure', async () => {
      mockConnect.mockResolvedValue(undefined)
      mockQuery.mockRejectedValue(new Error('Query failed'))

      const result = await validator.checkDatabaseConnection()

      expect(result).toBe(false)
    })
  })

  describe('verifyBaseBranchIndexed', () => {
    it('should return indexed=true when base branch has chunks', async () => {
      const { spawn } = await import('child_process')

      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(
                JSON.stringify({
                  worktrees: [
                    {
                      name: 'main',
                      worktree: 'main',
                      chunk_count: 150,
                    },
                  ],
                }),
              )
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }

      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const result = await validator.verifyBaseBranchIndexed('crewchief', 'main')

      expect(result.indexed).toBe(true)
      expect(result.chunkCount).toBe(150)
    })

    it('should return indexed=false when base branch not in database', async () => {
      const { spawn } = await import('child_process')

      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(
                JSON.stringify({
                  worktrees: [],
                }),
              )
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }

      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const result = await validator.verifyBaseBranchIndexed('crewchief', 'main')

      expect(result.indexed).toBe(false)
      expect(result.chunkCount).toBe(0)
    })

    it('should return indexed=false when maproom command fails', async () => {
      const { spawn } = await import('child_process')

      const mockChild = {
        stdout: {
          on: vi.fn(),
        },
        stderr: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback('Command failed')
            }
          }),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(1)
          }
        }),
      }

      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const result = await validator.verifyBaseBranchIndexed('crewchief', 'main')

      expect(result.indexed).toBe(false)
      expect(result.chunkCount).toBe(0)
    })
  })

  describe('checkWorktreeScanned', () => {
    it('should pass when worktree has chunks indexed', async () => {
      const { spawn } = await import('child_process')

      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(
                JSON.stringify({
                  worktrees: [
                    {
                      name: 'test-worktree',
                      worktree: 'test-worktree',
                      chunk_count: 50,
                    },
                  ],
                }),
              )
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }

      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const result = await validator.checkWorktreeScanned('crewchief', 'test-worktree')

      expect(result.passed).toBe(true)
      expect(result.message).toContain('50 chunks')
      expect(result.details.chunkCount).toBe(50)
    })

    it('should fail when worktree has 0 chunks', async () => {
      const { spawn } = await import('child_process')

      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(
                JSON.stringify({
                  worktrees: [
                    {
                      name: 'test-worktree',
                      worktree: 'test-worktree',
                      chunk_count: 0,
                    },
                  ],
                }),
              )
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }

      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const result = await validator.checkWorktreeScanned('crewchief', 'test-worktree')

      expect(result.passed).toBe(false)
      expect(result.message).toContain('0 chunks')
    })

    it('should fail when worktree not in database', async () => {
      const { spawn } = await import('child_process')

      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(
                JSON.stringify({
                  worktrees: [],
                }),
              )
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }

      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const result = await validator.checkWorktreeScanned('crewchief', 'test-worktree')

      expect(result.passed).toBe(false)
      expect(result.message).toContain('not in database')
    })
  })

  describe('checkMcpConfigValid', () => {
    it('should pass for valid .mcp.json with maproom server', async () => {
      const mcpConfigPath = join(testDir, '.mcp.json')
      const validConfig = {
        mcpServers: {
          maproom: {
            command: 'node',
            args: ['bin/maproom.js'],
          },
        },
      }

      await writeFile(mcpConfigPath, JSON.stringify(validConfig, null, 2))

      const result = await validator.checkMcpConfigValid(testDir)

      expect(result.passed).toBe(true)
      expect(result.message).toContain('valid')
    })

    it('should fail when .mcp.json is missing', async () => {
      const result = await validator.checkMcpConfigValid(testDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('Missing .mcp.json')
    })

    it('should fail when .mcp.json has invalid JSON', async () => {
      const mcpConfigPath = join(testDir, '.mcp.json')
      await writeFile(mcpConfigPath, '{ invalid json }')

      const result = await validator.checkMcpConfigValid(testDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('Invalid JSON')
    })

    it('should fail when mcpServers is missing', async () => {
      const mcpConfigPath = join(testDir, '.mcp.json')
      await writeFile(mcpConfigPath, JSON.stringify({}))

      const result = await validator.checkMcpConfigValid(testDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('Missing mcpServers')
    })

    it('should fail when maproom server is missing', async () => {
      const mcpConfigPath = join(testDir, '.mcp.json')
      await writeFile(
        mcpConfigPath,
        JSON.stringify({
          mcpServers: {
            otherServer: {},
          },
        }),
      )

      const result = await validator.checkMcpConfigValid(testDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('Missing maproom server')
    })

    it('should fail when command field is missing', async () => {
      const mcpConfigPath = join(testDir, '.mcp.json')
      await writeFile(
        mcpConfigPath,
        JSON.stringify({
          mcpServers: {
            maproom: {
              args: ['test'],
            },
          },
        }),
      )

      const result = await validator.checkMcpConfigValid(testDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('Missing command')
    })

    it('should fail when args field is missing or invalid', async () => {
      const mcpConfigPath = join(testDir, '.mcp.json')
      await writeFile(
        mcpConfigPath,
        JSON.stringify({
          mcpServers: {
            maproom: {
              command: 'node',
            },
          },
        }),
      )

      const result = await validator.checkMcpConfigValid(testDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('args')
    })
  })

  describe('checkFilePermissions', () => {
    it('should pass when directory has read/write permissions', async () => {
      const result = await validator.checkFilePermissions(testDir)

      expect(result.passed).toBe(true)
      expect(result.message).toContain('Read/write permissions OK')
    })

    it('should fail when directory does not exist', async () => {
      const nonExistentDir = join(testDir, 'non-existent')

      const result = await validator.checkFilePermissions(nonExistentDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('Cannot read')
    })

    it('should handle read-only directories', async () => {
      // Create package.json to test read access
      await writeFile(join(testDir, 'package.json'), '{}')

      // We can't easily make a directory read-only in tests without sudo
      // So we'll just verify the test completes successfully
      const result = await validator.checkFilePermissions(testDir)

      expect(result.passed).toBe(true)
    })
  })

  describe('validateVariantEnvironment', () => {
    it('should pass when all checks pass', async () => {
      // Setup valid environment
      const mcpConfigPath = join(testDir, '.mcp.json')
      const validConfig = {
        mcpServers: {
          maproom: {
            command: 'node',
            args: ['bin/maproom.js'],
          },
        },
      }
      await writeFile(mcpConfigPath, JSON.stringify(validConfig, null, 2))

      // Mock spawn for worktree checks
      const { spawn } = await import('child_process')
      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(
                JSON.stringify({
                  worktrees: [
                    {
                      name: 'test-worktree',
                      worktree: 'test-worktree',
                      chunk_count: 50,
                    },
                  ],
                }),
              )
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }
      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const env: VariantEnvironment = {
        variantId: 'variant-1',
        worktreePath: testDir,
        repo: 'crewchief',
        worktree: 'test-worktree',
      }

      const result = await validator.validateVariantEnvironment(env)

      expect(result.overall).toBe('pass')
      expect(result.variantId).toBe('variant-1')
      expect(result.failureReason).toBeUndefined()
    })

    it('should fail when worktree is not scanned', async () => {
      // Setup valid MCP config but no worktree scan
      const mcpConfigPath = join(testDir, '.mcp.json')
      const validConfig = {
        mcpServers: {
          maproom: {
            command: 'node',
            args: ['bin/maproom.js'],
          },
        },
      }
      await writeFile(mcpConfigPath, JSON.stringify(validConfig, null, 2))

      // Mock spawn to return empty worktrees
      const { spawn } = await import('child_process')
      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(JSON.stringify({ worktrees: [] }))
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }
      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const env: VariantEnvironment = {
        variantId: 'variant-1',
        worktreePath: testDir,
        repo: 'crewchief',
        worktree: 'test-worktree',
      }

      const result = await validator.validateVariantEnvironment(env)

      expect(result.overall).toBe('fail')
      expect(result.failureReason).toContain('not scanned')
    })

    it('should fail when MCP config is invalid', async () => {
      // No MCP config file

      // Mock spawn to return valid worktrees
      const { spawn } = await import('child_process')
      const mockChild = {
        stdout: {
          on: vi.fn((event, callback) => {
            if (event === 'data') {
              callback(
                JSON.stringify({
                  worktrees: [
                    {
                      name: 'test-worktree',
                      worktree: 'test-worktree',
                      chunk_count: 50,
                    },
                  ],
                }),
              )
            }
          }),
        },
        stderr: {
          on: vi.fn(),
        },
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            callback(0)
          }
        }),
      }
      vi.mocked(spawn).mockReturnValue(mockChild as unknown as ChildProcess)

      const env: VariantEnvironment = {
        variantId: 'variant-1',
        worktreePath: testDir,
        repo: 'crewchief',
        worktree: 'test-worktree',
      }

      const result = await validator.validateVariantEnvironment(env)

      expect(result.overall).toBe('fail')
      expect(result.failureReason).toContain('Invalid MCP configuration')
    })
  })

  describe('validateCompetitionSetup', () => {
    it('should fail when database connection fails', async () => {
      mockConnect.mockRejectedValue(new Error('Connection refused'))

      const config = {
        task: {},
        variants: [],
      }

      const result = await validator.validateCompetitionSetup(config)

      expect(result.valid).toBe(false)
      expect(result.errors).toHaveLength(1)
      expect(result.errors[0].check).toBe('database_connection')
      expect(result.errors[0].troubleshooting).toContain('PostgreSQL')
    })

    it('should pass when database connection succeeds', async () => {
      mockConnect.mockResolvedValue(undefined)
      mockQuery.mockResolvedValue({ rows: [{ '?column?': 1 }] } as unknown as ReturnType<typeof mockQuery>)
      mockEnd.mockResolvedValue(undefined)

      const config = {
        task: {},
        variants: [],
      }

      const result = await validator.validateCompetitionSetup(config)

      expect(result.valid).toBe(true)
      expect(result.errors).toHaveLength(0)
    })
  })
})
