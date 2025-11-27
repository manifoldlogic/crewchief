/**
 * MCP Configuration Writer for Maproom MCP Server Registration
 *
 * Writes .vscode/mcp.json to register the Maproom MCP server with VS Code's MCP client.
 * Handles provider-specific environment variables and merges with existing configurations.
 *
 * Key features:
 * - Versioned package reference using @crewchief/maproom-mcp@VERSION
 * - Provider-specific environment variable mapping (OpenAI, Google, Ollama)
 * - Preserves existing MCP server configurations
 * - Path validation to prevent directory traversal attacks
 * - Cross-platform path handling
 *
 * Security:
 * - Never writes plaintext credentials
 * - Uses environment variable syntax: ${env:VAR_NAME}
 * - Validates all paths are within workspace
 */

import * as fs from 'fs/promises'
import * as path from 'path'
import { MAPROOM_MCP_VERSION } from '../constants'

/**
 * Supported embedding providers
 */
export type EmbeddingProvider = 'ollama' | 'openai' | 'google'

/**
 * MCP Server configuration entry
 */
interface MCPServerConfig {
  command: string
  args: string[]
  env?: Record<string, string>
}

/**
 * Complete MCP configuration file structure
 * Uses "servers" key per VS Code MCP specification
 */
interface MCPConfig {
  servers: Record<string, MCPServerConfig>
}

/**
 * Legacy MCP configuration (for migration)
 */
interface LegacyMCPConfig {
  mcpServers?: Record<string, MCPServerConfig>
  servers?: Record<string, MCPServerConfig>
}

/**
 * Editor type for MCP configuration
 */
export type EditorType = 'vscode' | 'cursor'

/**
 * MCP Configuration Writer
 *
 * Writes MCP configuration files to register the Maproom MCP server.
 * Supports both VS Code (.vscode/mcp.json) and Cursor (.cursor/mcp.json).
 */
export class MCPConfigWriter {
  /**
   * Register the Maproom MCP server in MCP configuration file
   *
   * Creates or updates the MCP configuration file to include the Maproom server.
   * Preserves any existing MCP server configurations.
   * Migrates legacy "mcpServers" key to "servers" if found.
   *
   * @param workspaceRoot - Absolute path to workspace root directory
   * @param provider - Embedding provider (determines environment variables)
   * @param editor - Editor type ('vscode' or 'cursor'), defaults to 'vscode'
   * @throws Error if workspaceRoot is invalid or path validation fails
   */
  async registerMCPServer(
    workspaceRoot: string,
    provider: EmbeddingProvider,
    editor: EditorType = 'vscode'
  ): Promise<void> {
    // Validate workspace root is provided
    if (!workspaceRoot || workspaceRoot.trim() === '') {
      throw new Error('Workspace root is required')
    }

    // Validate paths are within workspace BEFORE building paths (prevent directory traversal)
    const resolvedWorkspace = path.resolve(workspaceRoot)
    await this.validateWorkspaceRoot(resolvedWorkspace)

    // Build paths based on editor type
    const configDir = editor === 'cursor'
      ? path.join(workspaceRoot, '.cursor')
      : path.join(workspaceRoot, '.vscode')
    const configPath = path.join(configDir, 'mcp.json')

    // Final path validation
    this.validatePath(configPath, resolvedWorkspace)

    // Read and migrate existing configuration if it exists
    let existingServers: Record<string, MCPServerConfig> = {}

    try {
      const fileContent = await fs.readFile(configPath, 'utf-8')
      const parsed = JSON.parse(fileContent) as LegacyMCPConfig

      // Migrate from legacy "mcpServers" to "servers"
      if (parsed.mcpServers && !parsed.servers) {
        existingServers = parsed.mcpServers
      } else if (parsed.servers) {
        existingServers = parsed.servers
      }
    } catch (error: any) {
      // File doesn't exist or is invalid JSON - use empty config
      if (error.code !== 'ENOENT') {
        // Log parse errors but continue with empty config
        console.warn('Failed to parse existing mcp.json, will overwrite:', error.message)
      }
    }

    // Build Maproom server configuration
    const maproomConfig: MCPServerConfig = {
      command: 'npx',
      args: ['-y', `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`],
    }

    // Add provider-specific environment variables
    const env = this.buildEnvironment(provider)
    if (Object.keys(env).length > 0) {
      maproomConfig.env = env
    }

    // Merge with existing configuration (always use "servers" key)
    const updatedConfig: MCPConfig = {
      servers: {
        ...existingServers,
        maproom: maproomConfig,
      },
    }

    // Ensure config directory exists
    await fs.mkdir(configDir, { recursive: true })

    // Write configuration file
    const configJson = JSON.stringify(updatedConfig, null, 2)
    await fs.writeFile(configPath, configJson, 'utf-8')
  }

  /**
   * Register the Maproom MCP server for both VS Code and Cursor
   *
   * Convenience method that writes configuration for both editors.
   *
   * @param workspaceRoot - Absolute path to workspace root directory
   * @param provider - Embedding provider (determines environment variables)
   */
  async registerMCPServerForAllEditors(
    workspaceRoot: string,
    provider: EmbeddingProvider
  ): Promise<void> {
    await this.registerMCPServer(workspaceRoot, provider, 'vscode')
    await this.registerMCPServer(workspaceRoot, provider, 'cursor')
  }

  /**
   * Build environment variables for provider
   *
   * Returns environment variable mapping using VS Code's ${env:VAR_NAME} syntax.
   * Never includes plaintext credentials.
   *
   * @param provider - Embedding provider
   * @returns Environment variable mapping
   */
  private buildEnvironment(provider: EmbeddingProvider): Record<string, string> {
    const env: Record<string, string> = {
      // Always include database URL (required for MCP server) - uses SQLite
      MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
      // Always include provider selection
      MAPROOM_EMBEDDING_PROVIDER: provider,
    }

    // Add provider-specific credentials
    switch (provider) {
      case 'openai':
        env.OPENAI_API_KEY = '${env:OPENAI_API_KEY}'
        break
      case 'google':
        env.GOOGLE_APPLICATION_CREDENTIALS = '${env:GOOGLE_APPLICATION_CREDENTIALS}'
        break
      case 'ollama': {
        // Check if custom Ollama endpoint is configured
        try {
          // eslint-disable-next-line @typescript-eslint/no-var-requires
          const vscode = require('vscode') as typeof import('vscode')
          const config = vscode.workspace.getConfiguration('maproom')
          const endpoint = config.get('ollama.endpoint') as string | undefined
          if (endpoint && endpoint !== 'http://127.0.0.1:11434') {
            // Add /api/embed suffix for the Rust daemon
            env.MAPROOM_EMBEDDING_API_ENDPOINT = endpoint.replace(/\/?$/, '/api/embed')
          }
        } catch {
          // In test environment where vscode is not available - use defaults
        }
        break
      }
      default:
        // TypeScript should prevent this, but handle unknown providers gracefully
        const _exhaustive: never = provider
        throw new Error(`Unknown provider: ${_exhaustive}`)
    }

    return env
  }

  /**
   * Validate workspace root is a safe directory
   *
   * Checks that the workspace root doesn't point to sensitive system locations.
   * Prevents attempting to write to /etc, /usr, /System, etc.
   * Resolves symlinks to ensure they don't escape to dangerous locations.
   *
   * @param resolvedWorkspace - Resolved absolute workspace path
   * @throws Error if workspace root is invalid
   */
  private async validateWorkspaceRoot(resolvedWorkspace: string): Promise<void> {
    // Resolve symlinks to get the real path
    let realPath: string
    try {
      realPath = await fs.realpath(resolvedWorkspace)
    } catch (error: any) {
      // Path doesn't exist yet - use resolved path
      // This is OK for new workspaces
      if (error.code === 'ENOENT') {
        realPath = resolvedWorkspace
      } else {
        throw error
      }
    }

    // Prevent writing to system directories
    const dangerousPaths = ['/etc', '/usr', '/System', '/bin', '/sbin', '/var']

    for (const dangerous of dangerousPaths) {
      if (realPath.startsWith(dangerous)) {
        throw new Error('Invalid path: configuration file must be within workspace')
      }
    }

    // Prevent writing to root directory
    if (realPath === '/' || realPath === 'C:\\' || realPath === 'C:/') {
      throw new Error('Invalid path: configuration file must be within workspace')
    }
  }

  /**
   * Validate path is within workspace (prevent directory traversal)
   *
   * Ensures the resolved path starts with the workspace root.
   * Prevents attacks like ../../etc/passwd.
   *
   * @param targetPath - Path to validate
   * @param resolvedWorkspace - Resolved workspace root directory
   * @throws Error if path is outside workspace
   */
  private validatePath(targetPath: string, resolvedWorkspace: string): void {
    const resolvedTarget = path.resolve(targetPath)

    if (!resolvedTarget.startsWith(resolvedWorkspace)) {
      throw new Error('Invalid path: configuration file must be within workspace')
    }
  }
}
