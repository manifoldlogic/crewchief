/**
 * Maproom API Routes
 * 
 * Express routes for Maproom operations: indexing, searching, status, and file updates.
 * Provides a REST API interface for the web UI to interact with Maproom.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { getMaproomService, SearchFilters } from '../services/maproom.js';
import { getDatabase } from '../db/connection.js';

export interface MaproomRequest extends Request {
  sessionId?: string;
  userId?: string;
}

// Error handling middleware for Maproom operations
function handleMaproomError(error: any, req: Request, res: Response, next: NextFunction): void {
  console.error('Maproom operation error:', error);

  // Handle specific Maproom error types
  if (error.code) {
    switch (error.code) {
      case 'TIMEOUT':
        res.status(408).json({
          error: 'Request timeout',
          message: 'The operation took too long to complete',
          code: error.code,
        });
        return;
      case 'COMMAND_FAILED':
        res.status(500).json({
          error: 'Command failed',
          message: error.message,
          code: error.code,
          exitCode: error.exitCode,
        });
        return;
      case 'INVALID_JSON':
        res.status(500).json({
          error: 'Invalid response',
          message: 'Maproom returned invalid JSON',
          code: error.code,
        });
        return;
      case 'SPAWN_ERROR':
        res.status(503).json({
          error: 'Service unavailable',
          message: 'Unable to start Maproom process',
          code: error.code,
        });
        return;
    }
  }

  // Generic error response
  res.status(500).json({
    error: 'Internal server error',
    message: error.message || 'An unexpected error occurred',
  });
}

// Session middleware to extract session info
async function extractSession(req: MaproomRequest, res: Response, next: NextFunction): Promise<void> {
  try {
    // Extract session ID from headers, cookies, or query params
    req.sessionId = req.headers['x-session-id'] as string || 
                   req.query.sessionId as string || 
                   'anonymous';
    
    // Extract user ID if available (for future multi-user support)
    req.userId = req.headers['x-user-id'] as string || undefined;
    
    next();
  } catch (error) {
    next(error);
  }
}

// Validation middleware for search requests
function validateSearchRequest(req: Request, res: Response, next: NextFunction): void {
  const { query } = req.body;
  
  if (!query || typeof query !== 'string' || query.trim().length === 0) {
    res.status(400).json({
      error: 'Invalid request',
      message: 'Query parameter is required and must be a non-empty string',
    });
    return;
  }

  if (query.length > 1000) {
    res.status(400).json({
      error: 'Invalid request',
      message: 'Query is too long (maximum 1000 characters)',
    });
    return;
  }

  next();
}

// Validation middleware for index requests
function validateIndexRequest(req: Request, res: Response, next: NextFunction): void {
  const { paths } = req.body;
  
  if (!paths || !Array.isArray(paths) || paths.length === 0) {
    res.status(400).json({
      error: 'Invalid request',
      message: 'Paths parameter is required and must be a non-empty array',
    });
    return;
  }

  // Validate each path
  for (const path of paths) {
    if (typeof path !== 'string' || path.trim().length === 0) {
      res.status(400).json({
        error: 'Invalid request',
        message: 'All paths must be non-empty strings',
      });
      return;
    }
  }

  next();
}

const router = Router();

// Apply session middleware to all routes
router.use(extractSession);

/**
 * POST /api/maproom/search
 * Perform a semantic or full-text search
 */
router.post('/search', validateSearchRequest, async (req: MaproomRequest, res: Response, next: NextFunction) => {
  try {
    const { query, filters = {} } = req.body;
    const maproom = getMaproomService();

    // Validate and normalize filters
    const searchFilters: SearchFilters = {
      worktree: filters.worktree,
      fileTypes: Array.isArray(filters.fileTypes) ? filters.fileTypes : undefined,
      language: filters.language,
      relevanceThreshold: typeof filters.relevanceThreshold === 'number' ? filters.relevanceThreshold : undefined,
      maxResults: typeof filters.maxResults === 'number' ? Math.min(filters.maxResults, 100) : 20,
      dateRange: filters.dateRange && typeof filters.dateRange === 'object' ? filters.dateRange : undefined,
    };

    // Perform the search
    const searchResponse = await maproom.search(query, searchFilters);

    // Store in search history
    if (req.sessionId) {
      await maproom.storeSearchHistory(
        req.sessionId,
        query,
        searchFilters,
        searchResponse.results,
        searchResponse.executionTimeMs,
        req.userId
      );
    }

    res.json({
      success: true,
      data: searchResponse,
    });
  } catch (error) {
    handleMaproomError(error, req, res, next);
  }
});

/**
 * GET /api/maproom/status
 * Get the current status of the Maproom index
 */
router.get('/status', async (req: Request, res: Response, next: NextFunction) => {
  try {
    const maproom = getMaproomService();
    const status = await maproom.getStatus();

    res.json({
      success: true,
      data: status,
    });
  } catch (error) {
    handleMaproomError(error, req, res, next);
  }
});

/**
 * POST /api/maproom/index
 * Start indexing files/directories
 */
router.post('/index', validateIndexRequest, async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { paths, options = {} } = req.body;
    const maproom = getMaproomService();

    // Validate options
    const indexOptions = {
      repo: typeof options.repo === 'string' ? options.repo : undefined,
      worktree: typeof options.worktree === 'string' ? options.worktree : undefined,
      incremental: Boolean(options.incremental),
    };

    const progress = await maproom.index(paths, indexOptions);

    res.json({
      success: true,
      data: progress,
    });
  } catch (error) {
    handleMaproomError(error, req, res, next);
  }
});

/**
 * POST /api/maproom/upsert
 * Update specific files in the index
 */
router.post('/upsert', validateIndexRequest, async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { paths, options = {} } = req.body;
    const maproom = getMaproomService();

    // Validate options
    const upsertOptions = {
      repo: typeof options.repo === 'string' ? options.repo : undefined,
      worktree: typeof options.worktree === 'string' ? options.worktree : undefined,
      commit: typeof options.commit === 'string' ? options.commit : undefined,
    };

    await maproom.upsert(paths, upsertOptions);

    res.json({
      success: true,
      message: 'Files updated successfully',
    });
  } catch (error) {
    handleMaproomError(error, req, res, next);
  }
});

/**
 * GET /api/maproom/index/:processId
 * Get the progress of a running index operation
 */
router.get('/index/:processId', (req: Request, res: Response) => {
  try {
    const { processId } = req.params;
    const maproom = getMaproomService();

    const progress = maproom.getIndexProgress(processId);
    
    if (!progress) {
      res.status(404).json({
        error: 'Not found',
        message: 'Index process not found or completed',
      });
      return;
    }

    res.json({
      success: true,
      data: progress,
    });
  } catch (error) {
    res.status(500).json({
      error: 'Internal server error',
      message: error.message,
    });
  }
});

/**
 * DELETE /api/maproom/index/:processId
 * Cancel a running index operation
 */
router.delete('/index/:processId', (req: Request, res: Response) => {
  try {
    const { processId } = req.params;
    const maproom = getMaproomService();

    const cancelled = maproom.cancelIndex(processId);
    
    if (!cancelled) {
      res.status(404).json({
        error: 'Not found',
        message: 'Index process not found or already completed',
      });
      return;
    }

    res.json({
      success: true,
      message: 'Index operation cancelled',
    });
  } catch (error) {
    res.status(500).json({
      error: 'Internal server error',
      message: error.message,
    });
  }
});

/**
 * GET /api/maproom/health
 * Health check for the Maproom service
 */
router.get('/health', async (req: Request, res: Response, next: NextFunction) => {
  try {
    const maproom = getMaproomService();
    const health = await maproom.healthCheck();

    if (health.healthy) {
      res.json({
        success: true,
        data: {
          status: 'healthy',
          version: health.version,
        },
      });
    } else {
      res.status(503).json({
        success: false,
        data: {
          status: 'unhealthy',
          error: health.error,
        },
      });
    }
  } catch (error) {
    handleMaproomError(error, req, res, next);
  }
});

/**
 * GET /api/maproom/cache/stats
 * Get cache statistics
 */
router.get('/cache/stats', (req: Request, res: Response) => {
  try {
    const maproom = getMaproomService();
    const stats = maproom.getCacheStats();

    res.json({
      success: true,
      data: stats,
    });
  } catch (error) {
    res.status(500).json({
      error: 'Internal server error',
      message: error.message,
    });
  }
});

/**
 * DELETE /api/maproom/cache
 * Clear the search cache
 */
router.delete('/cache', (req: Request, res: Response) => {
  try {
    const maproom = getMaproomService();
    maproom.clearCache();

    res.json({
      success: true,
      message: 'Cache cleared successfully',
    });
  } catch (error) {
    res.status(500).json({
      error: 'Internal server error',
      message: error.message,
    });
  }
});

/**
 * GET /api/maproom/search/history
 * Get search history for the current session
 */
router.get('/search/history', async (req: MaproomRequest, res: Response, next: NextFunction) => {
  try {
    const { limit = 50, offset = 0 } = req.query;
    const db = getDatabase();

    let query = `
      SELECT id, query, search_type, filters, result_count, 
             execution_time_ms, searched_at, saved
      FROM web_search_history 
      WHERE session_id = $1
    `;
    const params: any[] = [req.sessionId];

    // Add user filter if available
    if (req.userId) {
      query += ` AND (user_id = $2 OR user_id IS NULL)`;
      params.push(req.userId);
    }

    query += ` ORDER BY searched_at DESC LIMIT $${params.length + 1} OFFSET $${params.length + 2}`;
    params.push(Number(limit), Number(offset));

    const result = await db.query(query, params);

    res.json({
      success: true,
      data: {
        history: result.rows,
        total: result.rowCount || 0,
        limit: Number(limit),
        offset: Number(offset),
      },
    });
  } catch (error) {
    console.error('Failed to fetch search history:', error);
    res.status(500).json({
      error: 'Internal server error',
      message: 'Failed to fetch search history',
    });
  }
});

/**
 * GET /api/maproom/search/popular
 * Get popular searches
 */
router.get('/search/popular', async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { period = '7 days', limit = 10 } = req.query;
    const db = getDatabase();

    const result = await db.query(`
      SELECT * FROM get_popular_searches($1::INTERVAL, $2::INTEGER)
    `, [period, Number(limit)]);

    res.json({
      success: true,
      data: {
        popular: result.rows,
        period,
        limit: Number(limit),
      },
    });
  } catch (error) {
    console.error('Failed to fetch popular searches:', error);
    res.status(500).json({
      error: 'Internal server error',
      message: 'Failed to fetch popular searches',
    });
  }
});

export default router;