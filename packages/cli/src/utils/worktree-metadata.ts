import fs from 'node:fs/promises'
import path from 'node:path'

export interface WorktreeMetadata {
  sourceBranch: string
  createdAt: string
  createdFrom: string
  baseBranch: string
  purpose: 'agent' | 'manual'
}

export class WorktreeMetadataService {
  private metadataFileName = '.crewchief-meta.json'

  async save(worktreePath: string, metadata: WorktreeMetadata): Promise<void> {
    const metadataPath = path.join(worktreePath, this.metadataFileName)
    await fs.writeFile(metadataPath, JSON.stringify(metadata, null, 2))
  }

  async read(worktreePath: string): Promise<WorktreeMetadata | null> {
    try {
      const metadataPath = path.join(worktreePath, this.metadataFileName)
      const content = await fs.readFile(metadataPath, 'utf-8')
      return JSON.parse(content) as WorktreeMetadata
    } catch {
      return null
    }
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
