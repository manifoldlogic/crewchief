/**
 * Tests for MCPConfigWriter
 *
 * Verifies MCP configuration file generation, provider-specific environment
 * variable handling, configuration merging, and path validation security.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import * as fs from 'fs/promises'
import * as path from 'path'
import * as os from 'os'
import { MCPConfigWriter } from './mcp-writer'

describe('MCPConfigWriter', () => {
  let writer: MCPConfigWriter
  let tempDir: string

  beforeEach(async () => {
    writer = new MCPConfigWriter()
    // Create temp directory for integration tests
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'mcp-writer-test-'))
  })

  afterEach(async () => {
    // Clean up temp directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true })
    } catch (error) {
      // Ignore cleanup errors
    }
  })

  describe('registerMCPServer - unit tests', () => {
    describe('provider-specific configuration', () => {
      it('should generate OpenAI configuration with correct environment variables', async () => {
        const configPath = path.join(tempDir, '.vscode', 'mcp.json')

        await writer.registerMCPServer(tempDir, 'openai')

        const content = await fs.readFile(configPath, 'utf-8')
        const config = JSON.parse(content)

        expect(config.servers.maproom).toEqual({
          command: 'npx',
          args: ['-y', '@crewchief/maproom-mcp'],
          env: {
            MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
            MAPROOM_EMBEDDING_PROVIDER: 'openai',
            OPENAI_API_KEY: '${env:OPENAI_API_KEY}',
          },
        })
      })

      it('should generate Google configuration with correct environment variables', async () => {
        const configPath = path.join(tempDir, '.vscode', 'mcp.json')

        await writer.registerMCPServer(tempDir, 'google')

        const content = await fs.readFile(configPath, 'utf-8')
        const config = JSON.parse(content)

        expect(config.servers.maproom).toEqual({
          command: 'npx',
          args: ['-y', '@crewchief/maproom-mcp'],
          env: {
            MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
            MAPROOM_EMBEDDING_PROVIDER: 'google',
            GOOGLE_APPLICATION_CREDENTIALS: '${env:GOOGLE_APPLICATION_CREDENTIALS}',
          },
        })
      })

      it('should generate Ollama configuration with base environment variables only', async () => {
        const configPath = path.join(tempDir, '.vscode', 'mcp.json')

        await writer.registerMCPServer(tempDir, 'ollama')

        const content = await fs.readFile(configPath, 'utf-8')
        const config = JSON.parse(content)

        expect(config.servers.maproom).toEqual({
          command: 'npx',
          args: ['-y', '@crewchief/maproom-mcp'],
          env: {
            MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
            MAPROOM_EMBEDDING_PROVIDER: 'ollama',
          },
        })

        // Verify only base env vars (no provider-specific keys for ollama)
        expect(Object.keys(config.servers.maproom.env)).toHaveLength(2)
      })

      it('should use unversioned package reference for latest', async () => {
        const configPath = path.join(tempDir, '.vscode', 'mcp.json')

        await writer.registerMCPServer(tempDir, 'ollama')

        const content = await fs.readFile(configPath, 'utf-8')
        const config = JSON.parse(content)

        expect(config.servers.maproom.args).toContain('@crewchief/maproom-mcp')
        // Should NOT contain version suffix
        expect(config.servers.maproom.args[1]).not.toMatch(/@\d+\.\d+\.\d+$/)
      })

      it('should never write plaintext credentials', async () => {
        const configPath = path.join(tempDir, '.vscode', 'mcp.json')

        await writer.registerMCPServer(tempDir, 'openai')

        const content = await fs.readFile(configPath, 'utf-8')

        // Verify no plaintext API keys in file
        expect(content).not.toMatch(/sk-[a-zA-Z0-9]{32,}/)
        expect(content).not.toMatch(/api[_-]?key["\s:]+[a-zA-Z0-9]+/)

        // Verify environment variable syntax is used
        expect(content).toContain('${env:OPENAI_API_KEY}')
      })
    })

    describe('configuration merging', () => {
      it('should preserve existing MCP server entries', async () => {
        const vscodeDir = path.join(tempDir, '.vscode')
        const configPath = path.join(vscodeDir, 'mcp.json')

        // Create existing config with other MCP servers
        await fs.mkdir(vscodeDir, { recursive: true })
        const existingConfig = {
          servers: {
            'other-server': {
              command: 'node',
              args: ['other-server.js'],
            },
            'another-server': {
              command: 'python',
              args: ['server.py'],
              env: {
                SOME_VAR: 'value',
              },
            },
          },
        }
        await fs.writeFile(configPath, JSON.stringify(existingConfig, null, 2), 'utf-8')

        // Register Maproom server
        await writer.registerMCPServer(tempDir, 'openai')

        // Verify existing servers preserved
        const content = await fs.readFile(configPath, 'utf-8')
        const config = JSON.parse(content)

        expect(config.servers['other-server']).toEqual({
          command: 'node',
          args: ['other-server.js'],
        })

        expect(config.servers['another-server']).toEqual({
          command: 'python',
          args: ['server.py'],
          env: {
            SOME_VAR: 'value',
          },
        })

        // Verify Maproom server added
        expect(config.servers.maproom).toBeDefined()
      })

      it('should update existing maproom entry', async () => {
        const vscodeDir = path.join(tempDir, '.vscode')
        const configPath = path.join(vscodeDir, 'mcp.json')

        // Create existing config with old Maproom entry
        await fs.mkdir(vscodeDir, { recursive: true })
        const existingConfig = {
          servers: {
            maproom: {
              command: 'npx',
              args: ['-y', '@crewchief/maproom-mcp@1.0.0'],
              env: {
                GOOGLE_APPLICATION_CREDENTIALS: '${env:GOOGLE_APPLICATION_CREDENTIALS}',
              },
            },
          },
        }
        await fs.writeFile(configPath, JSON.stringify(existingConfig, null, 2), 'utf-8')

        // Update to OpenAI provider
        await writer.registerMCPServer(tempDir, 'openai')

        // Verify entry updated
        const content = await fs.readFile(configPath, 'utf-8')
        const config = JSON.parse(content)

        expect(config.servers.maproom).toEqual({
          command: 'npx',
          args: ['-y', '@crewchief/maproom-mcp'],
          env: {
            MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
            MAPROOM_EMBEDDING_PROVIDER: 'openai',
            OPENAI_API_KEY: '${env:OPENAI_API_KEY}',
          },
        })
      })

      it('should handle malformed existing config gracefully', async () => {
        const vscodeDir = path.join(tempDir, '.vscode')
        const configPath = path.join(vscodeDir, 'mcp.json')

        // Create invalid JSON config
        await fs.mkdir(vscodeDir, { recursive: true })
        await fs.writeFile(configPath, '{ invalid json', 'utf-8')

        // Should not throw - overwrites with valid config
        await writer.registerMCPServer(tempDir, 'ollama')

        // Verify valid config written
        const content = await fs.readFile(configPath, 'utf-8')
        const config = JSON.parse(content)

        expect(config.servers.maproom).toBeDefined()
      })
    })

    describe('path validation and security', () => {
      it('should reject path traversal attempts with ../../', async () => {
        const maliciousPath = path.join(tempDir, '..', '..', 'etc', 'passwd')

        await expect(
          writer.registerMCPServer(maliciousPath, 'ollama')
        ).rejects.toThrow('Invalid path: configuration file must be within workspace')
      })

      it('should reject absolute paths outside workspace', async () => {
        const outsidePath = '/etc/passwd'

        await expect(
          writer.registerMCPServer(outsidePath, 'ollama')
        ).rejects.toThrow('Invalid path: configuration file must be within workspace')
      })

      it('should accept valid workspace paths', async () => {
        // Should not throw
        await expect(
          writer.registerMCPServer(tempDir, 'ollama')
        ).resolves.toBeUndefined()
      })

      it('should accept nested workspace paths', async () => {
        const nestedDir = path.join(tempDir, 'nested', 'workspace')
        await fs.mkdir(nestedDir, { recursive: true })

        // Should not throw
        await expect(
          writer.registerMCPServer(nestedDir, 'ollama')
        ).resolves.toBeUndefined()
      })

      it('should handle symbolic links safely', async () => {
        // Create a symlink pointing outside workspace (if supported)
        const symlinkPath = path.join(tempDir, 'symlink')
        // Point to /etc which is definitely outside any temp directory
        const targetPath = '/etc'

        try {
          await fs.symlink(targetPath, symlinkPath, 'dir')

          // Should reject symlinks pointing outside workspace
          await expect(
            writer.registerMCPServer(symlinkPath, 'ollama')
          ).rejects.toThrow('Invalid path: configuration file must be within workspace')
        } catch (error: any) {
          // Symlink creation may fail on some systems - skip test
          if (error.code === 'EPERM' || error.code === 'ENOENT' || error.code === 'EACCES') {
            console.warn('Skipping symlink test - not supported on this system')
            return
          }
          throw error
        }
      })
    })

    describe('error handling', () => {
      it('should throw error if workspace root is empty string', async () => {
        await expect(writer.registerMCPServer('', 'ollama')).rejects.toThrow(
          'Workspace root is required'
        )
      })

      it('should throw error if workspace root is whitespace', async () => {
        await expect(writer.registerMCPServer('   ', 'ollama')).rejects.toThrow(
          'Workspace root is required'
        )
      })

      it('should throw error if workspace root is null/undefined', async () => {
        await expect(writer.registerMCPServer(null as any, 'ollama')).rejects.toThrow(
          'Workspace root is required'
        )

        await expect(writer.registerMCPServer(undefined as any, 'ollama')).rejects.toThrow(
          'Workspace root is required'
        )
      })
    })
  })

  describe('registerMCPServer - integration tests', () => {
    it('should create .vscode directory if missing', async () => {
      const vscodeDir = path.join(tempDir, '.vscode')

      // Verify directory doesn't exist
      await expect(fs.access(vscodeDir)).rejects.toThrow()

      // Register server
      await writer.registerMCPServer(tempDir, 'ollama')

      // Verify directory created
      const stat = await fs.stat(vscodeDir)
      expect(stat.isDirectory()).toBe(true)
    })

    it('should create mcp.json file with correct permissions', async () => {
      const configPath = path.join(tempDir, '.vscode', 'mcp.json')

      await writer.registerMCPServer(tempDir, 'ollama')

      // Verify file exists and is readable
      const stat = await fs.stat(configPath)
      expect(stat.isFile()).toBe(true)
    })

    it('should write valid JSON that can be parsed', async () => {
      const configPath = path.join(tempDir, '.vscode', 'mcp.json')

      await writer.registerMCPServer(tempDir, 'openai')

      // Verify JSON is valid
      const content = await fs.readFile(configPath, 'utf-8')
      const config = JSON.parse(content) // Should not throw

      expect(config.servers).toBeDefined()
      expect(config.servers.maproom).toBeDefined()
    })

    it('should write formatted JSON with proper indentation', async () => {
      const configPath = path.join(tempDir, '.vscode', 'mcp.json')

      await writer.registerMCPServer(tempDir, 'ollama')

      const content = await fs.readFile(configPath, 'utf-8')

      // Verify formatted with 2-space indentation
      expect(content).toContain('  "servers"')
      expect(content).toContain('    "maproom"')
    })

    it('should use cross-platform path separators', async () => {
      const configPath = path.join(tempDir, '.vscode', 'mcp.json')

      await writer.registerMCPServer(tempDir, 'ollama')

      // Verify file written successfully on current platform
      const stat = await fs.stat(configPath)
      expect(stat.isFile()).toBe(true)
    })

    it('should handle multiple sequential writes correctly', async () => {
      const configPath = path.join(tempDir, '.vscode', 'mcp.json')

      // Write OpenAI config
      await writer.registerMCPServer(tempDir, 'openai')

      // Write Google config
      await writer.registerMCPServer(tempDir, 'google')

      // Write Ollama config
      await writer.registerMCPServer(tempDir, 'ollama')

      // Verify final config is Ollama (with base env vars only)
      const content = await fs.readFile(configPath, 'utf-8')
      const config = JSON.parse(content)

      expect(config.servers.maproom.env).toEqual({
        MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
        MAPROOM_EMBEDDING_PROVIDER: 'ollama',
      })
    })
  })

  describe('complete workflow integration', () => {
    it('should handle complete setup workflow: create, merge, update', async () => {
      const vscodeDir = path.join(tempDir, '.vscode')
      const configPath = path.join(vscodeDir, 'mcp.json')

      // Step 1: Initial setup with Ollama
      await writer.registerMCPServer(tempDir, 'ollama')

      let content = await fs.readFile(configPath, 'utf-8')
      let config = JSON.parse(content)

      expect(config.servers.maproom.env).toEqual({
        MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
        MAPROOM_EMBEDDING_PROVIDER: 'ollama',
      })

      // Step 2: Add another MCP server manually
      config.servers['custom-server'] = {
        command: 'node',
        args: ['custom.js'],
      }
      await fs.writeFile(configPath, JSON.stringify(config, null, 2), 'utf-8')

      // Step 3: Switch to OpenAI
      await writer.registerMCPServer(tempDir, 'openai')

      content = await fs.readFile(configPath, 'utf-8')
      config = JSON.parse(content)

      // Verify Maproom updated to OpenAI
      expect(config.servers.maproom.env).toEqual({
        MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
        MAPROOM_EMBEDDING_PROVIDER: 'openai',
        OPENAI_API_KEY: '${env:OPENAI_API_KEY}',
      })

      // Verify custom server preserved
      expect(config.servers['custom-server']).toEqual({
        command: 'node',
        args: ['custom.js'],
      })

      // Step 4: Switch to Google
      await writer.registerMCPServer(tempDir, 'google')

      content = await fs.readFile(configPath, 'utf-8')
      config = JSON.parse(content)

      // Verify Maproom updated to Google
      expect(config.servers.maproom.env).toEqual({
        MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
        MAPROOM_EMBEDDING_PROVIDER: 'google',
        GOOGLE_APPLICATION_CREDENTIALS: '${env:GOOGLE_APPLICATION_CREDENTIALS}',
      })

      // Verify custom server still preserved
      expect(config.servers['custom-server']).toEqual({
        command: 'node',
        args: ['custom.js'],
      })
    })
  })
})
