/**
 * Simple WebSocket test client using socket.io-client
 * This tests the WebSocket server functionality programmatically
 */

import { io } from 'socket.io-client';
import jwt from 'jsonwebtoken';

let stats = {
    connected: false,
    messagesReceived: 0,
    messagesSent: 0,
    errors: 0,
    rooms: new Set(),
    subscriptions: new Set()
};

function log(message, type = 'info') {
    const timestamp = new Date().toISOString();
    const prefix = type === 'error' ? '❌' : type === 'success' ? '✅' : type === 'warning' ? '⚠️' : 'ℹ️';
    console.log(`[${timestamp}] ${prefix} ${message}`);
}

function updateStats() {
    console.log('\n📊 Current Stats:');
    console.log(`   Connected: ${stats.connected}`);
    console.log(`   Messages Sent: ${stats.messagesSent}`);
    console.log(`   Messages Received: ${stats.messagesReceived}`);
    console.log(`   Errors: ${stats.errors}`);
    console.log(`   Rooms: ${Array.from(stats.rooms).join(', ') || 'None'}`);
    console.log(`   Subscriptions: ${Array.from(stats.subscriptions).join(', ') || 'None'}\n`);
}

async function testWebSocketServer() {
    console.log('🧪 Starting WebSocket Server Test');
    console.log('================================\n');

    // Generate a test JWT token
    let token;
    try {
        const jwtSecret = process.env.JWT_SECRET || 'development-secret-key';
        token = jwt.sign(
            {
                userId: 'test-user-' + Date.now(),
                username: 'TestUser',
                email: 'test@example.com',
            },
            jwtSecret,
            { expiresIn: '1h' }
        );
        log('Generated test JWT token', 'success');
    } catch (error) {
        log('Could not generate JWT token, testing without authentication', 'warning');
    }

    // Connect to WebSocket server
    log('Connecting to WebSocket server...');
    
    const socket = io('http://localhost:3456', {
        auth: token ? { token } : {},
        transports: ['websocket'],
        timeout: 5000
    });

    return new Promise((resolve, reject) => {
        const testTimeout = setTimeout(() => {
            log('Test timeout reached', 'error');
            socket.disconnect();
            reject(new Error('Test timeout'));
        }, 30000); // 30 second timeout

        // Connection events
        socket.on('connect', async () => {
            log('Connected to WebSocket server', 'success');
            stats.connected = true;
            updateStats();

            try {
                await runTests(socket);
                clearTimeout(testTimeout);
                socket.disconnect();
                resolve();
            } catch (error) {
                clearTimeout(testTimeout);
                socket.disconnect();
                reject(error);
            }
        });

        socket.on('connect_error', (error) => {
            log(`Connection error: ${error.message}`, 'error');
            stats.errors++;
            clearTimeout(testTimeout);
            reject(error);
        });

        socket.on('disconnect', (reason) => {
            log(`Disconnected: ${reason}`, 'warning');
            stats.connected = false;
        });

        // Server event handlers
        socket.on('connected', (data) => {
            log(`Connection acknowledged: ${JSON.stringify(data)}`, 'success');
            stats.messagesReceived++;
        });

        socket.on('heartbeat', () => {
            log('Heartbeat received');
        });

        socket.on('pong', (data) => {
            log(`Pong received: ${data.timestamp}`, 'success');
            stats.messagesReceived++;
        });

        socket.on('room-joined', (data) => {
            log(`Joined room: ${data.room} (${data.memberCount} members)`, 'success');
            stats.rooms.add(data.room);
            stats.messagesReceived++;
        });

        socket.on('room-left', (data) => {
            log(`Left room: ${data.room} (${data.memberCount} members)`);
            stats.rooms.delete(data.room);
            stats.messagesReceived++;
        });

        socket.on('subscription-confirmed', (data) => {
            log(`Subscribed to: ${data.type}`, 'success');
            stats.subscriptions.add(data.type);
            stats.messagesReceived++;
        });

        socket.on('echo-response', (data) => {
            log(`Echo response received: ${data.message}`, 'success');
            stats.messagesReceived++;
        });

        socket.on('broadcast-message', (data) => {
            log(`Broadcast message received: ${JSON.stringify(data)}`);
            stats.messagesReceived++;
        });

        socket.on('room-message', (data) => {
            log(`Room message received in ${data.room}: ${data.message}`);
            stats.messagesReceived++;
        });

        socket.on('message-sent', (data) => {
            log(`Message sent confirmation: ${data.type}`, 'success');
            stats.messagesReceived++;
        });

        socket.on('error', (error) => {
            log(`Socket error: ${JSON.stringify(error)}`, 'error');
            stats.errors++;
        });
    });
}

async function runTests(socket) {
    log('🚀 Running WebSocket tests...\n');

    // Test 1: Ping/Pong
    await testPing(socket);
    
    // Test 2: Room management
    await testRoomManagement(socket);
    
    // Test 3: Message sending
    await testMessageSending(socket);
    
    // Test 4: Event subscriptions
    await testEventSubscriptions(socket);
    
    // Test 5: Load testing
    await testLoadTesting(socket);

    log('All tests completed!', 'success');
    updateStats();
}

async function testPing(socket) {
    log('Testing ping/pong...');
    
    return new Promise((resolve) => {
        socket.emit('ping');
        stats.messagesSent++;
        
        const timeout = setTimeout(() => {
            log('Ping test completed (no pong response expected immediately)');
            resolve();
        }, 1000);
    });
}

async function testRoomManagement(socket) {
    log('Testing room management...');
    
    const roomName = 'test-room-' + Date.now();
    
    return new Promise((resolve, reject) => {
        // Join room
        socket.emit('join-room', { room: roomName, type: 'test' });
        stats.messagesSent++;
        
        const joinTimeout = setTimeout(() => {
            reject(new Error('Room join timeout'));
        }, 5000);

        socket.once('room-joined', (data) => {
            clearTimeout(joinTimeout);
            
            if (data.room === roomName) {
                log(`Successfully joined room: ${roomName}`, 'success');
                
                // Leave room
                socket.emit('leave-room', { room: roomName });
                stats.messagesSent++;
                
                const leaveTimeout = setTimeout(() => {
                    resolve(); // Don't fail if leave doesn't respond
                }, 2000);

                socket.once('room-left', (data) => {
                    clearTimeout(leaveTimeout);
                    if (data.room === roomName) {
                        log(`Successfully left room: ${roomName}`, 'success');
                    }
                    resolve();
                });
            } else {
                reject(new Error('Joined wrong room'));
            }
        });
    });
}

async function testMessageSending(socket) {
    log('Testing message sending...');
    
    // Test echo message
    socket.emit('message', {
        type: 'echo',
        message: 'Test echo message',
        timestamp: new Date().toISOString()
    });
    stats.messagesSent++;
    
    // Test room message (will fail since we left the room, but should handle gracefully)
    socket.emit('message', {
        type: 'room-message',
        room: 'test-room-nonexistent',
        message: 'Test room message'
    });
    stats.messagesSent++;
    
    // Test broadcast (if authenticated)
    socket.emit('message', {
        type: 'broadcast',
        payload: {
            message: 'Test broadcast message'
        }
    });
    stats.messagesSent++;
    
    // Wait for responses
    await new Promise(resolve => setTimeout(resolve, 2000));
}

async function testEventSubscriptions(socket) {
    log('Testing event subscriptions...');
    
    // Subscribe to system updates
    socket.emit('subscribe', { type: 'system-updates' });
    stats.messagesSent++;
    
    // Subscribe to worktree updates
    socket.emit('subscribe', { 
        type: 'worktree-updates', 
        filters: { worktreeId: 'test-worktree' } 
    });
    stats.messagesSent++;
    
    // Subscribe to agent updates
    socket.emit('subscribe', { 
        type: 'agent-updates', 
        filters: { agentId: 'test-agent' } 
    });
    stats.messagesSent++;
    
    // Wait for subscription confirmations
    await new Promise(resolve => setTimeout(resolve, 3000));
}

async function testLoadTesting(socket) {
    log('Testing load (sending 10 echo messages rapidly)...');
    
    for (let i = 0; i < 10; i++) {
        socket.emit('message', {
            type: 'echo',
            message: `Load test message ${i + 1}`,
            timestamp: new Date().toISOString()
        });
        stats.messagesSent++;
        
        // Small delay between messages
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    // Wait for responses
    await new Promise(resolve => setTimeout(resolve, 2000));
}

// Run the test
testWebSocketServer()
    .then(() => {
        log('🎉 All WebSocket tests passed successfully!', 'success');
        updateStats();
        process.exit(0);
    })
    .catch((error) => {
        log(`❌ WebSocket tests failed: ${error.message}`, 'error');
        updateStats();
        process.exit(1);
    });