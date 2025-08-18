#!/usr/bin/env tsx

/**
 * WebSocket Test Client
 * 
 * This script tests the WebSocket server functionality including:
 * - Connection establishment
 * - Authentication
 * - Room joining/leaving
 * - Message sending
 * - Event subscription
 * - Heartbeat/ping-pong
 * - Connection management
 */

import { io, Socket } from 'socket.io-client';
import jwt from 'jsonwebtoken';

interface TestConfig {
  url: string;
  token?: string;
  testDuration: number;
  numClients: number;
}

class WebSocketTestClient {
  private socket: Socket | null = null;
  private clientId: string;
  private config: TestConfig;
  private stats = {
    connected: false,
    messagesReceived: 0,
    messagesSent: 0,
    errors: 0,
    rooms: new Set<string>(),
    subscriptions: new Set<string>(),
  };

  constructor(clientId: string, config: TestConfig) {
    this.clientId = clientId;
    this.config = config;
  }

  public async connect(): Promise<void> {
    console.log(`[${this.clientId}] Connecting to ${this.config.url}...`);

    this.socket = io(this.config.url, {
      auth: {
        token: this.config.token,
      },
      transports: ['websocket'],
      timeout: 5000,
    });

    return new Promise((resolve, reject) => {
      if (!this.socket) {
        reject(new Error('Failed to create socket'));
        return;
      }

      this.socket.on('connect', () => {
        console.log(`[${this.clientId}] ✅ Connected`);
        this.stats.connected = true;
        this.setupEventHandlers();
        resolve();
      });

      this.socket.on('connect_error', (error) => {
        console.error(`[${this.clientId}] ❌ Connection error:`, error.message);
        this.stats.errors++;
        reject(error);
      });

      this.socket.on('disconnect', (reason) => {
        console.log(`[${this.clientId}] 🔌 Disconnected:`, reason);
        this.stats.connected = false;
      });
    });
  }

  private setupEventHandlers(): void {
    if (!this.socket) return;

    this.socket.on('connected', (data) => {
      console.log(`[${this.clientId}] 🎉 Connection acknowledged:`, data);
    });

    this.socket.on('heartbeat', () => {
      // Respond to server heartbeat
    });

    this.socket.on('pong', (data) => {
      console.log(`[${this.clientId}] 🏓 Pong received`);
    });

    this.socket.on('room-joined', (data) => {
      console.log(`[${this.clientId}] 📺 Joined room: ${data.room}`);
      this.stats.rooms.add(data.room);
    });

    this.socket.on('room-left', (data) => {
      console.log(`[${this.clientId}] 📺 Left room: ${data.room}`);
      this.stats.rooms.delete(data.room);
    });

    this.socket.on('subscription-confirmed', (data) => {
      console.log(`[${this.clientId}] 📡 Subscribed to: ${data.type}`);
      this.stats.subscriptions.add(data.type);
    });

    this.socket.on('broadcast-message', (data) => {
      console.log(`[${this.clientId}] 📢 Broadcast:`, data.message || data);
      this.stats.messagesReceived++;
    });

    this.socket.on('room-message', (data) => {
      console.log(`[${this.clientId}] 📺 Room message in ${data.room}:`, data.message);
      this.stats.messagesReceived++;
    });

    this.socket.on('message-sent', (data) => {
      console.log(`[${this.clientId}] ✅ Message sent:`, data.type);
    });

    this.socket.on('error', (error) => {
      console.error(`[${this.clientId}] ❌ Socket error:`, error);
      this.stats.errors++;
    });

    // Real-time update events
    this.socket.on('worktree-update', (data) => {
      console.log(`[${this.clientId}] 🌳 Worktree update:`, data);
      this.stats.messagesReceived++;
    });

    this.socket.on('agent-status-change', (data) => {
      console.log(`[${this.clientId}] 🤖 Agent status:`, data);
      this.stats.messagesReceived++;
    });

    this.socket.on('global-update', (data) => {
      console.log(`[${this.clientId}] 🌐 Global update:`, data.type);
      this.stats.messagesReceived++;
    });
  }

  public async runTests(): Promise<void> {
    if (!this.socket?.connected) {
      throw new Error('Not connected');
    }

    console.log(`[${this.clientId}] 🧪 Starting tests...`);

    // Test 1: Send ping
    await this.testPing();

    // Test 2: Join a room
    await this.testRoomJoin();

    // Test 3: Subscribe to events
    await this.testEventSubscription();

    // Test 4: Send messages
    await this.testMessageSending();

    // Test 5: Load testing
    await this.testLoadTesting();

    console.log(`[${this.clientId}] ✅ All tests completed`);
  }

  private async testPing(): Promise<void> {
    console.log(`[${this.clientId}] Testing ping...`);
    this.socket!.emit('ping');
    await this.sleep(1000);
  }

  private async testRoomJoin(): Promise<void> {
    console.log(`[${this.clientId}] Testing room join...`);
    
    const roomName = `test-room-${this.clientId}`;
    
    return new Promise((resolve, reject) => {
      this.socket!.emit('join-room', { room: roomName, type: 'test' });
      
      const timeout = setTimeout(() => {
        reject(new Error('Room join timeout'));
      }, 5000);

      this.socket!.once('room-joined', (data) => {
        clearTimeout(timeout);
        if (data.room === roomName) {
          console.log(`[${this.clientId}] ✅ Successfully joined room: ${roomName}`);
          resolve();
        }
      });
    });
  }

  private async testEventSubscription(): Promise<void> {
    console.log(`[${this.clientId}] Testing event subscription...`);
    
    return new Promise((resolve, reject) => {
      this.socket!.emit('subscribe', { 
        type: 'system-updates',
        filters: { clientId: this.clientId } 
      });
      
      const timeout = setTimeout(() => {
        reject(new Error('Subscription timeout'));
      }, 5000);

      this.socket!.once('subscription-confirmed', (data) => {
        clearTimeout(timeout);
        if (data.type === 'system-updates') {
          console.log(`[${this.clientId}] ✅ Successfully subscribed to system-updates`);
          resolve();
        }
      });
    });
  }

  private async testMessageSending(): Promise<void> {
    console.log(`[${this.clientId}] Testing message sending...`);
    
    // Send echo message
    this.socket!.emit('message', {
      type: 'echo',
      message: `Hello from ${this.clientId}`,
      timestamp: new Date().toISOString(),
    });
    this.stats.messagesSent++;

    // Send room message
    this.socket!.emit('message', {
      type: 'room-message',
      room: `test-room-${this.clientId}`,
      message: `Room message from ${this.clientId}`,
    });
    this.stats.messagesSent++;

    // Send broadcast (if authenticated)
    if (this.config.token) {
      this.socket!.emit('message', {
        type: 'broadcast',
        payload: {
          message: `Broadcast from ${this.clientId}`,
        },
      });
      this.stats.messagesSent++;
    }

    await this.sleep(2000);
  }

  private async testLoadTesting(): Promise<void> {
    console.log(`[${this.clientId}] Testing load (sending 10 messages)...`);
    
    for (let i = 0; i < 10; i++) {
      this.socket!.emit('message', {
        type: 'echo',
        message: `Load test message ${i + 1} from ${this.clientId}`,
        timestamp: new Date().toISOString(),
      });
      this.stats.messagesSent++;
      
      await this.sleep(100); // 100ms between messages
    }
  }

  public getStats() {
    return {
      clientId: this.clientId,
      ...this.stats,
      rooms: Array.from(this.stats.rooms),
      subscriptions: Array.from(this.stats.subscriptions),
    };
  }

  public disconnect(): void {
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// Main test runner
async function runWebSocketTests() {
  console.log('🧪 Starting WebSocket Server Tests');
  console.log('=====================================');

  const config: TestConfig = {
    url: process.env.WS_URL || 'http://localhost:3456',
    token: undefined, // Will generate a test token
    testDuration: 30000, // 30 seconds
    numClients: 5,
  };

  // Generate a test JWT token
  try {
    const jwtSecret = process.env.JWT_SECRET || 'development-secret-key';
    config.token = jwt.sign(
      {
        userId: 'test-user-' + Date.now(),
        username: 'TestUser',
        email: 'test@example.com',
      },
      jwtSecret,
      { expiresIn: '1h' }
    );
    console.log('✅ Generated test JWT token');
  } catch (error) {
    console.warn('⚠️  Could not generate JWT token, testing without authentication');
  }

  const clients: WebSocketTestClient[] = [];
  const results: any[] = [];

  try {
    // Create and connect clients
    for (let i = 0; i < config.numClients; i++) {
      const client = new WebSocketTestClient(`client-${i + 1}`, config);
      clients.push(client);
      
      try {
        await client.connect();
        console.log(`✅ Client ${i + 1} connected`);
      } catch (error) {
        console.error(`❌ Client ${i + 1} failed to connect:`, error);
        continue;
      }
    }

    console.log(`\n🚀 Running tests with ${clients.length} connected clients...`);

    // Run tests on all clients concurrently
    const testPromises = clients.map(async (client, index) => {
      try {
        await client.runTests();
        console.log(`✅ Client ${index + 1} tests completed`);
      } catch (error) {
        console.error(`❌ Client ${index + 1} tests failed:`, error);
      }
      return client.getStats();
    });

    // Wait for all tests to complete or timeout
    const testResults = await Promise.allSettled(testPromises);
    
    testResults.forEach((result, index) => {
      if (result.status === 'fulfilled') {
        results.push(result.value);
      } else {
        console.error(`Client ${index + 1} test rejected:`, result.reason);
      }
    });

    // Keep connections alive for a bit to test sustained load
    console.log(`\n⏱️  Keeping connections alive for ${config.testDuration / 1000}s...`);
    await new Promise(resolve => setTimeout(resolve, config.testDuration));

  } finally {
    // Clean up all clients
    console.log('\n🧹 Cleaning up clients...');
    clients.forEach((client, index) => {
      try {
        client.disconnect();
        console.log(`✅ Client ${index + 1} disconnected`);
      } catch (error) {
        console.error(`❌ Error disconnecting client ${index + 1}:`, error);
      }
    });
  }

  // Print final results
  console.log('\n📊 Test Results Summary');
  console.log('========================');
  
  const totalStats = results.reduce((acc, stats) => ({
    clients: acc.clients + 1,
    messagesReceived: acc.messagesReceived + stats.messagesReceived,
    messagesSent: acc.messagesSent + stats.messagesSent,
    errors: acc.errors + stats.errors,
    rooms: acc.rooms + stats.rooms.length,
    subscriptions: acc.subscriptions + stats.subscriptions.length,
  }), {
    clients: 0,
    messagesReceived: 0,
    messagesSent: 0,
    errors: 0,
    rooms: 0,
    subscriptions: 0,
  });

  console.log(`Clients: ${totalStats.clients}`);
  console.log(`Messages Sent: ${totalStats.messagesSent}`);
  console.log(`Messages Received: ${totalStats.messagesReceived}`);
  console.log(`Errors: ${totalStats.errors}`);
  console.log(`Total Rooms Joined: ${totalStats.rooms}`);
  console.log(`Total Subscriptions: ${totalStats.subscriptions}`);

  if (totalStats.errors === 0) {
    console.log('\n🎉 All tests passed successfully!');
  } else {
    console.log(`\n⚠️  Tests completed with ${totalStats.errors} errors`);
  }
}

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runWebSocketTests().catch(console.error);
}

export { runWebSocketTests, WebSocketTestClient };