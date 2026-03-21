import fs from 'node:fs/promises'
import path from 'node:path'

/**
 * Index state of a worktree in the Maproom semantic search index.
 *
 * - "pending"  — worktree created but not yet scanned
 * - "indexed"  — scan completed successfully
 * - "stale"    — files changed since last scan; re-index recommended
 */
export type IndexState = 'indexed' | 'pending' | 'stale'

export interface WorktreeMetadata {
  sourceBranch: string
  createdAt: string
  createdFrom: string
  baseBranch: string
  purpose: 'agent' | 'manual'

  // --- Fields added by CRITIQUE.2004 ---

  /** Current Maproom index state for this worktree */
  index_state: IndexState
  /** ID of the currently-running (or last) agent run, if any */
  agent_run_id: string | null
  /** Platform name of the agent (e.g. "claude", "gemini"), if any */
  agent_platform: string | null
  /** Absolute path to the Maproom MCP Unix socket for this worktree */
  mcp_socket_path: string | null
  /** Human-readable description of the task assigned to the agent */
  task_description: string
}

/**
 * Derive the Maproom daemon Unix socket path for a given worktree.
 *
 * Convention: `/tmp/maproom-<uid>-<worktree-dir-name>.sock`
 *
 * The path is deterministic given the worktree directory path:
 * - The user's numeric UID scopes the socket to the current user (matching
 *   the global daemon convention `/tmp/maproom-<uid>.sock`).
 * - The worktree directory basename disambiguates sockets when multiple
 *   worktrees are active simultaneously.
 * - The `/tmp/` prefix keeps sockets out of the repository tree and avoids
 *   permissions issues on shared filesystems.
 *
 * Other modules (scheduler, MCP server, bus handlers) should import this
 * function rather than re-deriving the path independently.
 *
 * @param worktreePath - Absolute path to the worktree directory
 * @returns Absolute path to the Unix socket file
 */
export function deriveMaproomSocketPath(worktreePath: string): string {
  const uid = process.getuid?.() ?? 0
  const worktreeName = path.basename(worktreePath)
  return `/tmp/maproom-${uid}-${worktreeName}.sock`
}

/** Default values used when reading legacy metadata files that lack the new fields. */
const METADATA_DEFAULTS: Pick<
  WorktreeMetadata,
  'index_state' | 'agent_run_id' | 'agent_platform' | 'mcp_socket_path' | 'task_description'
> = {
  index_state: 'pending',
  agent_run_id: null,
  agent_platform: null,
  mcp_socket_path: null,
  task_description: '',
}

export class WorktreeMetadataService {
  private metadataFileName = '.crewchief-meta.json'

  async save(worktreePath: string, metadata: WorktreeMetadata): Promise<void> {
    const metadataPath = path.join(worktreePath, this.metadataFileName)
    await fs.writeFile(metadataPath, JSON.stringify(metadata, null, 2))
  }

  /**
   * Read worktree metadata, applying defaults for any fields missing from
   * legacy metadata files that predate the CRITIQUE.2004 schema extension.
   */
  async read(worktreePath: string): Promise<WorktreeMetadata | null> {
    try {
      const metadataPath = path.join(worktreePath, this.metadataFileName)
      const content = await fs.readFile(metadataPath, 'utf-8')
      const raw = JSON.parse(content) as Partial<WorktreeMetadata>
      // Merge defaults for backward compatibility with pre-CRITIQUE.2004 files
      return { ...METADATA_DEFAULTS, ...raw } as WorktreeMetadata
    } catch {
      return null
    }
  }

  /**
   * Partially update metadata fields without overwriting the entire file.
   * Reads the current metadata, merges the provided fields, and writes back.
   *
   * @param worktreePath - Absolute path to the worktree directory
   * @param fields - Partial metadata fields to update
   */
  async update(worktreePath: string, fields: Partial<WorktreeMetadata>): Promise<void> {
    const existing = await this.read(worktreePath)
    if (!existing) return
    const merged = { ...existing, ...fields }
    await this.save(worktreePath, merged)
  }

  async delete(worktreePath: string): Promise<void> {
    try {
      const metadataPath = path.join(worktreePath, this.metadataFileName)
      await fs.unlink(metadataPath)
    } catch {
      // Ignore errors if file doesn't exist
    }
  }
}
