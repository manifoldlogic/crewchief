import { getDatabaseService } from '../services/database.js';

export const eventResolvers = {
  Event: {
    id: (parent: any) => parent.id?.toString(),
    entityId: (parent: any) => parent.entity_id?.toString(),
    timestamp: (parent: any) => parent.timestamp,
    createdAt: (parent: any) => parent.created_at,
    updatedAt: (parent: any) => parent.updated_at,

    // Extended fields with default mappings
    source: (parent: any) => parent.source || 'SYSTEM',
    severity: (parent: any) => parent.severity || 'INFO',
    message: (parent: any) => parent.message || '',
    description: (parent: any) => parent.description,
    entityType: (parent: any) => parent.entity_type,
    userId: (parent: any) => parent.user_id,
    sessionId: (parent: any) => parent.session_id,
    ipAddress: (parent: any) => parent.ip_address,
    userAgent: (parent: any) => parent.user_agent,
    correlationId: (parent: any) => parent.correlation_id,
    traceId: (parent: any) => parent.trace_id,
    spanId: (parent: any) => parent.span_id,
    parentEventId: (parent: any) => parent.parent_event_id?.toString(),
    rootEventId: (parent: any) => parent.root_event_id?.toString(),
    sequence: (parent: any) => parent.sequence || 0,
    version: (parent: any) => parent.version || '1.0',
    schema: (parent: any) => parent.schema || 'event/v1',
    checksum: (parent: any) => parent.checksum || '',
    metadata: (parent: any) => parent.metadata || {},
    context: (parent: any) => parent.context || {},
    tags: (parent: any) => parent.tags || [],
    fingerprint: (parent: any) => parent.fingerprint || '',
    processed: (parent: any) => parent.processed || false,
    processedAt: (parent: any) => parent.processed_at,
    processingDurationMs: (parent: any) => parent.processing_duration_ms,
    retryCount: (parent: any) => parent.retry_count || 0,
    maxRetries: (parent: any) => parent.max_retries || 3,
    nextRetryAt: (parent: any) => parent.next_retry_at,
    errorMessage: (parent: any) => parent.error_message,

    // Computed fields
    age: (parent: any) => {
      const diff = Date.now() - new Date(parent.timestamp || parent.created_at).getTime();
      const minutes = Math.floor(diff / (1000 * 60));
      if (minutes < 60) return `${minutes}m ago`;
      const hours = Math.floor(minutes / 60);
      if (hours < 24) return `${hours}h ago`;
      const days = Math.floor(hours / 24);
      return `${days}d ago`;
    },
    isRecent: (parent: any) => {
      const diff = Date.now() - new Date(parent.timestamp || parent.created_at).getTime();
      return diff < 1000 * 60 * 60; // 1 hour
    },
    needsAttention: (parent: any) => {
      return parent.severity === 'ERROR' || parent.severity === 'CRITICAL';
    },
    canRetry: (parent: any) => {
      return !parent.processed && (parent.retry_count || 0) < (parent.max_retries || 3);
    },

    // Relations
    parentEvent: async (parent: any) => {
      if (!parent.parent_event_id) return null;
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM events WHERE id = ?',
        [parent.parent_event_id]
      );
    },

    childEvents: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM events WHERE parent_event_id = ? ORDER BY created_at ASC',
        [parent.id]
      );
    },

    relatedEntity: async (parent: any) => {
      if (!parent.entity_id || !parent.entity_type) return null;
      
      const db = getDatabaseService();
      let table;
      switch (parent.entity_type.toLowerCase()) {
        case 'worktree':
          table = 'worktree_status';
          break;
        case 'agent':
        case 'run':
          table = 'agent_runs';
          break;
        case 'maproom_index':
          table = 'maproom_worktrees';
          break;
        case 'configuration':
          table = 'web_ui_preferences';
          break;
        default:
          return null;
      }
      
      return db.executeQuerySingle(
        `SELECT * FROM ${table} WHERE id = ?`,
        [parent.entity_id]
      );
    },
  },

  Query: {
    event: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      // Note: This would require an events table to be created
      return db.executeQuerySingle(
        'SELECT * FROM events WHERE id = ?',
        [id]
      );
    },

    events: async (_: any, { filter, sort, pagination }: any) => {
      const db = getDatabaseService();
      const dbFilter: Record<string, any> = {};
      
      if (filter) {
        if (filter.types) dbFilter.type = filter.types;
        if (filter.sources) dbFilter.source = filter.sources;
        if (filter.severities) dbFilter.severity = filter.severities;
        if (filter.entityId) dbFilter.entity_id = filter.entityId;
        if (filter.entityType) dbFilter.entity_type = filter.entityType;
        if (filter.userId) dbFilter.user_id = filter.userId;
        if (filter.sessionId) dbFilter.session_id = filter.sessionId;
        if (filter.correlationId) dbFilter.correlation_id = filter.correlationId;
        if (filter.traceId) dbFilter.trace_id = filter.traceId;
        if (filter.processed !== undefined) dbFilter.processed = filter.processed;
        if (filter.search) dbFilter.search = filter.search;
      }

      return db.getConnection('events', dbFilter, sort, pagination);
    },

    eventsByEntity: async (_: any, { entityId, entityType }: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM events WHERE entity_id = ? AND entity_type = ? ORDER BY created_at DESC',
        [entityId, entityType]
      );
    },

    eventsByCorrelation: async (_: any, { correlationId }: { correlationId: string }) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM events WHERE correlation_id = ? ORDER BY created_at ASC',
        [correlationId]
      );
    },

    eventsByTrace: async (_: any, { traceId }: { traceId: string }) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM events WHERE trace_id = ? ORDER BY created_at ASC',
        [traceId]
      );
    },

    eventThread: async (_: any, { rootEventId }: { rootEventId: string }) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM events WHERE root_event_id = ? ORDER BY sequence ASC, created_at ASC',
        [rootEventId]
      );
    },

    eventStream: async (_: any, { input }: any) => {
      // Placeholder implementation
      return {
        events: [],
        cursor: '',
        hasMore: false,
        totalCount: 0,
      };
    },

    recentEvents: async (_: any, { limit = 20, severities }: any) => {
      const db = getDatabaseService();
      let query = 'SELECT * FROM events ORDER BY created_at DESC LIMIT ?';
      let params = [limit];
      
      if (severities && severities.length > 0) {
        query = 'SELECT * FROM events WHERE severity = ANY(?) ORDER BY created_at DESC LIMIT ?';
        params = [severities, limit];
      }
      
      return db.executeQuery(query, params);
    },

    unprocessedEvents: async (_: any, { limit = 100 }: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM events WHERE processed = false ORDER BY created_at ASC LIMIT ?',
        [limit]
      );
    },

    errorEvents: async (_: any, { limit = 50, hours = 24 }: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        `SELECT * FROM events 
         WHERE severity IN ('ERROR', 'CRITICAL') 
         AND created_at > NOW() - INTERVAL '${hours} hours'
         ORDER BY created_at DESC 
         LIMIT ?`,
        [limit]
      );
    },

    eventStatistics: async (_: any, { dateRange, filter }: any) => {
      // Placeholder implementation
      return {
        totalEvents: 0,
        eventsByType: [],
        eventsBySeverity: [],
        eventsBySource: [],
        recentActivity: [],
        errorRate: 0,
        processingRate: 0,
        averageProcessingTime: 0,
      };
    },

    eventTimeline: async (_: any, { entityId, entityType, hours = 24 }: any) => {
      // Placeholder implementation
      return [];
    },

    eventAlerts: async () => {
      // Placeholder implementation
      return [];
    },

    eventAlert: async (_: any, { id }: { id: string }) => {
      // Placeholder implementation
      return null;
    },
  },

  Mutation: {
    createEvent: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    updateEvent: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    deleteEvent: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    bulkDeleteEvents: async (_: any, { filter }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    markEventProcessed: async (_: any, { id, processingResult }: any) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    retryEventProcessing: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.createResponse(false, null, [{ message: 'Not implemented', code: 'NOT_IMPLEMENTED' }]);
    },

    bulkMarkProcessed: async (_: any, { ids }: { ids: string[] }) => {
      const db = getDatabaseService();
      return [];
    },

    cleanupOldEvents: async (_: any, { olderThan, keepCritical = true }: any) => {
      // Placeholder implementation
      return 0;
    },

    archiveEvents: async (_: any, { filter }: any) => {
      // Placeholder implementation
      return 0;
    },

    createEventAlert: async (_: any, { input }: any) => {
      // Placeholder implementation
      return null;
    },

    updateEventAlert: async (_: any, { id, input }: any) => {
      // Placeholder implementation
      return null;
    },

    deleteEventAlert: async (_: any, { id }: { id: string }) => {
      // Placeholder implementation
      return false;
    },

    enableEventAlert: async (_: any, { id }: { id: string }) => {
      // Placeholder implementation
      return null;
    },

    disableEventAlert: async (_: any, { id }: { id: string }) => {
      // Placeholder implementation
      return null;
    },
  },

  Subscription: {
    eventCreated: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    eventUpdated: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    eventProcessed: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    eventsStream: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    eventAlertTriggered: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    systemEvents: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    errorEvents: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    userEvents: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    entityEvents: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },
  },
};