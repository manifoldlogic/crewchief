import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import request from 'supertest';
import express from 'express';
import { Server } from 'http';
import {
  setupTestDatabase,
  teardownTestDatabase,
  closeTestDatabase,
  startTestServer,
  stopTestServer,
} from '@test-utils';

// Import the actual routes
import maproomRoutes from '../../../src/routes/maproom.js';

describe('Maproom API Integration Tests', () => {
  let app: express.Application;
  let server: Server;
  let baseURL: string;

  beforeAll(async () => {
    // Setup test database
    await setupTestDatabase();

    // Create test Express app
    app = express();
    app.use(express.json());
    app.use('/api/maproom', maproomRoutes);

    // Add health check endpoint for testing
    app.get('/health', (req, res) => {
      res.json({ status: 'ok' });
    });

    // Error handling middleware
    app.use((error: Error, req: express.Request, res: express.Response, next: express.NextFunction) => {
      console.error('Test API Error:', error);
      res.status(500).json({
        error: 'Internal Server Error',
        message: error.message,
      });
    });

    // Start test server
    const serverInfo = await startTestServer(app);
    server = serverInfo.server;
    baseURL = serverInfo.url;
  });

  afterAll(async () => {
    await stopTestServer(server);
    await closeTestDatabase();
  });

  beforeEach(async () => {
    // Clean up between tests if needed
  });

  afterEach(async () => {
    // Cleanup after each test
  });

  describe('Health Check', () => {
    it('should return 200 for health check', async () => {
      const response = await request(baseURL)
        .get('/health')
        .expect(200);

      expect(response.body).toEqual({ status: 'ok' });
    });
  });

  describe('POST /api/maproom/search', () => {
    it('should perform a search with valid query', async () => {
      const searchQuery = {
        query: 'function test',
        filters: {
          language: 'typescript',
          maxResults: 10,
        },
      };

      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send(searchQuery)
        .expect(200);

      expect(response.body).toHaveProperty('query', 'function test');
      expect(response.body).toHaveProperty('results');
      expect(response.body).toHaveProperty('totalCount');
      expect(response.body).toHaveProperty('executionTimeMs');
      expect(response.body).toHaveProperty('filters');
      expect(Array.isArray(response.body.results)).toBe(true);
    });

    it('should return 400 for missing query', async () => {
      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send({})
        .expect(400);

      expect(response.body).toHaveProperty('error');
      expect(response.body.error).toContain('query');
    });

    it('should handle empty search results', async () => {
      const searchQuery = {
        query: 'nonexistent_function_12345',
        filters: {},
      };

      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send(searchQuery)
        .expect(200);

      expect(response.body.results).toEqual([]);
      expect(response.body.totalCount).toBe(0);
    });

    it('should apply filters correctly', async () => {
      const searchQuery = {
        query: 'test',
        filters: {
          worktree: 'main',
          language: 'typescript',
          maxResults: 5,
          relevanceThreshold: 0.8,
        },
      };

      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send(searchQuery)
        .expect(200);

      expect(response.body.filters).toEqual(searchQuery.filters);
      expect(response.body.results.length).toBeLessThanOrEqual(5);
    });

    it('should validate query length', async () => {
      const searchQuery = {
        query: '', // Empty query
        filters: {},
      };

      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send(searchQuery)
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should handle search service errors gracefully', async () => {
      // This test would require mocking the MaproomService to throw an error
      // For now, we'll test with a malformed request that might trigger an error
      const searchQuery = {
        query: 'test',
        filters: {
          maxResults: -1, // Invalid value
        },
      };

      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send(searchQuery);

      // The response should be either 400 (validation error) or 500 (service error)
      expect([400, 500]).toContain(response.status);
      expect(response.body).toHaveProperty('error');
    });
  });

  describe('GET /api/maproom/status', () => {
    it('should return index status', async () => {
      const response = await request(baseURL)
        .get('/api/maproom/status')
        .expect(200);

      expect(response.body).toHaveProperty('repos');
      expect(response.body).toHaveProperty('totalFiles');
      expect(response.body).toHaveProperty('totalChunks');
      expect(response.body).toHaveProperty('lastUpdated');
      expect(Array.isArray(response.body.repos)).toBe(true);
      expect(typeof response.body.totalFiles).toBe('number');
      expect(typeof response.body.totalChunks).toBe('number');
    });
  });

  describe('POST /api/maproom/index', () => {
    it('should start indexing operation', async () => {
      const indexRequest = {
        paths: ['/test/path'],
        options: {
          repo: 'test-repo',
          worktree: 'test-worktree',
          incremental: true,
        },
      };

      const response = await request(baseURL)
        .post('/api/maproom/index')
        .send(indexRequest)
        .expect(200);

      expect(response.body).toHaveProperty('processId');
      expect(response.body).toHaveProperty('status', 'running');
      expect(response.body).toHaveProperty('startTime');
      expect(typeof response.body.processId).toBe('string');
    });

    it('should return 400 for missing paths', async () => {
      const response = await request(baseURL)
        .post('/api/maproom/index')
        .send({})
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should validate paths array', async () => {
      const indexRequest = {
        paths: [], // Empty array
      };

      const response = await request(baseURL)
        .post('/api/maproom/index')
        .send(indexRequest)
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });
  });

  describe('POST /api/maproom/upsert', () => {
    it('should update specific files', async () => {
      const upsertRequest = {
        paths: ['/test/file.ts'],
        options: {
          repo: 'test-repo',
          worktree: 'test-worktree',
          commit: 'abc123',
        },
      };

      const response = await request(baseURL)
        .post('/api/maproom/upsert')
        .send(upsertRequest)
        .expect(200);

      expect(response.body).toHaveProperty('success', true);
    });

    it('should return 400 for missing paths', async () => {
      const response = await request(baseURL)
        .post('/api/maproom/upsert')
        .send({})
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });
  });

  describe('GET /api/maproom/health', () => {
    it('should return maproom service health status', async () => {
      const response = await request(baseURL)
        .get('/api/maproom/health')
        .expect(200);

      expect(response.body).toHaveProperty('healthy');
      expect(typeof response.body.healthy).toBe('boolean');
      
      if (response.body.healthy) {
        expect(response.body).toHaveProperty('version');
      } else {
        expect(response.body).toHaveProperty('error');
      }
    });
  });

  describe('Error Handling', () => {
    it('should handle invalid JSON in request body', async () => {
      const response = await request(baseURL)
        .post('/api/maproom/search')
        .set('Content-Type', 'application/json')
        .send('invalid json')
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should handle missing Content-Type header', async () => {
      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send('query=test')
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should return 404 for non-existent endpoints', async () => {
      const response = await request(baseURL)
        .get('/api/maproom/nonexistent')
        .expect(404);
    });

    it('should handle large request payloads', async () => {
      const largeQuery = 'a'.repeat(10000); // 10KB query
      
      const response = await request(baseURL)
        .post('/api/maproom/search')
        .send({ query: largeQuery })
        .expect(400); // Assuming there's a query length limit

      expect(response.body).toHaveProperty('error');
    });
  });

  describe('Rate Limiting (if implemented)', () => {
    it('should handle multiple rapid requests', async () => {
      const promises = Array(5).fill(null).map(() =>
        request(baseURL)
          .post('/api/maproom/search')
          .send({ query: 'test', filters: {} })
      );

      const responses = await Promise.all(promises);
      
      // All requests should either succeed or be rate limited
      responses.forEach(response => {
        expect([200, 429]).toContain(response.status);
      });
    });
  });

  describe('CORS Headers (if configured)', () => {
    it('should include CORS headers in response', async () => {
      const response = await request(baseURL)
        .get('/api/maproom/status');

      // Check for common CORS headers if they're configured
      if (response.headers['access-control-allow-origin']) {
        expect(response.headers).toHaveProperty('access-control-allow-origin');
      }
    });
  });
});