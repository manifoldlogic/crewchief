import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import {
  setupTestDatabase,
  teardownTestDatabase,
  closeTestDatabase,
  queryTestDatabase,
  getTestDatabaseClient,
} from '@test-utils';

describe('Database Integration Tests', () => {
  beforeAll(async () => {
    await setupTestDatabase();
  });

  afterAll(async () => {
    await closeTestDatabase();
  });

  beforeEach(async () => {
    // Clean up data before each test
    await teardownTestDatabase();
    await setupTestDatabase();
  });

  afterEach(async () => {
    // Clean up after each test
    await teardownTestDatabase();
  });

  describe('Database Connection', () => {
    it('should connect to the test database', async () => {
      const result = await queryTestDatabase('SELECT NOW() as timestamp');
      expect(result.rows).toHaveLength(1);
      expect(result.rows[0]).toHaveProperty('timestamp');
    });

    it('should execute basic queries', async () => {
      const result = await queryTestDatabase('SELECT 1 as number, $1 as text', ['test']);
      expect(result.rows[0]).toEqual({ number: 1, text: 'test' });
    });
  });

  describe('Web Sessions Table', () => {
    it('should insert and retrieve web sessions', async () => {
      const sessionData = {
        id: 'test-session-1',
        user_id: 'test-user-1',
        expires_at: new Date('2024-12-31T23:59:59Z'),
        metadata: { ip: '127.0.0.1', userAgent: 'test-browser' },
      };

      // Insert session
      await queryTestDatabase(`
        INSERT INTO web_sessions (id, user_id, expires_at, metadata)
        VALUES ($1, $2, $3, $4)
      `, [sessionData.id, sessionData.user_id, sessionData.expires_at, JSON.stringify(sessionData.metadata)]);

      // Retrieve session
      const result = await queryTestDatabase('SELECT * FROM web_sessions WHERE id = $1', [sessionData.id]);
      
      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].id).toBe(sessionData.id);
      expect(result.rows[0].user_id).toBe(sessionData.user_id);
      expect(new Date(result.rows[0].expires_at)).toEqual(sessionData.expires_at);
      expect(result.rows[0].metadata).toEqual(sessionData.metadata);
    });

    it('should handle session expiration', async () => {
      const expiredSession = {
        id: 'expired-session',
        user_id: 'test-user',
        expires_at: new Date('2020-01-01T00:00:00Z'), // Expired
        metadata: {},
      };

      await queryTestDatabase(`
        INSERT INTO web_sessions (id, user_id, expires_at, metadata)
        VALUES ($1, $2, $3, $4)
      `, [expiredSession.id, expiredSession.user_id, expiredSession.expires_at, JSON.stringify(expiredSession.metadata)]);

      // Query for non-expired sessions
      const result = await queryTestDatabase(`
        SELECT * FROM web_sessions 
        WHERE id = $1 AND expires_at > NOW()
      `, [expiredSession.id]);

      expect(result.rows).toHaveLength(0);
    });
  });

  describe('Search History Table', () => {
    it('should store search history with results', async () => {
      // First create a session
      const sessionId = 'search-session-1';
      await queryTestDatabase(`
        INSERT INTO web_sessions (id, user_id, expires_at, metadata)
        VALUES ($1, $2, $3, $4)
      `, [sessionId, 'test-user', new Date('2024-12-31'), JSON.stringify({})]);

      const searchData = {
        session_id: sessionId,
        user_id: 'test-user',
        query: 'function test',
        search_type: 'semantic',
        filters: { language: 'typescript' },
        result_count: 5,
        execution_time_ms: 150,
        top_results: [
          { id: 'result-1', file_path: '/test.ts', relevance_score: 0.95 },
          { id: 'result-2', file_path: '/test2.ts', relevance_score: 0.85 },
        ],
      };

      await queryTestDatabase(`
        INSERT INTO web_search_history (
          session_id, user_id, query, search_type, filters,
          result_count, execution_time_ms, top_results
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      `, [
        searchData.session_id,
        searchData.user_id,
        searchData.query,
        searchData.search_type,
        JSON.stringify(searchData.filters),
        searchData.result_count,
        searchData.execution_time_ms,
        JSON.stringify(searchData.top_results),
      ]);

      const result = await queryTestDatabase(`
        SELECT * FROM web_search_history WHERE query = $1
      `, [searchData.query]);

      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].query).toBe(searchData.query);
      expect(result.rows[0].filters).toEqual(searchData.filters);
      expect(result.rows[0].top_results).toEqual(searchData.top_results);
    });

    it('should handle search history without user_id', async () => {
      const sessionId = 'anonymous-session';
      await queryTestDatabase(`
        INSERT INTO web_sessions (id, user_id, expires_at, metadata)
        VALUES ($1, $2, $3, $4)
      `, [sessionId, null, new Date('2024-12-31'), JSON.stringify({})]);

      await queryTestDatabase(`
        INSERT INTO web_search_history (
          session_id, query, search_type, result_count
        ) VALUES ($1, $2, $3, $4)
      `, [sessionId, 'anonymous search', 'fulltext', 3]);

      const result = await queryTestDatabase(`
        SELECT * FROM web_search_history WHERE session_id = $1
      `, [sessionId]);

      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].user_id).toBeNull();
    });
  });

  describe('UI Preferences Table', () => {
    it('should store and update user preferences', async () => {
      const sessionId = 'prefs-session';
      await queryTestDatabase(`
        INSERT INTO web_sessions (id, user_id, expires_at, metadata)
        VALUES ($1, $2, $3, $4)
      `, [sessionId, 'prefs-user', new Date('2024-12-31'), JSON.stringify({})]);

      const preferences = {
        session_id: sessionId,
        user_id: 'prefs-user',
        theme: 'dark',
        sidebar_collapsed: false,
        auto_refresh_interval: 5000,
        notifications_enabled: true,
      };

      // Insert preferences
      await queryTestDatabase(`
        INSERT INTO web_ui_preferences (
          session_id, user_id, theme, sidebar_collapsed,
          auto_refresh_interval, notifications_enabled
        ) VALUES ($1, $2, $3, $4, $5, $6)
      `, Object.values(preferences));

      // Update preferences
      await queryTestDatabase(`
        UPDATE web_ui_preferences 
        SET theme = $1, sidebar_collapsed = $2
        WHERE session_id = $3
      `, ['light', true, sessionId]);

      const result = await queryTestDatabase(`
        SELECT * FROM web_ui_preferences WHERE session_id = $1
      `, [sessionId]);

      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].theme).toBe('light');
      expect(result.rows[0].sidebar_collapsed).toBe(true);
      expect(result.rows[0].auto_refresh_interval).toBe(5000);
    });
  });

  describe('Agent Runs Table', () => {
    it('should track agent execution lifecycle', async () => {
      const agentRun = {
        id: 'agent-run-1',
        agent_id: 'claude-code',
        task_description: 'Write unit tests',
        status: 'running',
        metadata: {
          worktree_path: '/path/to/worktree',
          tmux_session: 'test-session',
        },
      };

      // Insert initial agent run
      await queryTestDatabase(`
        INSERT INTO agent_runs (
          id, agent_id, task_description, status, metadata
        ) VALUES ($1, $2, $3, $4, $5)
      `, [
        agentRun.id,
        agentRun.agent_id,
        agentRun.task_description,
        agentRun.status,
        JSON.stringify(agentRun.metadata),
      ]);

      // Update to completed status
      await queryTestDatabase(`
        UPDATE agent_runs 
        SET status = $1, completed_at = NOW(), exit_code = $2
        WHERE id = $3
      `, ['completed', 0, agentRun.id]);

      const result = await queryTestDatabase(`
        SELECT * FROM agent_runs WHERE id = $1
      `, [agentRun.id]);

      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].status).toBe('completed');
      expect(result.rows[0].exit_code).toBe(0);
      expect(result.rows[0].completed_at).toBeTruthy();
    });

    it('should handle agent run failures', async () => {
      const agentRun = {
        id: 'failed-run',
        agent_id: 'test-agent',
        task_description: 'Failing task',
        status: 'failed',
        error_message: 'Test error',
        exit_code: 1,
      };

      await queryTestDatabase(`
        INSERT INTO agent_runs (
          id, agent_id, task_description, status, error_message, exit_code
        ) VALUES ($1, $2, $3, $4, $5, $6)
      `, Object.values(agentRun));

      const result = await queryTestDatabase(`
        SELECT * FROM agent_runs WHERE status = 'failed'
      `);

      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].error_message).toBe('Test error');
      expect(result.rows[0].exit_code).toBe(1);
    });
  });

  describe('Agent Messages Table', () => {
    it('should store agent communication messages', async () => {
      // First create an agent run
      const runId = 'msg-test-run';
      await queryTestDatabase(`
        INSERT INTO agent_runs (id, agent_id, task_description, status)
        VALUES ($1, $2, $3, $4)
      `, [runId, 'test-agent', 'Test task', 'running']);

      const message = {
        id: 'msg-1',
        run_id: runId,
        role: 'user',
        content: 'Please write tests',
        metadata: { priority: 'high' },
      };

      await queryTestDatabase(`
        INSERT INTO agent_messages (id, run_id, role, content, metadata)
        VALUES ($1, $2, $3, $4, $5)
      `, [
        message.id,
        message.run_id,
        message.role,
        message.content,
        JSON.stringify(message.metadata),
      ]);

      const result = await queryTestDatabase(`
        SELECT * FROM agent_messages WHERE run_id = $1
      `, [runId]);

      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].role).toBe('user');
      expect(result.rows[0].content).toBe('Please write tests');
    });
  });

  describe('Worktree Status Table', () => {
    it('should track worktree states', async () => {
      const worktree = {
        id: 'worktree-1',
        path: '/path/to/worktree',
        branch: 'feature/tests',
        status: 'active',
        agent_id: 'claude-code',
        git_status: {
          modified: ['file1.ts', 'file2.ts'],
          added: ['test.ts'],
          deleted: [],
        },
      };

      await queryTestDatabase(`
        INSERT INTO worktree_status (
          id, path, branch, status, agent_id, git_status
        ) VALUES ($1, $2, $3, $4, $5, $6)
      `, [
        worktree.id,
        worktree.path,
        worktree.branch,
        worktree.status,
        worktree.agent_id,
        JSON.stringify(worktree.git_status),
      ]);

      const result = await queryTestDatabase(`
        SELECT * FROM worktree_status WHERE id = $1
      `, [worktree.id]);

      expect(result.rows).toHaveLength(1);
      expect(result.rows[0].git_status).toEqual(worktree.git_status);
    });
  });

  describe('Transaction Handling', () => {
    it('should handle transaction rollback on error', async () => {
      const client = await getTestDatabaseClient();
      
      try {
        await client.query('BEGIN');
        
        // Insert valid data
        await client.query(`
          INSERT INTO web_sessions (id, user_id, expires_at, metadata)
          VALUES ($1, $2, $3, $4)
        `, ['tx-session', 'tx-user', new Date('2024-12-31'), JSON.stringify({})]);
        
        // This should fail due to foreign key constraint (invalid session_id)
        await client.query(`
          INSERT INTO web_search_history (session_id, query, search_type)
          VALUES ($1, $2, $3)
        `, ['nonexistent-session', 'test query', 'semantic']);
        
        await client.query('COMMIT');
      } catch (error) {
        await client.query('ROLLBACK');
        
        // Verify that no data was inserted
        const result = await client.query(`
          SELECT * FROM web_sessions WHERE id = 'tx-session'
        `);
        expect(result.rows).toHaveLength(0);
      } finally {
        client.release();
      }
    });
  });

  describe('Performance', () => {
    it('should handle bulk inserts efficiently', async () => {
      const sessionId = 'bulk-session';
      await queryTestDatabase(`
        INSERT INTO web_sessions (id, user_id, expires_at, metadata)
        VALUES ($1, $2, $3, $4)
      `, [sessionId, 'bulk-user', new Date('2024-12-31'), JSON.stringify({})]);

      const startTime = Date.now();
      
      // Insert 100 search history entries
      const promises = Array(100).fill(null).map((_, index) =>
        queryTestDatabase(`
          INSERT INTO web_search_history (session_id, query, search_type, result_count)
          VALUES ($1, $2, $3, $4)
        `, [sessionId, `query ${index}`, 'semantic', index % 10])
      );

      await Promise.all(promises);
      
      const executionTime = Date.now() - startTime;
      
      // Verify all records were inserted
      const result = await queryTestDatabase(`
        SELECT COUNT(*) as count FROM web_search_history WHERE session_id = $1
      `, [sessionId]);
      
      expect(result.rows[0].count).toBe('100');
      expect(executionTime).toBeLessThan(5000); // Should complete within 5 seconds
    });
  });
});