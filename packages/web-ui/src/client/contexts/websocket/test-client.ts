/**
 * WebSocket Client Test
 * 
 * Test utilities and examples for the WebSocket client integration.
 */

import { WebSocketClient } from './client.js';
import type { WebSocketConfig, AuthToken } from './types.js';

/**
 * Test WebSocket connection with various scenarios
 */
export async function testWebSocketClient(config: Partial<WebSocketConfig> = {}) {
  console.log('🧪 Starting WebSocket Client Tests...');
  
  const client = new WebSocketClient({
    url: 'ws://localhost:3457',
    autoConnect: false,
    maxReconnectAttempts: 3,
    reconnectBaseDelay: 1000,
    ...config,
  });

  // Test 1: Basic Connection
  console.log('\n📡 Test 1: Basic Connection');
  try {
    await client.connect();
    console.log('✅ Connection successful');
    console.log('📊 Stats:', client.stats);
    
    await new Promise(resolve => setTimeout(resolve, 1000));
    client.disconnect();
    console.log('✅ Disconnection successful');
  } catch (error) {
    console.error('❌ Connection failed:', error);
  }

  // Test 2: Authentication
  console.log('\n🔐 Test 2: Authentication');
  const mockToken: AuthToken = {
    accessToken: 'test-jwt-token',
    refreshToken: 'test-refresh-token',
    expiresAt: new Date(Date.now() + 24 * 60 * 60 * 1000), // 24 hours
  };
  
  client.setAuthToken(mockToken);
  
  try {
    await client.connect();
    console.log('✅ Authenticated connection successful');
    console.log('🔒 Auth status:', client.isAuthenticated);
    
    client.disconnect();
  } catch (error) {
    console.error('❌ Authenticated connection failed:', error);
  }

  // Test 3: Message Queue
  console.log('\n📬 Test 3: Message Queue');
  client.sendMessageSync('test-event', { message: 'queued message 1' });
  client.sendMessageSync('test-event', { message: 'queued message 2' });
  console.log('📥 Queue size:', client.queueSize);
  
  try {
    await client.connect();
    console.log('✅ Connection established, processing queue');
    await new Promise(resolve => setTimeout(resolve, 2000));
    console.log('📤 Queue size after processing:', client.queueSize);
    
    client.disconnect();
  } catch (error) {
    console.error('❌ Queue processing failed:', error);
  }

  // Test 4: Reconnection
  console.log('\n🔄 Test 4: Reconnection Logic');
  client.on('connectionStateChange', (state) => {
    console.log(`🔄 Connection state: ${state}`);
  });
  
  client.on('reconnectAttempt', (attempts) => {
    console.log(`🔄 Reconnect attempt: ${attempts}`);
  });
  
  try {
    await client.connect();
    console.log('✅ Initial connection successful');
    
    // Simulate server disconnect
    console.log('🔌 Simulating disconnect...');
    // Note: In a real test, you would disconnect from the server side
    
    await new Promise(resolve => setTimeout(resolve, 5000));
    client.disconnect();
  } catch (error) {
    console.error('❌ Reconnection test failed:', error);
  }

  console.log('\n🧪 WebSocket Client Tests Complete');
}

/**
 * Example usage for components
 */
export function createExampleUsage() {
  return `
// Example 1: Basic Usage in React Component
import React from 'react';
import { WebSocketProvider, useWebSocket } from './contexts/websocket';

function App() {
  return (
    <WebSocketProvider config={{ autoConnect: true }}>
      <Dashboard />
    </WebSocketProvider>
  );
}

function Dashboard() {
  const {
    connected,
    connecting,
    error,
    stats,
    activities,
    agents,
    connect,
    disconnect,
    refreshStats,
  } = useWebSocket();

  if (connecting) return <div>Connecting...</div>;
  if (error) return <div>Error: {error}</div>;
  if (!connected) return <button onClick={connect}>Connect</button>;

  return (
    <div>
      <h1>Dashboard</h1>
      <button onClick={refreshStats}>Refresh</button>
      <button onClick={disconnect}>Disconnect</button>
      
      <div>Stats: {JSON.stringify(stats)}</div>
      <div>Activities: {activities.length}</div>
      <div>Agents: {agents.length}</div>
    </div>
  );
}

// Example 2: Using Specific Hooks
import { 
  useWebSocketConnection,
  useWebSocketDashboard,
  useWebSocketMessaging,
} from './contexts/websocket';

function ConnectionStatus() {
  const { connectionState, isConnected, error, reconnectAttempts } = useWebSocketConnection();
  
  return (
    <div>
      <div>Status: {connectionState}</div>
      <div>Connected: {isConnected ? 'Yes' : 'No'}</div>
      <div>Reconnect Attempts: {reconnectAttempts}</div>
      {error && <div>Error: {error.message}</div>}
    </div>
  );
}

function MessageSender() {
  const { sendMessage, sendMessageSync, queueSize } = useWebSocketMessaging();
  
  const handleSendMessage = async () => {
    try {
      await sendMessage('test-event', { data: 'Hello WebSocket!' });
      console.log('Message sent successfully');
    } catch (error) {
      console.error('Failed to send message:', error);
    }
  };
  
  return (
    <div>
      <button onClick={handleSendMessage}>Send Message</button>
      <div>Queue Size: {queueSize}</div>
    </div>
  );
}

// Example 3: Authentication Integration
import { useWebSocketAuth } from './contexts/websocket';

function AuthenticatedWebSocket() {
  const { isAuthenticated, setAuthToken, refreshAuth } = useWebSocketAuth();
  
  // This would typically come from your auth context
  const handleLogin = (token: string) => {
    setAuthToken({
      accessToken: token,
      expiresAt: new Date(Date.now() + 24 * 60 * 60 * 1000),
    });
  };
  
  const handleLogout = () => {
    setAuthToken(null);
  };
  
  return (
    <div>
      <div>Authenticated: {isAuthenticated ? 'Yes' : 'No'}</div>
      <button onClick={() => handleLogin('mock-token')}>Login</button>
      <button onClick={handleLogout}>Logout</button>
      <button onClick={refreshAuth}>Refresh Auth</button>
    </div>
  );
}

// Example 4: Room Management
import { useWebSocketRooms } from './contexts/websocket';

function RoomManager() {
  const { joinedRooms, joinRoom, leaveRoom } = useWebSocketRooms();
  
  const handleJoinRoom = async (roomName: string) => {
    try {
      await joinRoom(roomName);
      console.log(\`Joined room: \${roomName}\`);
    } catch (error) {
      console.error('Failed to join room:', error);
    }
  };
  
  return (
    <div>
      <div>Joined Rooms: {joinedRooms.length}</div>
      {joinedRooms.map(room => (
        <div key={room.name}>
          {room.name} ({room.memberCount} members)
          <button onClick={() => leaveRoom(room.name)}>Leave</button>
        </div>
      ))}
      <button onClick={() => handleJoinRoom('general')}>Join General</button>
    </div>
  );
}
`;
}

/**
 * Performance test for message handling
 */
export async function performanceTest() {
  console.log('⚡ Starting Performance Tests...');
  
  const client = new WebSocketClient({
    url: 'ws://localhost:3457',
    messageQueueMaxSize: 1000,
  });

  try {
    await client.connect();
    
    // Test message throughput
    const messageCount = 100;
    const startTime = performance.now();
    
    for (let i = 0; i < messageCount; i++) {
      client.sendMessageSync('perf-test', { 
        index: i, 
        timestamp: Date.now(),
        data: new Array(100).fill('x').join(''), // 100 character payload
      });
    }
    
    const endTime = performance.now();
    const duration = endTime - startTime;
    const messagesPerSecond = (messageCount / duration) * 1000;
    
    console.log(\`📊 Performance Results:\`);
    console.log(\`   Messages: \${messageCount}\`);
    console.log(\`   Duration: \${duration.toFixed(2)}ms\`);
    console.log(\`   Rate: \${messagesPerSecond.toFixed(2)} messages/second\`);
    console.log(\`   Queue Size: \${client.queueSize}\`);
    
    // Wait for queue to process
    await new Promise(resolve => setTimeout(resolve, 5000));
    console.log(\`   Final Queue Size: \${client.queueSize}\`);
    
    client.disconnect();
    console.log('✅ Performance test completed');
    
  } catch (error) {
    console.error('❌ Performance test failed:', error);
  }
}

/**
 * Stress test for reconnection
 */
export async function reconnectionStressTest() {
  console.log('🔄 Starting Reconnection Stress Test...');
  
  const client = new WebSocketClient({
    url: 'ws://localhost:3457',
    maxReconnectAttempts: 10,
    reconnectBaseDelay: 500,
  });

  let reconnectCount = 0;
  client.on('reconnectAttempt', () => {
    reconnectCount++;
    console.log(\`🔄 Reconnect attempt: \${reconnectCount}\`);
  });

  try {
    // Connect and disconnect rapidly
    for (let i = 0; i < 5; i++) {
      console.log(\`🔌 Connection cycle \${i + 1}\`);
      await client.connect();
      await new Promise(resolve => setTimeout(resolve, 1000));
      client.disconnect();
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    
    console.log(\`✅ Stress test completed. Total reconnects: \${reconnectCount}\`);
    
  } catch (error) {
    console.error('❌ Stress test failed:', error);
  }
}

// Export test runner
export function runAllTests() {
  console.log('🚀 Running All WebSocket Tests...');
  
  return Promise.all([
    testWebSocketClient(),
    performanceTest(),
    reconnectionStressTest(),
  ]).then(() => {
    console.log('✅ All tests completed');
  }).catch((error) => {
    console.error('❌ Test suite failed:', error);
  });
}