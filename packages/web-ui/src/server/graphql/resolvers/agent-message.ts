import { getDatabaseService } from '../services/database.js';

export const agentMessageResolvers = {
  AgentMessage: {
    id: (parent: any) => parent.id?.toString(),
    messageId: (parent: any) => parent.message_id,
    correlationId: (parent: any) => parent.correlation_id,
    replyToId: (parent: any) => parent.reply_to_id,
    runId: (parent: any) => parent.run_id,
    senderAgentId: (parent: any) => parent.sender_agent_id,
    recipientAgentId: (parent: any) => parent.recipient_agent_id,
    type: (parent: any) => parent.message_type,
    priority: (parent: any) => parent.priority,
    subject: (parent: any) => parent.subject,
    content: (parent: any) => parent.content,
    contentFormat: (parent: any) => parent.content_format || 'text',
    metadata: (parent: any) => parent.metadata,
    attachments: (parent: any) => parent.attachments,
    broadcast: (parent: any) => parent.broadcast || false,
    deliveredAt: (parent: any) => parent.delivered_at,
    acknowledgedAt: (parent: any) => parent.acknowledged_at,
    createdAt: (parent: any) => parent.created_at,
    expiresAt: (parent: any) => parent.expires_at,
    processed: (parent: any) => parent.processed || false,
    processingResult: (parent: any) => parent.processing_result,
    retryCount: (parent: any) => parent.retry_count || 0,
    maxRetries: (parent: any) => parent.max_retries || 3,
    busTopic: (parent: any) => parent.bus_topic,
    busPartition: (parent: any) => parent.bus_partition,
    busOffset: (parent: any) => parent.bus_offset,
    tags: (parent: any) => parent.tags || [],
    sizeBytes: (parent: any) => parent.size_bytes || 0,
    processingTimeMs: (parent: any) => parent.processing_time_ms,
    updatedAt: (parent: any) => parent.created_at, // Use created_at as fallback

    // Computed fields
    isExpired: (parent: any) => {
      if (!parent.expires_at) return false;
      return new Date(parent.expires_at) < new Date();
    },
    canRetry: (parent: any) => {
      return !parent.processed && (parent.retry_count || 0) < (parent.max_retries || 3);
    },
    threadLength: async (parent: any) => {
      const db = getDatabaseService();
      const result = await db.executeQuery(
        'SELECT COUNT(*) FROM agent_messages WHERE correlation_id = ?',
        [parent.correlation_id || parent.message_id]
      );
      return parseInt(result[0].count);
    },
    age: (parent: any) => {
      const diff = Date.now() - new Date(parent.created_at).getTime();
      const minutes = Math.floor(diff / (1000 * 60));
      if (minutes < 60) return `${minutes}m ago`;
      const hours = Math.floor(minutes / 60);
      if (hours < 24) return `${hours}h ago`;
      const days = Math.floor(hours / 24);
      return `${days}d ago`;
    },

    // Relations
    run: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_runs WHERE run_id = ?',
        [parent.run_id]
      );
    },

    replyTo: async (parent: any) => {
      if (!parent.reply_to_id) return null;
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_messages WHERE message_id = ?',
        [parent.reply_to_id]
      );
    },

    replies: async (parent: any) => {
      const db = getDatabaseService();
      return db.executeQuery(
        'SELECT * FROM agent_messages WHERE reply_to_id = ? ORDER BY created_at ASC',
        [parent.message_id]
      );
    },
  },

  MessageThread: {
    rootMessage: (parent: any) => parent.rootMessage,
    messages: (parent: any) => parent.messages,
    totalCount: (parent: any) => parent.messages?.length || 0,
    participantCount: (parent: any) => {
      const participants = new Set();
      parent.messages?.forEach((msg: any) => {
        participants.add(msg.sender_agent_id);
        if (msg.recipient_agent_id) participants.add(msg.recipient_agent_id);
      });
      return participants.size;
    },
    lastActivity: (parent: any) => {
      if (!parent.messages || parent.messages.length === 0) return null;
      return parent.messages[parent.messages.length - 1].created_at;
    },
  },

  Query: {
    agentMessage: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      return db.executeQuerySingle(
        'SELECT * FROM agent_messages WHERE id = ?',
        [id]
      );
    },

    agentMessages: async (_: any, { filter, sort, pagination }: any) => {
      const db = getDatabaseService();
      const dbFilter: Record<string, any> = {};
      
      if (filter) {
        if (filter.type) dbFilter.message_type = filter.type;
        if (filter.priority) dbFilter.priority = filter.priority;
        if (filter.senderAgentId) dbFilter.sender_agent_id = filter.senderAgentId;
        if (filter.recipientAgentId) dbFilter.recipient_agent_id = filter.recipientAgentId;
        if (filter.runId) dbFilter.run_id = filter.runId;
        if (filter.processed !== undefined) dbFilter.processed = filter.processed;
        if (filter.broadcast !== undefined) dbFilter.broadcast = filter.broadcast;
        if (filter.search) dbFilter.search = filter.search;
      }

      return db.getConnection('agent_messages', dbFilter, sort, pagination);
    },

    messageThread: async (_: any, { messageId }: { messageId: string }) => {
      const db = getDatabaseService();
      
      // Get the root message
      const rootMessage = await db.executeQuerySingle(
        'SELECT * FROM agent_messages WHERE message_id = ?',
        [messageId]
      );
      
      if (!rootMessage) return null;

      // Get all messages in the thread (using correlation_id or message_id)
      const correlationId = rootMessage.correlation_id || messageId;
      const messages = await db.executeQuery(
        'SELECT * FROM agent_messages WHERE correlation_id = ? OR (correlation_id IS NULL AND message_id = ?) ORDER BY created_at ASC',
        [correlationId, messageId]
      );

      return {
        rootMessage,
        messages,
      };
    },

    agentConversation: async (_: any, { agentId, otherAgentId }: any) => {
      const db = getDatabaseService();
      
      let query;
      let params;
      
      if (otherAgentId) {
        query = `SELECT * FROM agent_messages 
                 WHERE (sender_agent_id = ? AND recipient_agent_id = ?) 
                    OR (sender_agent_id = ? AND recipient_agent_id = ?)
                 ORDER BY created_at ASC`;
        params = [agentId, otherAgentId, otherAgentId, agentId];
      } else {
        query = `SELECT * FROM agent_messages 
                 WHERE sender_agent_id = ? OR recipient_agent_id = ?
                 ORDER BY created_at ASC`;
        params = [agentId, agentId];
      }
      
      return db.executeQuery(query, params);
    },

    recentMessages: async (_: any, { limit = 20, agentId }: any) => {
      const db = getDatabaseService();
      
      if (agentId) {
        return db.executeQuery(
          'SELECT * FROM agent_messages WHERE sender_agent_id = ? OR recipient_agent_id = ? ORDER BY created_at DESC LIMIT ?',
          [agentId, agentId, limit]
        );
      }
      
      return db.executeQuery(
        'SELECT * FROM agent_messages ORDER BY created_at DESC LIMIT ?',
        [limit]
      );
    },

    unprocessedMessages: async (_: any, { agentId }: any) => {
      const db = getDatabaseService();
      
      if (agentId) {
        return db.executeQuery(
          'SELECT * FROM agent_messages WHERE recipient_agent_id = ? AND processed = false ORDER BY created_at ASC',
          [agentId]
        );
      }
      
      return db.executeQuery(
        'SELECT * FROM agent_messages WHERE processed = false ORDER BY created_at ASC'
      );
    },
  },

  Mutation: {
    sendAgentMessage: async (_: any, { input }: any) => {
      const db = getDatabaseService();
      
      // Validate required fields
      const errors = db.validateRequired(input, ['runId', 'senderAgentId', 'type', 'content']);
      if (errors.length > 0) {
        return db.createResponse(false, null, errors);
      }

      try {
        const message = await db.withTransaction(async (client) => {
          const messageId = `msg_${Date.now()}_${Math.random().toString(36).substring(2)}`;
          
          const result = await client.query(
            `INSERT INTO agent_messages 
             (message_id, correlation_id, reply_to_id, run_id, sender_agent_id, recipient_agent_id,
              message_type, priority, subject, content, content_format, metadata, attachments,
              broadcast, expires_at, tags, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NOW())
             RETURNING *`,
            [
              messageId,
              input.correlationId || messageId,
              input.replyToId,
              input.runId,
              input.senderAgentId,
              input.recipientAgentId,
              input.type,
              input.priority || 'NORMAL',
              input.subject,
              input.content,
              input.contentFormat || 'text',
              input.metadata,
              input.attachments,
              input.broadcast || false,
              input.expiresAt,
              input.tags || []
            ]
          );
          
          return result.rows[0];
        });

        return db.createResponse(true, { message });
      } catch (error) {
        console.error('Error sending agent message:', error);
        return db.createResponse(false, null, [
          { message: 'Failed to send message', code: 'SEND_FAILED' },
        ]);
      }
    },

    acknowledgeMessage: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      
      try {
        const message = await db.executeQuerySingle(
          'UPDATE agent_messages SET acknowledged_at = NOW() WHERE id = ? RETURNING *',
          [id]
        );
        
        return db.createResponse(true, { message });
      } catch (error) {
        return db.createResponse(false, null, [
          { message: 'Failed to acknowledge message', code: 'ACKNOWLEDGE_FAILED' },
        ]);
      }
    },

    markMessageProcessed: async (_: any, { id, result }: any) => {
      const db = getDatabaseService();
      
      try {
        const message = await db.executeQuerySingle(
          'UPDATE agent_messages SET processed = true, processing_result = ?, processed_at = NOW() WHERE id = ? RETURNING *',
          [result, id]
        );
        
        return db.createResponse(true, { message });
      } catch (error) {
        return db.createResponse(false, null, [
          { message: 'Failed to mark message processed', code: 'PROCESS_FAILED' },
        ]);
      }
    },

    retryMessage: async (_: any, { id }: { id: string }) => {
      const db = getDatabaseService();
      
      try {
        const message = await db.executeQuerySingle(
          'UPDATE agent_messages SET retry_count = retry_count + 1, next_retry_at = NOW() + INTERVAL \'5 minutes\' WHERE id = ? RETURNING *',
          [id]
        );
        
        return db.createResponse(true, { message });
      } catch (error) {
        return db.createResponse(false, null, [
          { message: 'Failed to retry message', code: 'RETRY_FAILED' },
        ]);
      }
    },
  },

  Subscription: {
    messageReceived: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    messageAcknowledged: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    messageProcessed: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },

    broadcastMessage: {
      subscribe: () => {
        throw new Error('Subscriptions not yet implemented');
      },
    },
  },
};