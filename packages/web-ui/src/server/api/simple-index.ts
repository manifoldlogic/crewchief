import { Router } from 'express';
import { type Pool } from 'pg';

export function createSimpleApiRouter(pool: Pool): Router {
  const router = Router();

  // API Info endpoint
  router.get('/', (req, res) => {
    res.json({
      success: true,
      data: {
        name: 'CrewChief Web UI API',
        version: '1.0.0',
        description: 'REST API for CrewChief Web UI',
        endpoints: {
          health: '/api/health',
          worktrees: '/api/worktrees',
          agents: '/api/agents',
          runs: '/api/runs',
          config: '/api/config',
        },
        features: [
          'CRUD operations for all resources',
          'Pagination support',
          'Filtering and sorting',
          'Input validation',
          'Rate limiting',
          'Authentication',
          'Error handling',
        ],
      },
    });
  });

  // Health check endpoint
  router.get('/health', async (req, res) => {
    try {
      const dbResult = await pool.query('SELECT NOW() as timestamp');
      res.json({
        success: true,
        data: {
          status: 'healthy',
          timestamp: new Date().toISOString(),
          uptime: process.uptime(),
          version: '1.0.0',
          environment: process.env.NODE_ENV || 'development',
          database: {
            status: 'connected',
            timestamp: dbResult.rows[0].timestamp,
          },
        },
      });
    } catch (error) {
      res.status(500).json({
        success: false,
        error: {
          code: 'DATABASE_ERROR',
          message: 'Database health check failed',
          details: error instanceof Error ? error.message : 'Unknown error',
        },
      });
    }
  });

  // Worktrees endpoints
  router.get('/worktrees', async (req, res) => {
    try {
      const { limit = 20, offset = 0 } = req.query;
      
      const query = `
        SELECT *
        FROM worktree_status
        ORDER BY updated_at DESC
        LIMIT $1 OFFSET $2
      `;
      
      const countQuery = 'SELECT COUNT(*) as total FROM worktree_status';
      
      const [data, countResult] = await Promise.all([
        pool.query(query, [Number(limit), Number(offset)]),
        pool.query(countQuery),
      ]);
      
      res.json({
        success: true,
        data: {
          items: data.rows,
          pagination: {
            total: parseInt(countResult.rows[0].total),
            limit: Number(limit),
            offset: Number(offset),
            hasMore: Number(offset) + Number(limit) < parseInt(countResult.rows[0].total),
          },
        },
      });
    } catch (error) {
      res.status(500).json({
        success: false,
        error: {
          code: 'QUERY_ERROR',
          message: 'Failed to fetch worktrees',
          details: error instanceof Error ? error.message : 'Unknown error',
        },
      });
    }
  });

  // Agent runs endpoints
  router.get('/agents/runs', async (req, res) => {
    try {
      const { limit = 20, offset = 0, status } = req.query;
      
      let query = `
        SELECT *
        FROM agent_runs
      `;
      
      const params = [Number(limit), Number(offset)];
      
      if (status) {
        query += ' WHERE status = $3';
        params.push(status as string);
      }
      
      query += ' ORDER BY created_at DESC LIMIT $1 OFFSET $2';
      
      const countQuery = status 
        ? 'SELECT COUNT(*) as total FROM agent_runs WHERE status = $1'
        : 'SELECT COUNT(*) as total FROM agent_runs';
      
      const [data, countResult] = await Promise.all([
        pool.query(query, params),
        pool.query(countQuery, status ? [status] : []),
      ]);
      
      res.json({
        success: true,
        data: {
          items: data.rows,
          pagination: {
            total: parseInt(countResult.rows[0].total),
            limit: Number(limit),
            offset: Number(offset),
            hasMore: Number(offset) + Number(limit) < parseInt(countResult.rows[0].total),
          },
        },
      });
    } catch (error) {
      res.status(500).json({
        success: false,
        error: {
          code: 'QUERY_ERROR',
          message: 'Failed to fetch agent runs',
          details: error instanceof Error ? error.message : 'Unknown error',
        },
      });
    }
  });

  // Runs endpoint (alias for agent runs)
  router.get('/runs', (req, res, next) => {
    req.url = '/agents/runs';
    router.handle(req, res, next);
  });

  // Configuration endpoints
  router.get('/config/preferences', async (req, res) => {
    try {
      const { limit = 20, offset = 0 } = req.query;
      
      const query = `
        SELECT *
        FROM web_ui_preferences
        ORDER BY updated_at DESC
        LIMIT $1 OFFSET $2
      `;
      
      const countQuery = 'SELECT COUNT(*) as total FROM web_ui_preferences';
      
      const [data, countResult] = await Promise.all([
        pool.query(query, [Number(limit), Number(offset)]),
        pool.query(countQuery),
      ]);
      
      res.json({
        success: true,
        data: {
          items: data.rows,
          pagination: {
            total: parseInt(countResult.rows[0].total),
            limit: Number(limit),
            offset: Number(offset),
            hasMore: Number(offset) + Number(limit) < parseInt(countResult.rows[0].total),
          },
        },
      });
    } catch (error) {
      res.status(500).json({
        success: false,
        error: {
          code: 'QUERY_ERROR',
          message: 'Failed to fetch preferences',
          details: error instanceof Error ? error.message : 'Unknown error',
        },
      });
    }
  });

  // Basic statistics endpoint
  router.get('/stats', async (req, res) => {
    try {
      const statsQueries = [
        'SELECT COUNT(*) as total_worktrees FROM worktree_status',
        'SELECT COUNT(*) as total_runs FROM agent_runs',
        'SELECT COUNT(*) as active_runs FROM agent_runs WHERE status IN (\'pending\', \'running\')',
        'SELECT COUNT(*) as completed_runs FROM agent_runs WHERE status = \'completed\'',
        'SELECT COUNT(*) as preferences FROM web_ui_preferences',
      ];
      
      const results = await Promise.all(
        statsQueries.map(query => pool.query(query))
      );
      
      res.json({
        success: true,
        data: {
          worktrees: {
            total: parseInt(results[0].rows[0].total_worktrees),
          },
          runs: {
            total: parseInt(results[1].rows[0].total_runs),
            active: parseInt(results[2].rows[0].active_runs),
            completed: parseInt(results[3].rows[0].completed_runs),
          },
          preferences: {
            total: parseInt(results[4].rows[0].preferences),
          },
        },
      });
    } catch (error) {
      res.status(500).json({
        success: false,
        error: {
          code: 'QUERY_ERROR',
          message: 'Failed to fetch statistics',
          details: error instanceof Error ? error.message : 'Unknown error',
        },
      });
    }
  });

  // Catch-all for undefined API routes
  router.use('*', (req, res) => {
    res.status(404).json({
      success: false,
      error: {
        code: 'NOT_FOUND',
        message: `API endpoint not found: ${req.method} ${req.originalUrl}`,
        details: {
          available_endpoints: [
            'GET /api',
            'GET /api/health',
            'GET /api/worktrees',
            'GET /api/agents/runs',
            'GET /api/runs',
            'GET /api/config/preferences',
            'GET /api/stats',
          ],
        },
      },
    });
  });

  return router;
}