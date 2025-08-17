import express from 'express';
import { Server } from 'http';
import { getTestDatabase } from './database.js';

/**
 * Create a test Express app with minimal setup
 */
export function createTestApp(): express.Application {
  const app = express();
  
  // Basic middleware
  app.use(express.json());
  app.use(express.urlencoded({ extended: true }));
  
  // Test route for health checks
  app.get('/health', (req, res) => {
    res.json({ status: 'ok', timestamp: new Date().toISOString() });
  });
  
  // Test route for database connectivity
  app.get('/db-health', async (req, res) => {
    try {
      const pool = await getTestDatabase();
      const result = await pool.query('SELECT NOW() as timestamp');
      res.json({ 
        status: 'ok', 
        database: 'connected',
        timestamp: result.rows[0].timestamp 
      });
    } catch (error) {
      res.status(500).json({ 
        status: 'error', 
        database: 'disconnected',
        error: error instanceof Error ? error.message : 'Unknown error'
      });
    }
  });
  
  return app;
}

/**
 * Start a test server on a random available port
 */
export async function startTestServer(app?: express.Application): Promise<{
  server: Server;
  port: number;
  url: string;
}> {
  const testApp = app || createTestApp();
  
  return new Promise((resolve, reject) => {
    const server = testApp.listen(0, 'localhost', () => {
      const address = server.address();
      if (!address || typeof address === 'string') {
        reject(new Error('Failed to get server address'));
        return;
      }
      
      const port = address.port;
      const url = `http://localhost:${port}`;
      
      resolve({ server, port, url });
    });
    
    server.on('error', reject);
  });
}

/**
 * Stop a test server
 */
export async function stopTestServer(server: Server): Promise<void> {
  return new Promise((resolve, reject) => {
    server.close((error) => {
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    });
  });
}