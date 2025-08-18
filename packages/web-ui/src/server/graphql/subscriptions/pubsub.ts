import { PubSub } from 'graphql-subscriptions';
import { RedisPubSub } from 'graphql-redis-subscriptions';
import Redis from 'ioredis';

export interface PubSubInterface {
  publish(triggerName: string, payload: any): Promise<void>;
  subscribe(triggerName: string, onMessage: (payload: any) => void): Promise<number>;
  unsubscribe(subId: number): void;
  asyncIterator<T = any>(triggers: string | string[]): AsyncIterator<T>;
  close(): Promise<void>;
}

class FallbackPubSub implements PubSubInterface {
  private pubsub: PubSub;

  constructor() {
    this.pubsub = new PubSub();
  }

  async publish(triggerName: string, payload: any): Promise<void> {
    return this.pubsub.publish(triggerName, payload);
  }

  async subscribe(triggerName: string, onMessage: (payload: any) => void): Promise<number> {
    return this.pubsub.subscribe(triggerName, onMessage);
  }

  unsubscribe(subId: number): void {
    this.pubsub.unsubscribe(subId);
  }

  asyncIterator<T = any>(triggers: string | string[]): AsyncIterator<T> {
    return this.pubsub.asyncIterator(triggers);
  }

  async close(): Promise<void> {
    // In-memory PubSub doesn't need explicit cleanup
    return Promise.resolve();
  }
}

class RedisBasedPubSub implements PubSubInterface {
  private pubsub: RedisPubSub;

  constructor(redisUrl: string) {
    const redisOptions = {
      host: new URL(redisUrl).hostname,
      port: parseInt(new URL(redisUrl).port) || 6379,
      retryDelayOnFailover: 100,
      enableOfflineQueue: false,
      lazyConnect: true,
      maxRetriesPerRequest: 3,
    };

    this.pubsub = new RedisPubSub({
      publisher: new Redis(redisOptions),
      subscriber: new Redis(redisOptions),
    });
  }

  async publish(triggerName: string, payload: any): Promise<void> {
    return this.pubsub.publish(triggerName, payload);
  }

  async subscribe(triggerName: string, onMessage: (payload: any) => void): Promise<number> {
    return this.pubsub.subscribe(triggerName, onMessage);
  }

  unsubscribe(subId: number): void {
    this.pubsub.unsubscribe(subId);
  }

  asyncIterator<T = any>(triggers: string | string[]): AsyncIterator<T> {
    return this.pubsub.asyncIterator(triggers);
  }

  async close(): Promise<void> {
    await this.pubsub.close();
  }
}

// Subscription event types
export const SUBSCRIPTION_EVENTS = {
  // Worktree events
  WORKTREE_CREATED: 'WORKTREE_CREATED',
  WORKTREE_DELETED: 'WORKTREE_DELETED',
  WORKTREE_STATUS_CHANGED: 'WORKTREE_STATUS_CHANGED',
  WORKTREE_UPDATED: 'WORKTREE_UPDATED',

  // Agent events
  AGENT_SPAWNED: 'AGENT_SPAWNED',
  AGENT_STATUS_CHANGED: 'AGENT_STATUS_CHANGED',
  AGENT_COMPLETED: 'AGENT_COMPLETED',
  AGENT_FAILED: 'AGENT_FAILED',
  AGENT_MESSAGE_RECEIVED: 'AGENT_MESSAGE_RECEIVED',

  // Run events
  RUN_STARTED: 'RUN_STARTED',
  RUN_COMPLETED: 'RUN_COMPLETED',
  RUN_FAILED: 'RUN_FAILED',
  RUN_STATUS_CHANGED: 'RUN_STATUS_CHANGED',

  // Maproom events
  MAPROOM_INDEX_STARTED: 'MAPROOM_INDEX_STARTED',
  MAPROOM_INDEX_PROGRESS: 'MAPROOM_INDEX_PROGRESS',
  MAPROOM_INDEX_COMPLETED: 'MAPROOM_INDEX_COMPLETED',
  MAPROOM_INDEX_FAILED: 'MAPROOM_INDEX_FAILED',

  // File system events
  FILE_CHANGED: 'FILE_CHANGED',
  FILE_CREATED: 'FILE_CREATED',
  FILE_DELETED: 'FILE_DELETED',
  DIRECTORY_CHANGED: 'DIRECTORY_CHANGED',

  // Git events
  GIT_COMMIT: 'GIT_COMMIT',
  GIT_BRANCH_CHANGED: 'GIT_BRANCH_CHANGED',
  GIT_MERGE: 'GIT_MERGE',
  GIT_OPERATION_PROGRESS: 'GIT_OPERATION_PROGRESS',

  // Configuration events
  CONFIG_CHANGED: 'CONFIG_CHANGED',
  CONFIG_VALIDATED: 'CONFIG_VALIDATED',

  // System events
  SYSTEM_STATUS_CHANGED: 'SYSTEM_STATUS_CHANGED',
  ERROR_OCCURRED: 'ERROR_OCCURRED',
} as const;

export type SubscriptionEvent = typeof SUBSCRIPTION_EVENTS[keyof typeof SUBSCRIPTION_EVENTS];

// Create and export singleton PubSub instance
let pubsubInstance: PubSubInterface | null = null;

export function createPubSub(): PubSubInterface {
  if (pubsubInstance) {
    return pubsubInstance;
  }

  const redisUrl = process.env.REDIS_URL;
  
  if (redisUrl) {
    try {
      console.log('🔴 Initializing Redis PubSub...');
      pubsubInstance = new RedisBasedPubSub(redisUrl);
      
      // Test Redis connection
      const testConnection = async () => {
        try {
          await pubsubInstance!.publish('test_connection', { test: true });
          console.log('✅ Redis PubSub connection successful');
        } catch (error) {
          console.warn('⚠️ Redis PubSub connection failed, falling back to in-memory:', error);
          pubsubInstance = new FallbackPubSub();
          console.log('📝 Using in-memory PubSub');
        }
      };
      
      // Don't await this, let it run in background
      testConnection();
      
    } catch (error) {
      console.warn('⚠️ Failed to initialize Redis PubSub, falling back to in-memory:', error);
      pubsubInstance = new FallbackPubSub();
      console.log('📝 Using in-memory PubSub');
    }
  } else {
    console.log('📝 No Redis URL provided, using in-memory PubSub');
    pubsubInstance = new FallbackPubSub();
  }

  return pubsubInstance;
}

export function getPubSub(): PubSubInterface {
  if (!pubsubInstance) {
    return createPubSub();
  }
  return pubsubInstance;
}

// Helper function to publish events with error handling
export async function publishEvent(
  event: SubscriptionEvent,
  payload: any,
  filters?: { userId?: string; workspaceId?: string; entityId?: string }
): Promise<void> {
  try {
    const pubsub = getPubSub();
    
    // Add metadata to payload
    const enrichedPayload = {
      ...payload,
      timestamp: new Date().toISOString(),
      event,
      filters: filters || {},
    };

    await pubsub.publish(event, enrichedPayload);
    
    // Also publish to user-specific channels if userId is provided
    if (filters?.userId) {
      await pubsub.publish(`${event}_USER_${filters.userId}`, enrichedPayload);
    }
    
    // Also publish to workspace-specific channels if workspaceId is provided
    if (filters?.workspaceId) {
      await pubsub.publish(`${event}_WORKSPACE_${filters.workspaceId}`, enrichedPayload);
    }
    
  } catch (error) {
    console.error('Failed to publish event:', event, error);
    // Don't throw - we don't want to break the main operation if pub/sub fails
  }
}

// Helper function to create filtered async iterator
export function createFilteredAsyncIterator<T = any>(
  event: SubscriptionEvent | SubscriptionEvent[],
  filter?: (payload: T) => boolean,
  userId?: string,
  workspaceId?: string
): AsyncIterator<T> {
  const pubsub = getPubSub();
  
  // Build list of events to subscribe to
  const events = Array.isArray(event) ? event : [event];
  const subscriptionEvents: string[] = [];
  
  events.forEach(evt => {
    // Subscribe to global event
    subscriptionEvents.push(evt);
    
    // Subscribe to user-specific event if userId provided
    if (userId) {
      subscriptionEvents.push(`${evt}_USER_${userId}`);
    }
    
    // Subscribe to workspace-specific event if workspaceId provided
    if (workspaceId) {
      subscriptionEvents.push(`${evt}_WORKSPACE_${workspaceId}`);
    }
  });
  
  const iterator = pubsub.asyncIterator<T>(subscriptionEvents);
  
  if (!filter) {
    return iterator;
  }
  
  // Wrap iterator with filter
  return {
    [Symbol.asyncIterator]() {
      return this;
    },
    async next() {
      while (true) {
        const result = await iterator.next();
        if (result.done) {
          return result;
        }
        
        if (filter(result.value)) {
          return result;
        }
      }
    },
    async return(value?: any) {
      return iterator.return ? await iterator.return(value) : { done: true, value };
    },
    async throw(e: any) {
      return iterator.throw ? await iterator.throw(e) : Promise.reject(e);
    },
  };
}

// Cleanup function
export async function closePubSub(): Promise<void> {
  if (pubsubInstance) {
    await pubsubInstance.close();
    pubsubInstance = null;
    console.log('🔴 PubSub connection closed');
  }
}