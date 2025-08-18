import { getDatabaseService } from '../services/database.js';

export const runResolvers = {
  Run: {
    id: (parent: any) => parent.id?.toString(),
    agentId: (parent: any) => parent.agent_id?.toString(),
    worktreeId: (parent: any) => parent.worktree_id?.toString(),
    repoId: (parent: any) => parent.repo_id?.toString(),
    runId: (parent: any) => parent.run_id,
    parentRunId: (parent: any) => parent.parent_run_id,
    commitSha: (parent: any) => parent.commit_sha,
    taskDescription: (parent: any) => parent.task_description,
    taskType: (parent: any) => parent.task_type,
    contextFiles: (parent: any) => parent.context_files || [],
    startedAt: (parent: any) => parent.started_at,
    completedAt: (parent: any) => parent.completed_at,
    durationMs: (parent: any) => parent.duration_ms,
    result: (parent: any) => parent.artifacts,
    createdAt: (parent: any) => parent.created_at,
    updatedAt: (parent: any) => parent.updated_at,
    tags: (parent: any) => parent.tags || [],
    bookmarked: (parent: any) => parent.bookmarked || false,

    // Computed fields
    duration: (parent: any) => {
      if (parent.duration_ms) {
        return `${Math.round(parent.duration_ms / 1000)}s`;
      }
      return null;
    },
    isRunning: (parent: any) => parent.status === 'running',
    isCompleted: (parent: any) => parent.status === 'completed',
    isFailed: (parent: any) => parent.status === 'failed',
    performanceGrade: (parent: any) => {
      if (parent.evaluation_score >= 0.9) return 'A';
      if (parent.evaluation_score >= 0.8) return 'B';
      if (parent.evaluation_score >= 0.7) return 'C';
      if (parent.evaluation_score >= 0.6) return 'D';
      return 'F';
    },

    // Relations
    agent: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_runs WHERE agent_id = ? LIMIT 1',
        [parent.agent_id]
      );
    },

    worktree: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM worktree_status WHERE id = ?',
        [parent.worktree_id]
      );
    },

    messages: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_messages WHERE run_id = ? ORDER BY created_at DESC',
        [parent.run_id]
      );
    },
  },

  Query: {
    run: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_runs WHERE id = ?',
        [id]
      );
    },

    runs: async (_: any, { filter, sort, pagination }: any) => {
      const db = getDatabaseService();
      const dbFilter: Record<string, any> = {};
      
      if (filter) {
        if (filter.status) dbFilter.status = filter.status;
        if (filter.agentId) dbFilter.agent_id = filter.agentId;
        if (filter.worktreeId) dbFilter.worktree_id = filter.worktreeId;
        if (filter.bookmarked !== undefined) dbFilter.bookmarked = filter.bookmarked;
        if (filter.search) dbFilter.search = filter.search;
      }

      return db.getConnection('agent_runs', dbFilter, sort, pagination);
    },

    runByRunId: async (_: any, { runId }: { runId: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_runs WHERE run_id = ?',
        [runId]
      );
    },

    recentRuns: async (_: any, { limit = 10, agentId }: any) => {
      const db = getDatabaseService();
      const query = agentId
        ? 'SELECT * FROM agent_runs WHERE agent_id = ? ORDER BY started_at DESC LIMIT ?'
        : 'SELECT * FROM agent_runs ORDER BY started_at DESC LIMIT ?';
      const params = agentId ? [agentId, limit] : [limit];
      
      return db.executeQuery(query, params);
    },

    runStatistics: async (_: any, { dateRange, agentId, worktreeId }: any) => {
      // Placeholder implementation
      return {
        totalRuns: 0,
        successfulRuns: 0,
        failedRuns: 0,
        averageRunTime: 0,
        averageEvaluationScore: 0,
        totalDuration: 0,
        runsByStatus: [],
        runsByType: [],
        runsByAgent: [],
        performanceTrend: [],
      };
    },

    competitionRuns: async (_: any, { competitionId }: { competitionId: string }) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_runs WHERE competition_id = ? ORDER BY competition_rank ASC',
        [competitionId]
      );
    },
  },

  Mutation: {
    createRun: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    updateRun: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    deleteRun: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    cancelRun: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    retryRun: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    toggleRunBookmark: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      try {
        const run = await db.executeQuerySingle(
          'UPDATE agent_runs SET bookmarked = NOT bookmarked, updated_at = NOW() WHERE id = ? RETURNING *',
          [id]
        );
        return db.createResponse(true, { run });
      } catch (error) {
        return db.createResponse(false, null, [{ message: 'Failed to toggle bookmark', code: 'TOGGLE_FAILED' }]);
      }
    },

    evaluateRun: async (_: any, { id, score, feedback }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },
  },

  Subscription: {
    runUpdated: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    runStatusChanged: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    runStarted: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    runCompleted: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    competitionRunUpdated: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },
  },
};