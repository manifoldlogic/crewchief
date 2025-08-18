import { getDatabaseService, PaginationArgs, SortArgs } from '../services/database.js';

interface Worktree {
  id: string;
  name: string;
  branch: string;
  path: string;
  status: string;
  created_at: Date;
  updated_at: Date;
  repo_id: string;
  current_branch: string;
  upstream_branch?: string;
  is_clean: boolean;
  is_synced: boolean;
  head_commit_sha: string;
  head_commit_message?: string;
  head_commit_author?: string;
  head_commit_date?: Date;
  commits_ahead: number;
  commits_behind: number;
  modified_files: number;
  added_files: number;
  deleted_files: number;
  untracked_files: number;
  staged_files: number;
  file_changes?: any;
  total_files: number;
  total_size_bytes: number;
  programming_languages?: any;
  active_agents?: any;
  tmux_sessions: string[];
  disk_usage_bytes: number;
  last_build_status?: string;
  last_build_time?: Date;
  test_status?: string;
  test_coverage?: number;
  maproom_indexed_at?: Date;
  maproom_index_status?: string;
  chunk_count: number;
  last_scan_at?: Date;
  scan_duration_ms?: number;
  cache_version: number;
  last_error?: string;
  error_count: number;
  last_accessed_at?: Date;
  pinned: boolean;
  tags: string[];
  notes?: string;
}

interface WorktreeFilterInput {
  status?: string;
  branch?: string;
  repoId?: string;
  isClean?: boolean;
  isSynced?: boolean;
  pinned?: boolean;
  tags?: string[];
  search?: string;
}

interface WorktreeCreateInput {
  name: string;
  branch: string;
  path: string;
  repoId: string;
  notes?: string;
  tags?: string[];
}

interface WorktreeUpdateInput {
  id: string;
  name?: string;
  branch?: string;
  path?: string;
  status?: string;
  pinned?: boolean;
  notes?: string;
  tags?: string[];
}

export const worktreeResolvers = {
  Worktree: {
    // Map database fields to GraphQL fields
    id: (parent: Worktree) => parent.id.toString(),
    repoId: (parent: Worktree) => parent.repo_id?.toString(),
    currentBranch: (parent: Worktree) => parent.current_branch,
    upstreamBranch: (parent: Worktree) => parent.upstream_branch,
    isClean: (parent: Worktree) => parent.is_clean,
    isSynced: (parent: Worktree) => parent.is_synced,
    headCommitSha: (parent: Worktree) => parent.head_commit_sha,
    headCommitMessage: (parent: Worktree) => parent.head_commit_message,
    headCommitAuthor: (parent: Worktree) => parent.head_commit_author,
    headCommitDate: (parent: Worktree) => parent.head_commit_date,
    commitsAhead: (parent: Worktree) => parent.commits_ahead,
    commitsBehind: (parent: Worktree) => parent.commits_behind,
    modifiedFiles: (parent: Worktree) => parent.modified_files,
    addedFiles: (parent: Worktree) => parent.added_files,
    deletedFiles: (parent: Worktree) => parent.deleted_files,
    untrackedFiles: (parent: Worktree) => parent.untracked_files,
    stagedFiles: (parent: Worktree) => parent.staged_files,
    fileChanges: (parent: Worktree) => parent.file_changes,
    totalFiles: (parent: Worktree) => parent.total_files,
    totalSizeBytes: (parent: Worktree) => parent.total_size_bytes,
    programmingLanguages: (parent: Worktree) => parent.programming_languages,
    activeAgents: (parent: Worktree) => parent.active_agents,
    tmuxSessions: (parent: Worktree) => parent.tmux_sessions || [],
    diskUsageBytes: (parent: Worktree) => parent.disk_usage_bytes,
    lastBuildStatus: (parent: Worktree) => parent.last_build_status,
    lastBuildTime: (parent: Worktree) => parent.last_build_time,
    testStatus: (parent: Worktree) => parent.test_status,
    testCoverage: (parent: Worktree) => parent.test_coverage,
    maproomIndexedAt: (parent: Worktree) => parent.maproom_indexed_at,
    maproomIndexStatus: (parent: Worktree) => parent.maproom_index_status,
    chunkCount: (parent: Worktree) => parent.chunk_count,
    lastScanAt: (parent: Worktree) => parent.last_scan_at,
    scanDurationMs: (parent: Worktree) => parent.scan_duration_ms,
    cacheVersion: (parent: Worktree) => parent.cache_version,
    lastError: (parent: Worktree) => parent.last_error,
    errorCount: (parent: Worktree) => parent.error_count,
    lastAccessedAt: (parent: Worktree) => parent.last_accessed_at,
    createdAt: (parent: Worktree) => parent.created_at,
    updatedAt: (parent: Worktree) => parent.updated_at,

    // Relations
    agents: async (parent: Worktree) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_runs WHERE worktree_id = ? ORDER BY created_at DESC',
        [parent.id]
      );
    },

    runs: async (parent: Worktree) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_runs WHERE worktree_id = ? ORDER BY started_at DESC',
        [parent.id]
      );
    },

    maproomIndex: async (parent: Worktree) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM maproom_worktrees WHERE id = ?',
        [parent.id]
      );
    },
  },

  Query: {
    worktree: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      const worktree = await db.executeQuerySingle(
        `SELECT ws.*, mw.indexed_at as maproom_indexed_at 
         FROM worktree_status ws 
         LEFT JOIN maproom_worktrees mw ON ws.worktree_id = mw.id 
         WHERE ws.id = ?`,
        [id]
      );
      return worktree;
    },

    worktrees: async (
      _: any,
      {
        filter,
        sort,
        pagination,
      }: {
        filter?: WorktreeFilterInput;
        sort?: SortArgs;
        pagination?: PaginationArgs;
      }
    ) => {
      const db = getDatabaseService();
      
      // Build dynamic filter for the connection
      const dbFilter: Record<string, any> = {};
      
      if (filter) {
        if (filter.status) dbFilter.state = filter.status;
        if (filter.branch) dbFilter.current_branch = filter.branch;
        if (filter.repoId) dbFilter.repo_id = filter.repoId;
        if (filter.isClean !== undefined) dbFilter.is_clean = filter.isClean;
        if (filter.isSynced !== undefined) dbFilter.is_synced = filter.isSynced;
        if (filter.pinned !== undefined) dbFilter.pinned = filter.pinned;
        if (filter.search) dbFilter.search = filter.search;
        if (filter.tags && filter.tags.length > 0) {
          // Handle array intersection for tags
          dbFilter.tags = filter.tags;
        }
      }

      return db.getConnection('worktree_status', dbFilter, sort, pagination);
    },

    worktreeByPath: async (_: any, { path }: { path: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM worktree_status WHERE worktree_path = ?',
        [path]
      );
    },

    worktreeByBranch: async (_: any, { branch, repoId }: { branch: string; repoId: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM worktree_status WHERE current_branch = ? AND repo_id = ?',
        [branch, repoId]
      );
    },
  },

  Mutation: {
    createWorktree: async (_: any, { input }: { input: WorktreeCreateInput }) => {
      const db = getDatabaseService();
      
      // Validate required fields
      const errors = db.validateRequired(input, ['name', 'branch', 'path', 'repoId']);
      if (errors.length > 0) {
        return db.createResponse(false, null, errors);
      }

      try {
        const worktree = await db.withTransaction(async (client) => {
          // Insert into worktree_status table
          const result = await client.query(
            `INSERT INTO worktree_status 
             (worktree_name, current_branch, worktree_path, repo_id, state, 
              is_clean, is_synced, commits_ahead, commits_behind, 
              modified_files, added_files, deleted_files, untracked_files, staged_files,
              total_files, total_size_bytes, disk_usage_bytes, chunk_count, 
              cache_version, error_count, pinned, tags, notes, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', true, true, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, false, $5, $6, NOW(), NOW())
             RETURNING *`,
            [input.name, input.branch, input.path, input.repoId, input.tags || [], input.notes]
          );
          return result.rows[0];
        });

        return db.createResponse(true, { worktree });
      } catch (error) {
        console.error('Error creating worktree:', error);
        return db.createResponse(false, null, [
          { message: 'Failed to create worktree', code: 'CREATE_FAILED' },
        ]);
      }
    },

    updateWorktree: async (_: any, { input }: { input: WorktreeUpdateInput }) => {
      const db = getDatabaseService();
      
      try {
        const worktree = await db.withTransaction(async (client) => {
          const updates: string[] = [];
          const values: any[] = [];
          let paramIndex = 1;

          if (input.name) {
            updates.push(`worktree_name = $${paramIndex++}`);
            values.push(input.name);
          }
          if (input.branch) {
            updates.push(`current_branch = $${paramIndex++}`);
            values.push(input.branch);
          }
          if (input.path) {
            updates.push(`worktree_path = $${paramIndex++}`);
            values.push(input.path);
          }
          if (input.status) {
            updates.push(`state = $${paramIndex++}`);
            values.push(input.status);
          }
          if (input.pinned !== undefined) {
            updates.push(`pinned = $${paramIndex++}`);
            values.push(input.pinned);
          }
          if (input.notes !== undefined) {
            updates.push(`notes = $${paramIndex++}`);
            values.push(input.notes);
          }
          if (input.tags) {
            updates.push(`tags = $${paramIndex++}`);
            values.push(input.tags);
          }

          updates.push(`updated_at = NOW()`);
          values.push(input.id);

          const result = await client.query(
            `UPDATE worktree_status SET ${updates.join(', ')} WHERE id = $${paramIndex} RETURNING *`,
            values
          );

          return result.rows[0];
        });

        if (!worktree) {
          return db.createResponse(false, null, [
            { message: 'Worktree not found', code: 'NOT_FOUND' },
          ]);
        }

        return db.createResponse(true, { worktree });
      } catch (error) {
        console.error('Error updating worktree:', error);
        return db.createResponse(false, null, [
          { message: 'Failed to update worktree', code: 'UPDATE_FAILED' },
        ]);
      }
    },

    deleteWorktree: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      
      try {
        await db.withTransaction(async (client) => {
          // Check if worktree has active agents or runs
          const activeAgents = await client.query(
            'SELECT COUNT(*) FROM agent_runs WHERE worktree_id = ? AND status IN (\'pending\', \'running\')',
            [id]
          );

          if (parseInt(activeAgents.rows[0].count) > 0) {
            throw new Error('Cannot delete worktree with active agents');
          }

          // Delete the worktree
          await client.query('DELETE FROM worktree_status WHERE id = ?', [id]);
        });

        return db.createResponse(true, { deletedId: id });
      } catch (error) {
        console.error('Error deleting worktree:', error);
        return db.createResponse(false, null, [
          { message: error instanceof Error ? error.message : 'Failed to delete worktree', code: 'DELETE_FAILED' },
        ]);
      }
    },

    refreshWorktreeStatus: async (_: any, { id }: { id: string }) => {
      // This would typically trigger a refresh of git status and file system info
      // For now, just update the last_accessed_at timestamp
      const db = getDatabaseService();
      
      try {
        const worktree = await db.executeQuerySingle(
          'UPDATE worktree_status SET last_accessed_at = NOW(), updated_at = NOW() WHERE id = ? RETURNING *',
          [id]
        );

        return db.createResponse(true, { worktree });
      } catch (error) {
        console.error('Error refreshing worktree status:', error);
        return db.createResponse(false, null, [
          { message: 'Failed to refresh worktree status', code: 'REFRESH_FAILED' },
        ]);
      }
    },

    toggleWorktreePin: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      
      try {
        const worktree = await db.executeQuerySingle(
          'UPDATE worktree_status SET pinned = NOT pinned, updated_at = NOW() WHERE id = ? RETURNING *',
          [id]
        );

        return db.createResponse(true, { worktree });
      } catch (error) {
        console.error('Error toggling worktree pin:', error);
        return db.createResponse(false, null, [
          { message: 'Failed to toggle worktree pin', code: 'TOGGLE_FAILED' },
        ]);
      }
    },
  },

  Subscription: {
    // Placeholder subscription resolvers - would need real-time implementation
    worktreeUpdated: {
      subscribe: () => {
        // Would implement with GraphQL subscriptions and PubSub
        throw new Error('Subscriptions not yet implemented');
      },
    },

    worktreeStatusChanged: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },
  },
};