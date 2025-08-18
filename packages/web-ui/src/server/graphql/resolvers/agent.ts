import { getDatabaseService, PaginationArgs, SortArgs } from '../services/database.js';

export const agentResolvers = {
  Agent: {
    id: (parent: any) => parent.id?.toString(),
    worktreeId: (parent: any) => parent.worktree_id?.toString(),
    repoId: (parent: any) => parent.repo_id?.toString(),
    agentId: (parent: any) => parent.agent_id,
    runId: (parent: any) => parent.run_id,
    parentRunId: (parent: any) => parent.parent_run_id,
    commitSha: (parent: any) => parent.commit_sha,
    taskDescription: (parent: any) => parent.task_description,
    taskType: (parent: any) => parent.task_type,
    contextFiles: (parent: any) => parent.context_files || [],
    startedAt: (parent: any) => parent.started_at,
    completedAt: (parent: any) => parent.completed_at,
    durationMs: (parent: any) => parent.duration_ms,
    tmuxSession: (parent: any) => parent.tmux_session,
    tmuxWindow: (parent: any) => parent.tmux_window,
    tmuxPane: (parent: any) => parent.tmux_pane,
    exitCode: (parent: any) => parent.exit_code,
    errorMessage: (parent: any) => parent.error_message,
    evaluationScore: (parent: any) => parent.evaluation_score,
    testsPassed: (parent: any) => parent.tests_passed,
    reviewRequired: (parent: any) => parent.review_required,
    autoMergeEligible: (parent: any) => parent.auto_merge_eligible,
    cpuUsageAvg: (parent: any) => parent.cpu_usage_avg,
    memoryUsagePeak: (parent: any) => parent.memory_usage_peak,
    diskIoBytes: (parent: any) => parent.disk_io_bytes,
    networkRequests: (parent: any) => parent.network_requests,
    stdoutLogPath: (parent: any) => parent.stdout_log_path,
    stderrLogPath: (parent: any) => parent.stderr_log_path,
    logSummary: (parent: any) => parent.log_summary,
    createdAt: (parent: any) => parent.created_at,
    updatedAt: (parent: any) => parent.updated_at,
    competitionId: (parent: any) => parent.competition_id,
    competitionRank: (parent: any) => parent.competition_rank,
    userFeedback: (parent: any) => parent.user_feedback,
    bookmarked: (parent: any) => parent.bookmarked || false,
    tags: (parent: any) => parent.tags || [],

    // Relations
    worktree: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM worktree_status WHERE id = ?',
        [parent.worktree_id]
      );
    },

    runs: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_runs WHERE agent_id = ? ORDER BY started_at DESC',
        [parent.agent_id]
      );
    },

    messages: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_messages WHERE run_id = ? ORDER BY created_at DESC',
        [parent.run_id]
      );
    },

    currentRun: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_runs WHERE agent_id = ? AND status IN (\'pending\', \'running\') ORDER BY started_at DESC LIMIT 1',
        [parent.agent_id]
      );
    },
  },

  Query: {
    agent: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_runs WHERE id = ?',
        [id]
      );
    },

    agents: async (_: any, { filter, sort, pagination }: any) => {
      const db = getDatabaseService();
      const dbFilter: Record<string, any> = {};
      
      if (filter) {
        if (filter.type) dbFilter.agent_type = filter.type;
        if (filter.status) dbFilter.status = filter.status;
        if (filter.worktreeId) dbFilter.worktree_id = filter.worktreeId;
        if (filter.repoId) dbFilter.repo_id = filter.repoId;
        if (filter.bookmarked !== undefined) dbFilter.bookmarked = filter.bookmarked;
        if (filter.search) dbFilter.search = filter.search;
      }

      return db.getConnection('agent_runs', dbFilter, sort, pagination);
    },

    agentByRunId: async (_: any, { runId }: { runId: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_runs WHERE run_id = ?',
        [runId]
      );
    },

    agentPerformanceMetrics: async (_: any, { agentId, type, dateRange }: any) => {
      // Placeholder implementation
      return {
        averageRunTime: 0,
        successRate: 0,
        totalRuns: 0,
        averageEvaluationScore: 0,
        cpuUsageAverage: 0,
        memoryUsageAverage: 0,
        networkRequestsAverage: 0,
      };
    },

    activeAgents: async () => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_runs WHERE status IN (\'pending\', \'running\') ORDER BY started_at DESC'
      );
    },
  },

  Mutation: {
    createAgent: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    updateAgent: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    deleteAgent: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    startAgent: async (_: any, { id, taskDescription }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    stopAgent: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    toggleAgentBookmark: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      try {
        const agent = await db.executeQuerySingle(
          'UPDATE agent_runs SET bookmarked = NOT bookmarked, updated_at = NOW() WHERE id = ? RETURNING *',
          [id]
        );
        return db.createResponse(true, { agent });
      } catch (error) {
        return db.createResponse(false, null, [{ message: 'Failed to toggle bookmark', code: 'TOGGLE_FAILED' }]);
      }
    },
  },

  Subscription: {
    agentUpdated: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    agentStatusChanged: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    agentStarted: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    agentCompleted: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },
  },
};