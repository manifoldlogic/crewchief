#!/usr/bin/env node

/**
 * Simple test script to verify GraphQL subscriptions are working
 * Run with: node test-subscriptions.js
 */

import { createClient } from 'graphql-ws';
import WebSocket from 'ws';

// Mock WebSocket for Node.js environment
global.WebSocket = WebSocket;

const GRAPHQL_WS_URL = 'ws://localhost:3456/graphql';

async function testSubscriptions() {
  console.log('🧪 Testing GraphQL Subscriptions...');
  
  try {
    // Create WebSocket client
    const client = createClient({
      url: GRAPHQL_WS_URL,
      webSocketImpl: WebSocket,
      connectionParams: {
        authorization: 'Bearer test-token', // Mock token for testing
      },
      retryAttempts: 3,
      on: {
        connected: () => console.log('✅ Connected to GraphQL subscriptions'),
        closed: () => console.log('🔌 Disconnected from GraphQL subscriptions'),
        error: (error) => console.error('❌ Connection error:', error),
      },
    });

    // Test 1: Simple health subscription
    console.log('\n📡 Test 1: System status subscription...');
    const systemStatusSubscription = `
      subscription {
        systemStatusChanged {
          component
          status
          message
          timestamp
        }
      }
    `;

    let messageCount = 0;
    const maxMessages = 3;

    const unsubscribe = client.subscribe(
      { query: systemStatusSubscription },
      {
        next: (data) => {
          messageCount++;
          console.log(`📨 Message ${messageCount}:`, JSON.stringify(data, null, 2));
          
          if (messageCount >= maxMessages) {
            console.log('✅ Test 1 completed - received expected messages');
            unsubscribe();
            testWorktreeSubscription();
          }
        },
        error: (error) => {
          console.error('❌ Subscription error:', error);
          cleanup();
        },
        complete: () => {
          console.log('🔚 Subscription completed');
        },
      }
    );

    // Test 2: Worktree subscription
    function testWorktreeSubscription() {
      console.log('\n📡 Test 2: Worktree updates subscription...');
      const worktreeSubscription = `
        subscription {
          worktreeStatusChanged {
            id
            name
            status
            updatedAt
            isClean
            isSynced
          }
        }
      `;

      let worktreeMessageCount = 0;
      const worktreeUnsubscribe = client.subscribe(
        { query: worktreeSubscription },
        {
          next: (data) => {
            worktreeMessageCount++;
            console.log(`📨 Worktree Message ${worktreeMessageCount}:`, JSON.stringify(data, null, 2));
            
            if (worktreeMessageCount >= 2) {
              console.log('✅ Test 2 completed - received worktree updates');
              worktreeUnsubscribe();
              testAgentSubscription();
            }
          },
          error: (error) => {
            console.error('❌ Worktree subscription error:', error);
            cleanup();
          },
        }
      );
    }

    // Test 3: Agent subscription
    function testAgentSubscription() {
      console.log('\n📡 Test 3: Agent status subscription...');
      const agentSubscription = `
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

      let agentMessageCount = 0;
      const agentUnsubscribe = client.subscribe(
        { query: agentSubscription },
        {
          next: (data) => {
            agentMessageCount++;
            console.log(`📨 Agent Message ${agentMessageCount}:`, JSON.stringify(data, null, 2));
            
            if (agentMessageCount >= 2) {
              console.log('✅ Test 3 completed - received agent updates');
              agentUnsubscribe();
              console.log('\n🎉 All subscription tests completed successfully!');
              cleanup();
            }
          },
          error: (error) => {
            console.error('❌ Agent subscription error:', error);
            cleanup();
          },
        }
      );
    }

    // Cleanup function
    function cleanup() {
      console.log('\n🧹 Cleaning up...');
      client.dispose();
      process.exit(0);
    }

    // Handle process termination
    process.on('SIGINT', cleanup);
    process.on('SIGTERM', cleanup);

    // Set a timeout for the test
    setTimeout(() => {
      console.log('\n⏰ Test timeout - cleaning up...');
      cleanup();
    }, 30000); // 30 seconds

  } catch (error) {
    console.error('❌ Test failed:', error);
    process.exit(1);
  }
}

// Check if server is running first
async function checkServerHealth() {
  try {
    const response = await fetch('http://localhost:3456/api/health');
    if (response.ok) {
      console.log('✅ Server is running, starting subscription tests...');
      testSubscriptions();
    } else {
      console.error('❌ Server health check failed:', response.status);
      process.exit(1);
    }
  } catch (error) {
    console.error('❌ Cannot connect to server. Make sure the server is running on port 3456');
    console.error('   Start the server with: pnpm dev:server');
    process.exit(1);
  }
}

console.log('🚀 GraphQL Subscription Test Script');
console.log('=====================================');
checkServerHealth();