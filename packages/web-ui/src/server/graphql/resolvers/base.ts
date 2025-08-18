import { DateTime, JSON } from '../types/scalars.js';

export const baseResolvers = {
  // Custom scalar resolvers
  DateTime,
  JSON,

  // Base interface resolvers
  Node: {
    __resolveType(obj: any) {
      // Determine the concrete type based on the object properties
      if (obj.branch && obj.path) return 'Worktree';
      if (obj.agentId && obj.type) return 'Agent';
      if (obj.runId && obj.status) return 'Run';
      if (obj.worktreeId && obj.filesIndexed !== undefined) return 'MaproomIndex';
      if (obj.key && obj.value !== undefined) return 'Configuration';
      if (obj.timestamp && obj.entityId !== undefined) return 'Event';
      if (obj.messageId && obj.senderAgentId) return 'AgentMessage';
      return null;
    },
  },

  Response: {
    __resolveType(obj: any) {
      // Determine the response type based on the object properties
      if (obj.worktree) return 'WorktreeResponse';
      if (obj.agent) return 'AgentResponse';
      if (obj.run) return 'RunResponse';
      if (obj.index) return 'MaproomIndexResponse';
      if (obj.configuration) return 'ConfigurationResponse';
      if (obj.event) return 'EventResponse';
      if (obj.message) return 'AgentMessageResponse';
      if (obj.deletedId) {
        // Determine delete response type based on context
        return 'WorktreeDeleteResponse'; // Default, could be improved with more context
      }
      return null;
    },
  },

  EventEntity: {
    __resolveType(obj: any) {
      if (obj.branch && obj.path) return 'Worktree';
      if (obj.agentId && obj.type) return 'Agent';
      if (obj.runId && obj.status) return 'Run';
      if (obj.worktreeId && obj.filesIndexed !== undefined) return 'MaproomIndex';
      if (obj.key && obj.value !== undefined) return 'Configuration';
      return null;
    },
  },

  Query: {
    // Base health check is handled in the main schema
  },

  Mutation: {
    // Base mutations
  },

  Subscription: {
    // Base subscriptions
  },
};