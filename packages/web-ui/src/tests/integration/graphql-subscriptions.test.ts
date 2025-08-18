import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import { createClient } from 'graphql-ws';
import WebSocket from 'ws';
import { Server } from 'http';
import express from 'express';
import { setupGraphQLEndpoint } from '../../server/graphql/apollo.js';
import { getDatabaseConnection } from '../../db/connection.js';
import { publishEvent, SUBSCRIPTION_EVENTS } from '../../server/graphql/subscriptions/pubsub.js';

// Mock WebSocket for Node.js environment
global.WebSocket = WebSocket as any;

const TEST_PORT = 3457;

describe('GraphQL Subscriptions Integration', () => {
  let server: Server;
  let app: express.Express;
  let client: any;
  let wsCleanup: (() => Promise<void>) | undefined;

  beforeAll(async () => {
    // Create Express app and HTTP server
    app = express();
    server = Server(app);

    // Get database connection
    const db = await getDatabaseConnection({
      host: process.env.DB_HOST || 'localhost',
      port: parseInt(process.env.DB_PORT || '5432'),
      database: process.env.DB_NAME || 'crewchief_test',
      user: process.env.DB_USER || 'crewchief',
      password: process.env.DB_PASSWORD || 'crewchief',
    });

    // Setup GraphQL with subscriptions
    const result = await setupGraphQLEndpoint(app, server, db, '/graphql', true);
    wsCleanup = result.wsCleanup;

    // Start server
    await new Promise<void>((resolve) => {
      server.listen(TEST_PORT, resolve);
    });

    console.log(`Test server running on port ${TEST_PORT}`);
  });

  afterAll(async () => {
    if (client) {
      await client.dispose();
    }

    if (wsCleanup) {
      await wsCleanup();
    }

    if (server) {
      await new Promise<void>((resolve) => {
        server.close(() => resolve());
      });
    }
  });

  beforeEach(() => {
    // Create WebSocket client for each test
    client = createClient({
      url: `ws://localhost:${TEST_PORT}/graphql`,
      webSocketImpl: WebSocket,
      connectionParams: {
        authorization: 'Bearer test-token', // Mock token
      },
    });
  });

  afterEach(async () => {
    if (client) {
      await client.dispose();
      client = null;
    }
  });

  it('should connect to GraphQL subscriptions via WebSocket', async () => {
    const connectionPromise = new Promise((resolve, reject) => {
      client.on('connected', resolve);
      client.on('closed', reject);
      
      setTimeout(() => reject(new Error('Connection timeout')), 5000);
    });

    await connectionPromise;
    expect(true).toBe(true); // Connection successful
  });

  it('should receive worktree updates via subscription', async () => {
    const subscription = `
      subscription {
        worktreeStatusChanged {
          id
          name
          status
          updatedAt
        }
      }
    `;

    const messages: any[] = [];
    const messagePromise = new Promise((resolve, reject) => {
      client.subscribe(
        {
          query: subscription,
        },
        {
          next: (data: any) => {
            messages.push(data);
            resolve(data);
          },
          error: reject,
          complete: () => {},
        }
      );

      setTimeout(() => reject(new Error('Subscription timeout')), 10000);
    });

    // Wait a bit for subscription to be established
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Publish a test event
    await publishEvent(SUBSCRIPTION_EVENTS.WORKTREE_STATUS_CHANGED, {
      worktree: {
        id: 'test-worktree-1',
        name: 'test-worktree',
        status: 'ACTIVE',
        updatedAt: new Date().toISOString(),
      },
    });

    const result = await messagePromise;
    expect(result).toBeDefined();
    expect((result as any).data.worktreeStatusChanged).toBeDefined();
    expect((result as any).data.worktreeStatusChanged.id).toBe('test-worktree-1');
  });

  it('should receive agent status updates via subscription', async () => {
    const subscription = `
      subscription {
        agentStatusChanged {
          id
          name
          status
          worktreeId
          updatedAt
        }
      }
    `;

    const messagePromise = new Promise((resolve, reject) => {
      client.subscribe(
        {
          query: subscription,
        },
        {
          next: (data: any) => resolve(data),
          error: reject,
          complete: () => {},
        }
      );

      setTimeout(() => reject(new Error('Subscription timeout')), 10000);
    });

    // Wait for subscription to be established
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Publish test event
    await publishEvent(SUBSCRIPTION_EVENTS.AGENT_STATUS_CHANGED, {
      agent: {
        id: 'test-agent-1',
        name: 'test-agent',
        status: 'RUNNING',
        worktreeId: 'test-worktree-1',
        updatedAt: new Date().toISOString(),
      },
    });

    const result = await messagePromise;
    expect(result).toBeDefined();
    expect((result as any).data.agentStatusChanged).toBeDefined();
    expect((result as any).data.agentStatusChanged.id).toBe('test-agent-1');
  });

  it('should receive maproom indexing updates via subscription', async () => {
    const subscription = `
      subscription {
        indexingStatusChanged {
          id
          worktreeId
          status
          filesIndexed
          totalFiles
          indexingProgress
        }
      }
    `;

    const messagePromise = new Promise((resolve, reject) => {
      client.subscribe(
        {
          query: subscription,
        },
        {
          next: (data: any) => resolve(data),
          error: reject,
          complete: () => {},
        }
      );

      setTimeout(() => reject(new Error('Subscription timeout')), 10000);
    });

    // Wait for subscription to be established
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Publish test event
    await publishEvent(SUBSCRIPTION_EVENTS.MAPROOM_INDEX_PROGRESS, {
      index: {
        id: 'test-index-1',
        worktreeId: 'test-worktree-1',
        status: 'INDEXING',
        filesIndexed: 150,
        totalFiles: 500,
        indexingProgress: 30,
      },
    });

    const result = await messagePromise;
    expect(result).toBeDefined();
    expect((result as any).data.indexingStatusChanged).toBeDefined();
    expect((result as any).data.indexingStatusChanged.status).toBe('INDEXING');
  });

  it('should handle multiple concurrent subscribers', async () => {
    const subscription = `
      subscription {
        agentStatusChanged {
          id
          status
        }
      }
    `;

    // Create multiple clients
    const clients = Array.from({ length: 5 }, () =>
      createClient({
        url: `ws://localhost:${TEST_PORT}/graphql`,
        webSocketImpl: WebSocket,
        connectionParams: {
          authorization: 'Bearer test-token',
        },
      })
    );

    const messagePromises = clients.map((client, index) => 
      new Promise((resolve, reject) => {
        client.subscribe(
          { query: subscription },
          {
            next: (data: any) => {
              console.log(`Client ${index} received:`, data);
              resolve(data);
            },
            error: reject,
            complete: () => {},
          }
        );

        setTimeout(() => reject(new Error(`Client ${index} subscription timeout`)), 10000);
      })
    );

    // Wait for all subscriptions to be established
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Publish test event
    await publishEvent(SUBSCRIPTION_EVENTS.AGENT_STATUS_CHANGED, {
      agent: {
        id: 'test-agent-concurrent',
        status: 'COMPLETED',
      },
    });

    // All clients should receive the message
    const results = await Promise.all(messagePromises);
    expect(results.length).toBe(5);

    results.forEach(result => {
      expect(result).toBeDefined();
      expect((result as any).data.agentStatusChanged.id).toBe('test-agent-concurrent');
    });

    // Cleanup clients
    await Promise.all(clients.map(client => client.dispose()));
  });

  it('should handle subscription filtering by variables', async () => {
    const subscription = `
      subscription($id: ID) {
        worktreeUpdated(id: $id) {
          id
          name
          status
        }
      }
    `;

    let receivedMessages = 0;
    const messagePromise = new Promise((resolve, reject) => {
      client.subscribe(
        {
          query: subscription,
          variables: { id: 'target-worktree' },
        },
        {
          next: (data: any) => {
            receivedMessages++;
            if (data.data?.worktreeUpdated?.id === 'target-worktree') {
              resolve(data);
            }
          },
          error: reject,
          complete: () => {},
        }
      );

      setTimeout(() => reject(new Error('Subscription timeout')), 10000);
    });

    // Wait for subscription to be established
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Publish events for different worktrees
    await publishEvent(SUBSCRIPTION_EVENTS.WORKTREE_UPDATED, {
      worktree: {
        id: 'other-worktree',
        name: 'other',
        status: 'ACTIVE',
      },
    });

    await publishEvent(SUBSCRIPTION_EVENTS.WORKTREE_UPDATED, {
      worktree: {
        id: 'target-worktree',
        name: 'target',
        status: 'ACTIVE',
      },
    });

    const result = await messagePromise;
    expect(result).toBeDefined();
    expect((result as any).data.worktreeUpdated.id).toBe('target-worktree');
    
    // Should only receive the targeted message
    await new Promise(resolve => setTimeout(resolve, 1000));
    expect(receivedMessages).toBe(1);
  });

  it('should handle authentication errors gracefully', async () => {
    // Create client without auth token
    const unauthenticatedClient = createClient({
      url: `ws://localhost:${TEST_PORT}/graphql`,
      webSocketImpl: WebSocket,
      connectionParams: {}, // No auth token
    });

    const subscription = `
      subscription {
        agentStatusChanged {
          id
          status
        }
      }
    `;

    const errorPromise = new Promise((resolve, reject) => {
      unauthenticatedClient.subscribe(
        { query: subscription },
        {
          next: (data: any) => {
            reject(new Error('Should not receive data without auth'));
          },
          error: (error: any) => {
            console.log('Expected auth error:', error);
            resolve(error);
          },
          complete: () => {},
        }
      );

      setTimeout(() => reject(new Error('No error received')), 5000);
    });

    // Should receive an authentication error
    const error = await errorPromise;
    expect(error).toBeDefined();

    await unauthenticatedClient.dispose();
  });

  it('should handle network disconnections and reconnections', async () => {
    const subscription = `
      subscription {
        systemStatusChanged {
          component
          status
          message
        }
      }
    `;

    let reconnectCount = 0;
    const messages: any[] = [];
    
    const reconnectingClient = createClient({
      url: `ws://localhost:${TEST_PORT}/graphql`,
      webSocketImpl: WebSocket,
      connectionParams: {
        authorization: 'Bearer test-token',
      },
      retryAttempts: 3,
      shouldRetry: () => {
        reconnectCount++;
        return reconnectCount < 3;
      },
    });

    const messagePromise = new Promise((resolve, reject) => {
      reconnectingClient.subscribe(
        { query: subscription },
        {
          next: (data: any) => {
            messages.push(data);
            if (messages.length >= 2) {
              resolve(messages);
            }
          },
          error: reject,
          complete: () => {},
        }
      );

      setTimeout(() => reject(new Error('Reconnection test timeout')), 15000);
    });

    // Wait for initial connection
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Send first message
    await publishEvent(SUBSCRIPTION_EVENTS.SYSTEM_STATUS_CHANGED, {
      component: 'test',
      status: 'online',
      message: 'Before disconnect',
    });

    // Wait a bit
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Simulate disconnect by terminating the client connection
    reconnectingClient.terminate();

    // Wait for reconnection
    await new Promise(resolve => setTimeout(resolve, 3000));

    // Send second message after reconnection
    await publishEvent(SUBSCRIPTION_EVENTS.SYSTEM_STATUS_CHANGED, {
      component: 'test',
      status: 'online',
      message: 'After reconnect',
    });

    const result = await messagePromise;
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBeGreaterThanOrEqual(1);

    await reconnectingClient.dispose();
  });
});