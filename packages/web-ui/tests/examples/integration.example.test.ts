/**
 * Example Integration Test
 * 
 * Demonstrates best practices for integration testing in the CrewChief Web UI project.
 * Integration tests verify that multiple components work together correctly.
 */

import { describe, it, expect, beforeEach, afterEach, beforeAll, afterAll } from 'vitest';
import request from 'supertest';
import express from 'express';
import { Pool } from 'pg';

// Mock database setup for testing
const createTestApp = () => {
  const app = express();
  app.use(express.json());

  // Mock user storage (in real tests, this would be a test database)
  const users: Array<{ id: string; name: string; email: string }> = [];
  let nextId = 1;

  // User management endpoints
  app.post('/api/users', (req, res) => {
    const { name, email } = req.body;
    
    if (!name || !email) {
      return res.status(400).json({ error: 'Name and email are required' });
    }

    if (users.find(u => u.email === email)) {
      return res.status(409).json({ error: 'User with this email already exists' });
    }

    const user = { id: String(nextId++), name, email };
    users.push(user);
    res.status(201).json(user);
  });

  app.get('/api/users', (req, res) => {
    res.json(users);
  });

  app.get('/api/users/:id', (req, res) => {
    const user = users.find(u => u.id === req.params.id);
    if (!user) {
      return res.status(404).json({ error: 'User not found' });
    }
    res.json(user);
  });

  app.put('/api/users/:id', (req, res) => {
    const { name, email } = req.body;
    const userIndex = users.findIndex(u => u.id === req.params.id);
    
    if (userIndex === -1) {
      return res.status(404).json({ error: 'User not found' });
    }

    // Check for email conflicts with other users
    const existingUser = users.find(u => u.email === email && u.id !== req.params.id);
    if (existingUser) {
      return res.status(409).json({ error: 'Email already in use' });
    }

    users[userIndex] = { ...users[userIndex], name, email };
    res.json(users[userIndex]);
  });

  app.delete('/api/users/:id', (req, res) => {
    const userIndex = users.findIndex(u => u.id === req.params.id);
    if (userIndex === -1) {
      return res.status(404).json({ error: 'User not found' });
    }

    users.splice(userIndex, 1);
    res.status(204).send();
  });

  // Search endpoint
  app.get('/api/search/users', (req, res) => {
    const { q } = req.query;
    if (!q || typeof q !== 'string') {
      return res.status(400).json({ error: 'Query parameter "q" is required' });
    }

    const results = users.filter(user => 
      user.name.toLowerCase().includes(q.toLowerCase()) ||
      user.email.toLowerCase().includes(q.toLowerCase())
    );

    res.json({ results, total: results.length });
  });

  return app;
};

describe('Example Integration Tests', () => {
  let app: express.Application;

  beforeEach(() => {
    app = createTestApp();
  });

  describe('User Management API', () => {
    describe('POST /api/users', () => {
      it('creates a new user successfully', async () => {
        const userData = {
          name: 'John Doe',
          email: 'john@example.com'
        };

        const response = await request(app)
          .post('/api/users')
          .send(userData)
          .expect(201);

        expect(response.body).toMatchObject({
          id: expect.any(String),
          name: 'John Doe',
          email: 'john@example.com'
        });
      });

      it('returns validation error for missing fields', async () => {
        const response = await request(app)
          .post('/api/users')
          .send({ name: 'John Doe' }) // Missing email
          .expect(400);

        expect(response.body.error).toBe('Name and email are required');
      });

      it('prevents duplicate email addresses', async () => {
        const userData = {
          name: 'John Doe',
          email: 'john@example.com'
        };

        // Create first user
        await request(app)
          .post('/api/users')
          .send(userData)
          .expect(201);

        // Try to create duplicate
        const response = await request(app)
          .post('/api/users')
          .send({ name: 'Jane Doe', email: 'john@example.com' })
          .expect(409);

        expect(response.body.error).toBe('User with this email already exists');
      });
    });

    describe('GET /api/users', () => {
      it('returns empty array when no users exist', async () => {
        const response = await request(app)
          .get('/api/users')
          .expect(200);

        expect(response.body).toEqual([]);
      });

      it('returns all users', async () => {
        // Create some users first
        const users = [
          { name: 'John Doe', email: 'john@example.com' },
          { name: 'Jane Smith', email: 'jane@example.com' }
        ];

        for (const user of users) {
          await request(app).post('/api/users').send(user);
        }

        const response = await request(app)
          .get('/api/users')
          .expect(200);

        expect(response.body).toHaveLength(2);
        expect(response.body[0]).toMatchObject(users[0]);
        expect(response.body[1]).toMatchObject(users[1]);
      });
    });

    describe('GET /api/users/:id', () => {
      it('returns specific user by ID', async () => {
        const userData = { name: 'John Doe', email: 'john@example.com' };
        
        const createResponse = await request(app)
          .post('/api/users')
          .send(userData);

        const userId = createResponse.body.id;

        const response = await request(app)
          .get(`/api/users/${userId}`)
          .expect(200);

        expect(response.body).toMatchObject({
          id: userId,
          ...userData
        });
      });

      it('returns 404 for non-existent user', async () => {
        const response = await request(app)
          .get('/api/users/999')
          .expect(404);

        expect(response.body.error).toBe('User not found');
      });
    });

    describe('PUT /api/users/:id', () => {
      let userId: string;

      beforeEach(async () => {
        const response = await request(app)
          .post('/api/users')
          .send({ name: 'John Doe', email: 'john@example.com' });
        userId = response.body.id;
      });

      it('updates user successfully', async () => {
        const updatedData = {
          name: 'John Smith',
          email: 'johnsmith@example.com'
        };

        const response = await request(app)
          .put(`/api/users/${userId}`)
          .send(updatedData)
          .expect(200);

        expect(response.body).toMatchObject({
          id: userId,
          ...updatedData
        });
      });

      it('prevents email conflicts during update', async () => {
        // Create another user first
        await request(app)
          .post('/api/users')
          .send({ name: 'Jane Doe', email: 'jane@example.com' });

        // Try to update first user with second user's email
        const response = await request(app)
          .put(`/api/users/${userId}`)
          .send({ name: 'John Doe', email: 'jane@example.com' })
          .expect(409);

        expect(response.body.error).toBe('Email already in use');
      });

      it('returns 404 for non-existent user', async () => {
        const response = await request(app)
          .put('/api/users/999')
          .send({ name: 'Test', email: 'test@example.com' })
          .expect(404);

        expect(response.body.error).toBe('User not found');
      });
    });

    describe('DELETE /api/users/:id', () => {
      let userId: string;

      beforeEach(async () => {
        const response = await request(app)
          .post('/api/users')
          .send({ name: 'John Doe', email: 'john@example.com' });
        userId = response.body.id;
      });

      it('deletes user successfully', async () => {
        await request(app)
          .delete(`/api/users/${userId}`)
          .expect(204);

        // Verify user is deleted
        await request(app)
          .get(`/api/users/${userId}`)
          .expect(404);
      });

      it('returns 404 for non-existent user', async () => {
        const response = await request(app)
          .delete('/api/users/999')
          .expect(404);

        expect(response.body.error).toBe('User not found');
      });
    });
  });

  describe('Search Functionality', () => {
    beforeEach(async () => {
      // Create test users
      const users = [
        { name: 'John Doe', email: 'john.doe@example.com' },
        { name: 'Jane Smith', email: 'jane.smith@company.com' },
        { name: 'Bob Johnson', email: 'bob@example.org' },
        { name: 'Alice Brown', email: 'alice.brown@test.com' }
      ];

      for (const user of users) {
        await request(app).post('/api/users').send(user);
      }
    });

    describe('GET /api/search/users', () => {
      it('searches users by name', async () => {
        const response = await request(app)
          .get('/api/search/users')
          .query({ q: 'john' })
          .expect(200);

        expect(response.body.total).toBe(2); // John Doe and Bob Johnson
        expect(response.body.results).toHaveLength(2);
        
        const names = response.body.results.map((u: any) => u.name);
        expect(names).toContain('John Doe');
        expect(names).toContain('Bob Johnson');
      });

      it('searches users by email', async () => {
        const response = await request(app)
          .get('/api/search/users')
          .query({ q: 'example.com' })
          .expect(200);

        expect(response.body.total).toBe(1); // Only John Doe
        expect(response.body.results[0].name).toBe('John Doe');
      });

      it('performs case-insensitive search', async () => {
        const response = await request(app)
          .get('/api/search/users')
          .query({ q: 'ALICE' })
          .expect(200);

        expect(response.body.total).toBe(1);
        expect(response.body.results[0].name).toBe('Alice Brown');
      });

      it('returns empty results for no matches', async () => {
        const response = await request(app)
          .get('/api/search/users')
          .query({ q: 'nonexistent' })
          .expect(200);

        expect(response.body.total).toBe(0);
        expect(response.body.results).toHaveLength(0);
      });

      it('requires query parameter', async () => {
        const response = await request(app)
          .get('/api/search/users')
          .expect(400);

        expect(response.body.error).toBe('Query parameter "q" is required');
      });
    });
  });

  describe('End-to-End Workflows', () => {
    it('completes full user lifecycle', async () => {
      // 1. Create user
      const createResponse = await request(app)
        .post('/api/users')
        .send({ name: 'Test User', email: 'test@example.com' })
        .expect(201);

      const userId = createResponse.body.id;

      // 2. Verify user exists in list
      const listResponse = await request(app)
        .get('/api/users')
        .expect(200);

      expect(listResponse.body).toHaveLength(1);
      expect(listResponse.body[0].id).toBe(userId);

      // 3. Update user
      await request(app)
        .put(`/api/users/${userId}`)
        .send({ name: 'Updated User', email: 'updated@example.com' })
        .expect(200);

      // 4. Verify update
      const getResponse = await request(app)
        .get(`/api/users/${userId}`)
        .expect(200);

      expect(getResponse.body.name).toBe('Updated User');
      expect(getResponse.body.email).toBe('updated@example.com');

      // 5. Search for user
      const searchResponse = await request(app)
        .get('/api/search/users')
        .query({ q: 'updated' })
        .expect(200);

      expect(searchResponse.body.total).toBe(1);
      expect(searchResponse.body.results[0].id).toBe(userId);

      // 6. Delete user
      await request(app)
        .delete(`/api/users/${userId}`)
        .expect(204);

      // 7. Verify deletion
      await request(app)
        .get(`/api/users/${userId}`)
        .expect(404);

      const finalListResponse = await request(app)
        .get('/api/users')
        .expect(200);

      expect(finalListResponse.body).toHaveLength(0);
    });

    it('handles concurrent operations correctly', async () => {
      // Create multiple users concurrently
      const userPromises = Array.from({ length: 5 }, (_, i) => 
        request(app)
          .post('/api/users')
          .send({ 
            name: `User ${i}`, 
            email: `user${i}@example.com` 
          })
      );

      const responses = await Promise.all(userPromises);

      // All should succeed
      responses.forEach(response => {
        expect(response.status).toBe(201);
      });

      // Verify all users were created
      const listResponse = await request(app)
        .get('/api/users')
        .expect(200);

      expect(listResponse.body).toHaveLength(5);
    });
  });

  describe('Error Handling', () => {
    it('handles malformed JSON', async () => {
      const response = await request(app)
        .post('/api/users')
        .send('invalid json')
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('handles empty request body', async () => {
      const response = await request(app)
        .post('/api/users')
        .send({})
        .expect(400);

      expect(response.body.error).toBe('Name and email are required');
    });

    it('handles special characters in search', async () => {
      // Create user with special characters
      await request(app)
        .post('/api/users')
        .send({ name: 'John O\'Reilly', email: 'john@test.com' });

      const response = await request(app)
        .get('/api/search/users')
        .query({ q: 'O\'Reilly' })
        .expect(200);

      expect(response.body.total).toBe(1);
    });
  });
});