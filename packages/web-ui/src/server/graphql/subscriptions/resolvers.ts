import { withFilter } from 'graphql-subscriptions';
import type { GraphQLContext } from '../apollo.js';
import { 
  getPubSub, 
  SUBSCRIPTION_EVENTS, 
  createFilteredAsyncIterator,
  type SubscriptionEvent 
} from './pubsub.js';

// Authentication helper for subscriptions
function requireAuth(context: GraphQLContext) {
  if (!context.user) {
    throw new Error('Authentication required for subscriptions');
  }
  return context.user;
}

// Permission helper for subscriptions
function checkPermission(context: GraphQLContext, permission: string) {
  const user = requireAuth(context);
  if (!user.permissions.includes(permission) && !user.permissions.includes('admin')) {
    throw new Error(`Permission denied: ${permission} required`);
  }
}

// Generic subscription resolver helper
function createSubscriptionResolver<TPayload = any>(
  event: SubscriptionEvent | SubscriptionEvent[],
  permission?: string,
  filter?: (payload: TPayload, variables: any, context: GraphQLContext) => boolean
) {
  return {
    subscribe: withFilter(
      (root: any, args: any, context: GraphQLContext) => {
        // Check authentication and permissions
        const user = requireAuth(context);
        if (permission) {
          checkPermission(context, permission);
        }

        // Create filtered async iterator with user/workspace context
        return createFilteredAsyncIterator(
          event,
          filter ? (payload: TPayload) => filter(payload, args, context) : undefined,
          user.id,
          undefined // workspaceId - add if needed
        );
      },
      (payload: TPayload, variables: any, context: GraphQLContext) => {
        // Additional filtering can be done here
        return filter ? filter(payload, variables, context) : true;
      }
    ),
  };
}

// Worktree subscriptions
export const worktreeSubscriptions = {
  worktreeUpdated: createSubscriptionResolver(
    [SUBSCRIPTION_EVENTS.WORKTREE_UPDATED, SUBSCRIPTION_EVENTS.WORKTREE_STATUS_CHANGED],
    'read:worktrees',
    (payload, variables) => {
      // Filter by specific worktree ID if provided
      return !variables.id || payload.worktree?.id === variables.id;
    }
  ),

  worktreeStatusChanged: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.WORKTREE_STATUS_CHANGED,
    'read:worktrees'
  ),

  worktreeCreated: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.WORKTREE_CREATED,
    'read:worktrees'
  ),

  worktreeDeleted: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.WORKTREE_DELETED,
    'read:worktrees'
  ),
};

// Agent subscriptions
export const agentSubscriptions = {
  agentUpdated: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.AGENT_STATUS_CHANGED,
    'read:agents',
    (payload, variables) => {
      return !variables.id || payload.agent?.id === variables.id;
    }
  ),

  agentStatusChanged: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.AGENT_STATUS_CHANGED,
    'read:agents'
  ),

  agentStarted: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.AGENT_SPAWNED,
    'read:agents'
  ),

  agentCompleted: createSubscriptionResolver(
    [SUBSCRIPTION_EVENTS.AGENT_COMPLETED, SUBSCRIPTION_EVENTS.AGENT_FAILED],
    'read:agents'
  ),

  agentMessageReceived: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.AGENT_MESSAGE_RECEIVED,
    'read:agents',
    (payload, variables) => {
      return !variables.agentId || payload.message?.agentId === variables.agentId;
    }
  ),
};

// Run subscriptions
export const runSubscriptions = {
  runUpdated: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.RUN_STATUS_CHANGED,
    'read:runs',
    (payload, variables) => {
      return !variables.id || payload.run?.id === variables.id;
    }
  ),

  runStarted: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.RUN_STARTED,
    'read:runs'
  ),

  runCompleted: createSubscriptionResolver(
    [SUBSCRIPTION_EVENTS.RUN_COMPLETED, SUBSCRIPTION_EVENTS.RUN_FAILED],
    'read:runs'
  ),

  runStatusChanged: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.RUN_STATUS_CHANGED,
    'read:runs'
  ),
};

// Maproom subscriptions
export const maproomSubscriptions = {
  maproomIndexUpdated: createSubscriptionResolver(
    [
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_PROGRESS,
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_COMPLETED,
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_FAILED
    ],
    'read:maproom',
    (payload, variables) => {
      return !variables.id || payload.index?.id === variables.id;
    }
  ),

  indexingStatusChanged: createSubscriptionResolver(
    [
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_STARTED,
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_PROGRESS,
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_COMPLETED,
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_FAILED
    ],
    'read:maproom'
  ),

  indexingStarted: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.MAPROOM_INDEX_STARTED,
    'read:maproom'
  ),

  indexingCompleted: createSubscriptionResolver(
    [SUBSCRIPTION_EVENTS.MAPROOM_INDEX_COMPLETED, SUBSCRIPTION_EVENTS.MAPROOM_INDEX_FAILED],
    'read:maproom'
  ),

  searchPerformed: createSubscriptionResolver(
    'SEARCH_PERFORMED' as SubscriptionEvent, // Add this to SUBSCRIPTION_EVENTS if needed
    'read:maproom'
  ),
};

// Configuration subscriptions
export const configurationSubscriptions = {
  configurationUpdated: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.CONFIG_CHANGED,
    'read:config'
  ),

  configurationValidated: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.CONFIG_VALIDATED,
    'read:config'
  ),
};

// File system subscriptions
export const fileSystemSubscriptions = {
  fileChanged: createSubscriptionResolver(
    [
      SUBSCRIPTION_EVENTS.FILE_CHANGED,
      SUBSCRIPTION_EVENTS.FILE_CREATED,
      SUBSCRIPTION_EVENTS.FILE_DELETED
    ],
    'read:filesystem',
    (payload, variables) => {
      // Filter by path pattern if provided
      if (variables.pathPattern) {
        const regex = new RegExp(variables.pathPattern);
        return regex.test(payload.file?.path || '');
      }
      
      // Filter by worktree if provided
      if (variables.worktreeId) {
        return payload.file?.worktreeId === variables.worktreeId;
      }
      
      return true;
    }
  ),

  directoryChanged: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.DIRECTORY_CHANGED,
    'read:filesystem'
  ),
};

// Git subscriptions
export const gitSubscriptions = {
  gitCommit: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.GIT_COMMIT,
    'read:git'
  ),

  gitBranchChanged: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.GIT_BRANCH_CHANGED,
    'read:git'
  ),

  gitMerge: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.GIT_MERGE,
    'read:git'
  ),

  gitOperationProgress: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.GIT_OPERATION_PROGRESS,
    'read:git',
    (payload, variables) => {
      return !variables.operationId || payload.operation?.id === variables.operationId;
    }
  ),
};

// System subscriptions
export const systemSubscriptions = {
  systemStatusChanged: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.SYSTEM_STATUS_CHANGED,
    'read:system'
  ),

  errorOccurred: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.ERROR_OCCURRED,
    'read:system',
    (payload, variables) => {
      // Filter by error level if provided
      if (variables.level) {
        return payload.error?.level === variables.level;
      }
      return true;
    }
  ),
};

// Agent message subscriptions
export const agentMessageSubscriptions = {
  agentMessageAdded: createSubscriptionResolver(
    SUBSCRIPTION_EVENTS.AGENT_MESSAGE_RECEIVED,
    'read:agents',
    (payload, variables) => {
      if (variables.agentId) {
        return payload.message?.agentId === variables.agentId;
      }
      if (variables.runId) {
        return payload.message?.runId === variables.runId;
      }
      return true;
    }
  ),
};

// Event subscriptions (generic events)
export const eventSubscriptions = {
  eventAdded: createSubscriptionResolver(
    [
      SUBSCRIPTION_EVENTS.WORKTREE_CREATED,
      SUBSCRIPTION_EVENTS.AGENT_SPAWNED,
      SUBSCRIPTION_EVENTS.RUN_STARTED,
      SUBSCRIPTION_EVENTS.MAPROOM_INDEX_STARTED,
      SUBSCRIPTION_EVENTS.GIT_COMMIT,
      SUBSCRIPTION_EVENTS.CONFIG_CHANGED,
    ],
    'read:events',
    (payload, variables) => {
      // Filter by event type if provided
      if (variables.type) {
        return payload.event === variables.type;
      }
      return true;
    }
  ),
};

// Combine all subscription resolvers
export const subscriptionResolvers = {
  // Worktree subscriptions
  ...worktreeSubscriptions,
  
  // Agent subscriptions
  ...agentSubscriptions,
  
  // Run subscriptions
  ...runSubscriptions,
  
  // Maproom subscriptions
  ...maproomSubscriptions,
  
  // Configuration subscriptions
  ...configurationSubscriptions,
  
  // File system subscriptions
  ...fileSystemSubscriptions,
  
  // Git subscriptions
  ...gitSubscriptions,
  
  // System subscriptions
  ...systemSubscriptions,
  
  // Agent message subscriptions
  ...agentMessageSubscriptions,
  
  // Event subscriptions
  ...eventSubscriptions,
};

// Helper function to broadcast events
export async function broadcastSubscriptionEvent(
  event: SubscriptionEvent,
  payload: any,
  filters?: {
    userId?: string;
    workspaceId?: string;
    entityId?: string;
  }
): Promise<void> {
  const pubsub = getPubSub();
  
  try {
    // Enrich payload with metadata
    const enrichedPayload = {
      ...payload,
      timestamp: new Date().toISOString(),
      event,
      filters: filters || {},
    };

    // Publish to main event channel
    await pubsub.publish(event, enrichedPayload);
    
    // Publish to user-specific channels if userId provided
    if (filters?.userId) {
      await pubsub.publish(`${event}_USER_${filters.userId}`, enrichedPayload);
    }
    
    // Publish to workspace-specific channels if workspaceId provided  
    if (filters?.workspaceId) {
      await pubsub.publish(`${event}_WORKSPACE_${filters.workspaceId}`, enrichedPayload);
    }
    
    console.log(`📡 Broadcasted subscription event: ${event}`);
  } catch (error) {
    console.error('Failed to broadcast subscription event:', event, error);
    // Don't throw - we don't want to break the main operation
  }
}