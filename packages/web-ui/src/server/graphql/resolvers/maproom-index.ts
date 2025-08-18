import { getDatabaseService } from '../services/database.js';

export const maproomIndexResolvers = {
  MaproomIndex: {
    id: (parent: any) => parent.id?.toString(),
    worktreeId: (parent: any) => parent.worktree_id?.toString(),
    repoId: (parent: any) => parent.repo_id?.toString(),
    filesIndexed: (parent: any) => parent.files_indexed || 0,
    lastUpdated: (parent: any) => parent.last_updated || parent.indexed_at,
    createdAt: (parent: any) => parent.created_at,
    updatedAt: (parent: any) => parent.updated_at,

    // Computed fields
    indexingProgress: (parent: any) => {
      if (parent.total_files && parent.files_indexed) {
        return (parent.files_indexed / parent.total_files) * 100;
      }
      return 0;
    },
    isHealthy: (parent: any) => parent.status === 'COMPLETED',
    needsReindex: (parent: any) => parent.status === 'STALE',
    timeSinceLastUpdate: (parent: any) => {
      if (parent.last_updated) {
        const diff = Date.now() - new Date(parent.last_updated).getTime();
        const hours = Math.floor(diff / (1000 * 60 * 60));
        return `${hours} hours ago`;
      }
      return 'Never';
    },

    // Relations
    worktree: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM worktree_status WHERE worktree_id = ?',
        [parent.id]
      );
    },

    searchHistory: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM web_search_history WHERE worktree_id = ? ORDER BY searched_at DESC',
        [parent.id]
      );
    },
  },

  SearchHistory: {
    id: (parent: any) => parent.id?.toString(),
    clickedResults: (parent: any) => parent.clicked_results || [],
    saved: (parent: any) => parent.saved || false,
    createdAt: (parent: any) => parent.searched_at,
    updatedAt: (parent: any) => parent.searched_at,

    // Relations
    maproomIndex: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT mw.* FROM maproom_worktrees mw WHERE mw.id = ?',
        [parent.worktree_id]
      );
    },

    worktree: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM worktree_status WHERE worktree_id = ?',
        [parent.worktree_id]
      );
    },
  },

  Query: {
    maproomIndex: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM maproom_worktrees WHERE id = ?',
        [id]
      );
    },

    maproomIndices: async (_: any, { filter, sort, pagination }: any) => {
      const db = getDatabaseService();
      const dbFilter: Record<string, any> = {};
      
      if (filter) {
        if (filter.status) dbFilter.status = filter.status;
        if (filter.worktreeId) dbFilter.worktree_id = filter.worktreeId;
        if (filter.repoId) dbFilter.repo_id = filter.repoId;
      }

      return db.getConnection('maproom_worktrees', dbFilter, sort, pagination);
    },

    maproomIndexByWorktree: async (_: any, { worktreeId }: { worktreeId: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM maproom_worktrees WHERE id = ?',
        [worktreeId]
      );
    },

    indexStatistics: async () => {
      // Placeholder implementation
      return {
        totalWorktrees: 0,
        indexedWorktrees: 0,
        pendingWorktrees: 0,
        failedWorktrees: 0,
        totalFiles: 0,
        totalChunks: 0,
        totalSizeBytes: 0,
        averageIndexingTime: 0,
        languageBreakdown: [],
        recentIndexingActivity: [],
      };
    },

    search: async (_: any, { input }: any) => {
      // Placeholder search implementation
      return {
        results: [],
        totalCount: 0,
        executionTimeMs: 0,
        query: input.query,
        searchType: input.searchType,
        performanceMetrics: {},
      };
    },

    searchHistory: async (_: any, { filter, pagination }: any) => {
      const db = getDatabaseService();
      const dbFilter: Record<string, any> = {};
      
      if (filter) {
        if (filter.worktreeId) dbFilter.worktree_id = filter.worktreeId;
        if (filter.searchType) dbFilter.search_type = filter.searchType;
        if (filter.saved !== undefined) dbFilter.saved = filter.saved;
      }

      const result = await db.getConnection('web_search_history', dbFilter, undefined, pagination);
      return result.edges.map(edge => edge.node);
    },

    popularSearches: async (_: any, { limit = 10, days = 7 }: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        `SELECT query, COUNT(*) as frequency 
         FROM web_search_history 
         WHERE searched_at > NOW() - INTERVAL '${days} days'
         GROUP BY query 
         ORDER BY frequency DESC 
         LIMIT ?`,
        [limit]
      );
    },
  },

  Mutation: {
    createMaproomIndex: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    updateMaproomIndex: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    deleteMaproomIndex: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    reindexWorktree: async (_: any, { worktreeId, force = false }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    markIndexStale: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    saveSearch: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      const search = await db.executeQuerySingle(
        'UPDATE web_search_history SET saved = true WHERE id = ? RETURNING *',
        [id]
      );
      return search;
    },

    unsaveSearch: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      const search = await db.executeQuerySingle(
        'UPDATE web_search_history SET saved = false WHERE id = ? RETURNING *',
        [id]
      );
      return search;
    },

    deleteSearchHistory: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      try {
        await db.executeQuery('DELETE FROM web_search_history WHERE id = ?', [id]);
        return true;
      } catch (error) {
        return false;
      }
    },

    clearSearchHistory: async (_: any, { worktreeId }: { worktreeId?: string }) => {
      const db = getDatabaseService();
      try {
        if (worktreeId) {
          await db.executeQuery('DELETE FROM web_search_history WHERE worktree_id = ?', [worktreeId]);
        } else {
          await db.executeQuery('DELETE FROM web_search_history');
        }
        return true;
      } catch (error) {
        return false;
      }
    },
  },

  Subscription: {
    maproomIndexUpdated: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    indexingStatusChanged: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    indexingStarted: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    indexingCompleted: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    searchPerformed: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },
  },
};