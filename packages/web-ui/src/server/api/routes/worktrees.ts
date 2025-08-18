import { Router } from 'express';
import { type Pool } from 'pg';
import {
  WorktreeCreateSchema,
  WorktreeUpdateSchema,
  WorktreeQuerySchema,
  WorktreeStatsQuerySchema,
  type WorktreeBase,
  type WorktreeCreate,
  type WorktreeUpdate,
  type WorktreeQuery,
  type WorktreeStats,
} from '../schemas/worktree.js';
import {
  validateBody,
  validateQuery,
  validateParams,
  validateContentType,
} from '../middleware/validation.js';
import {
  authenticateRequest,
  requirePermissions,
  type AuthenticatedRequest,
} from '../middleware/auth.js';
import {
  standardRateLimit,
  readOnlyRateLimit,
  strictRateLimit,
} from '../middleware/rate-limit.js';
import {
  sendSuccess,
  sendCreated,
  sendNoContent,
  sendNotFound,
  sendBadRequest,
  sendPaginatedSuccess,
  asyncHandler,
} from '../utils/responses.js';
import {
  ApiQueryBuilder,
  buildPaginatedQuery,
  applyFilters,
  recordExists,
  withTransaction,
  queryWithMetrics,
} from '../utils/database.js';
import { IdSchema } from '../schemas/common.js';

export function createWorktreeRoutes(pool: Pool): Router {
  const router = Router();

  // Apply common middleware
  router.use(authenticateRequest());
  router.use(validateContentType());

  // GET /api/worktrees - List worktrees with pagination and filtering
  router.get('/',
    readOnlyRateLimit,
    validateQuery(WorktreeQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as WorktreeQuery;
      
      const baseQuery = new ApiQueryBuilder('worktree_status')
        .select([
          'ws.*',
          'mr.name as repo_name',
          'mw.name as worktree_name_ref',
          'mw.abs_path as worktree_abs_path'
        ])
        .leftJoin('maproom_repos mr', 'mr.id = ws.repo_id')
        .leftJoin('maproom_worktrees mw', 'mw.id = ws.worktree_id');

      // Apply filters
      applyFilters(baseQuery, query);

      // Build paginated response
      const result = await buildPaginatedQuery<WorktreeBase>(pool, baseQuery, query);
      
      sendPaginatedSuccess(res, result, 'Worktrees retrieved successfully');
    })
  );

  // GET /api/worktrees/stats - Get worktree statistics
  router.get('/stats',
    readOnlyRateLimit,
    validateQuery(WorktreeStatsQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as any;
      
      const statsQuery = `
        SELECT 
          COUNT(*) as total_worktrees,
          COUNT(*) FILTER (WHERE state = 'active') as active_count,
          COUNT(*) FILTER (WHERE state = 'stale') as stale_count,
          COUNT(*) FILTER (WHERE state = 'archived') as archived_count,
          COUNT(*) FILTER (WHERE state = 'error') as error_count,
          SUM(disk_usage_bytes) as total_disk_usage,
          AVG(total_files) as avg_files_per_worktree,
          COUNT(DISTINCT repo_id) as unique_repos
        FROM worktree_status
        WHERE ($1::int IS NULL OR repo_id = $1)
        AND ($2::text IS NULL OR state = $2)
        AND ($3::timestamptz IS NULL OR created_at >= $3)
        AND ($4::timestamptz IS NULL OR created_at <= $4)
      `;

      const { result: statsResult } = await queryWithMetrics(
        pool,
        statsQuery,
        [
          query.repo_id || null,
          query.state || null,
          query.date_range?.from || null,
          query.date_range?.to || null,
        ]
      );

      const stats = statsResult[0];

      // Get most active worktrees
      const activeWorkstreesQuery = `
        SELECT 
          id,
          worktree_name as name,
          1 as access_count,
          last_accessed_at
        FROM worktree_status
        WHERE last_accessed_at IS NOT NULL
        ORDER BY last_accessed_at DESC
        LIMIT 10
      `;

      const { result: activeWorktrees } = await queryWithMetrics(pool, activeWorkstreesQuery);

      // Get recent activity (simplified)
      const recentActivityQuery = `
        SELECT 
          id as worktree_id,
          'status_update' as event_type,
          updated_at as timestamp,
          jsonb_build_object('state', state, 'files_changed', modified_files + added_files + deleted_files) as details
        FROM worktree_status
        WHERE updated_at >= NOW() - INTERVAL '7 days'
        ORDER BY updated_at DESC
        LIMIT 20
      `;

      const { result: recentActivity } = await queryWithMetrics(pool, recentActivityQuery);

      const response: WorktreeStats = {
        total_worktrees: parseInt(stats.total_worktrees),
        by_state: {
          active: parseInt(stats.active_count),
          stale: parseInt(stats.stale_count),
          archived: parseInt(stats.archived_count),
          error: parseInt(stats.error_count),
          merging: 0, // Add query for merging state if needed
        },
        by_repo: {}, // Would need additional query to group by repo name
        total_disk_usage: parseInt(stats.total_disk_usage || 0),
        avg_files_per_worktree: parseFloat(stats.avg_files_per_worktree || 0),
        most_active_worktrees: activeWorktrees,
        recent_activity: recentActivity,
      };

      sendSuccess(res, response, 'Worktree statistics retrieved successfully');
    })
  );

  // GET /api/worktrees/:id - Get single worktree
  router.get('/:id',
    readOnlyRateLimit,
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      const query = new ApiQueryBuilder('worktree_status')
        .select([
          'ws.*',
          'mr.name as repo_name',
          'mw.name as worktree_name_ref',
          'mw.abs_path as worktree_abs_path'
        ])
        .leftJoin('maproom_repos mr', 'mr.id = ws.repo_id')
        .leftJoin('maproom_worktrees mw', 'mw.id = ws.worktree_id')
        .where('ws.id = ?', id);

      const worktrees = await query.execute(pool);

      if (worktrees.length === 0) {
        return sendNotFound(res, 'Worktree', id, req.headers['x-request-id'] as string);
      }

      // Update last accessed timestamp
      await pool.query(
        'UPDATE worktree_status SET last_accessed_at = NOW() WHERE id = $1',
        [id]
      );

      sendSuccess(res, worktrees[0], 'Worktree retrieved successfully');
    })
  );

  // POST /api/worktrees - Create new worktree
  router.post('/',
    standardRateLimit,
    requirePermissions(['worktree_create']),
    validateBody(WorktreeCreateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const worktreeData = req.body as WorktreeCreate;

      // Check if worktree already exists
      const exists = await recordExists(pool, 'worktree_status', {
        worktree_id: worktreeData.worktree_id,
      });

      if (exists) {
        return sendBadRequest(
          res,
          'Worktree already exists with this worktree_id',
          { worktree_id: worktreeData.worktree_id },
          req.headers['x-request-id'] as string
        );
      }

      const result = await withTransaction(pool, async (client) => {
        // Insert new worktree status
        const insertQuery = `
          INSERT INTO worktree_status (
            worktree_id,
            repo_id,
            worktree_name,
            worktree_path,
            current_branch,
            upstream_branch,
            state,
            is_clean,
            is_synced,
            head_commit_sha,
            commits_ahead,
            commits_behind,
            modified_files,
            added_files,
            deleted_files,
            untracked_files,
            staged_files,
            total_files,
            total_size_bytes,
            disk_usage_bytes,
            chunk_count,
            cache_version,
            error_count,
            pinned,
            tags,
            notes,
            created_at,
            updated_at
          ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27, $28
          ) RETURNING *
        `;

        const values = [
          worktreeData.worktree_id,
          worktreeData.repo_id,
          worktreeData.worktree_name,
          worktreeData.worktree_path,
          worktreeData.current_branch,
          worktreeData.upstream_branch || null,
          worktreeData.state,
          false, // is_clean - will be updated by git status check
          false, // is_synced - will be updated by git status check
          '0000000000000000000000000000000000000000', // placeholder commit sha
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // git status counters
          0, 1, 0, // chunk_count, cache_version, error_count
          worktreeData.pinned || false,
          JSON.stringify(worktreeData.tags || []),
          worktreeData.notes || null,
          new Date().toISOString(),
          new Date().toISOString(),
        ];

        const result = await client.query(insertQuery, values);
        return result.rows[0];
      });

      sendCreated(res, result, 'Worktree created successfully');
    })
  );

  // PUT /api/worktrees/:id - Update worktree
  router.put('/:id',
    standardRateLimit,
    requirePermissions(['worktree_update']),
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    validateBody(WorktreeUpdateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const updateData = req.body as WorktreeUpdate;

      // Check if worktree exists
      const exists = await recordExists(pool, 'worktree_status', { id });
      if (!exists) {
        return sendNotFound(res, 'Worktree', id, req.headers['x-request-id'] as string);
      }

      // Build update query dynamically
      const updateFields: string[] = [];
      const updateValues: any[] = [];
      let paramIndex = 1;

      for (const [key, value] of Object.entries(updateData)) {
        if (value !== undefined) {
          updateFields.push(`${key} = $${paramIndex++}`);
          if (key === 'tags' && Array.isArray(value)) {
            updateValues.push(JSON.stringify(value));
          } else {
            updateValues.push(value);
          }
        }
      }

      if (updateFields.length === 0) {
        return sendBadRequest(
          res,
          'No valid fields provided for update',
          undefined,
          req.headers['x-request-id'] as string
        );
      }

      // Add updated_at field
      updateFields.push(`updated_at = $${paramIndex++}`);
      updateValues.push(new Date().toISOString());
      
      // Add ID for WHERE clause
      updateValues.push(id);

      const updateQuery = `
        UPDATE worktree_status 
        SET ${updateFields.join(', ')}
        WHERE id = $${paramIndex}
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, updateQuery, updateValues);

      if (result.length === 0) {
        return sendNotFound(res, 'Worktree', id, req.headers['x-request-id'] as string);
      }

      sendSuccess(res, result[0], 'Worktree updated successfully');
    })
  );

  // DELETE /api/worktrees/:id - Delete worktree
  router.delete('/:id',
    strictRateLimit,
    requirePermissions(['worktree_delete']),
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      // Check if worktree exists
      const exists = await recordExists(pool, 'worktree_status', { id });
      if (!exists) {
        return sendNotFound(res, 'Worktree', id, req.headers['x-request-id'] as string);
      }

      // Check for active agents before deletion
      const activeAgentsQuery = `
        SELECT COUNT(*) as count 
        FROM agent_runs 
        WHERE worktree_id = (SELECT worktree_id FROM worktree_status WHERE id = $1)
        AND status IN ('pending', 'running')
      `;

      const { result: activeAgentsResult } = await queryWithMetrics(pool, activeAgentsQuery, [id]);
      const activeAgentCount = parseInt(activeAgentsResult[0].count);

      if (activeAgentCount > 0) {
        return sendBadRequest(
          res,
          'Cannot delete worktree with active agents',
          { activeAgents: activeAgentCount },
          req.headers['x-request-id'] as string
        );
      }

      // Soft delete by marking as archived
      const archiveQuery = `
        UPDATE worktree_status 
        SET state = 'archived', updated_at = NOW()
        WHERE id = $1
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, archiveQuery, [id]);

      if (result.length === 0) {
        return sendNotFound(res, 'Worktree', id, req.headers['x-request-id'] as string);
      }

      sendNoContent(res);
    })
  );

  // POST /api/worktrees/:id/refresh - Refresh worktree status
  router.post('/:id/refresh',
    standardRateLimit,
    requirePermissions(['worktree_update']),
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      // Check if worktree exists
      const exists = await recordExists(pool, 'worktree_status', { id });
      if (!exists) {
        return sendNotFound(res, 'Worktree', id, req.headers['x-request-id'] as string);
      }

      // In a real implementation, this would trigger a git status check
      // For now, just update the last_scan_at timestamp
      const refreshQuery = `
        UPDATE worktree_status 
        SET 
          last_scan_at = NOW(),
          scan_duration_ms = $2,
          cache_version = cache_version + 1,
          updated_at = NOW()
        WHERE id = $1
        RETURNING *
      `;

      const scanDuration = Math.floor(Math.random() * 1000) + 100; // Simulated scan time
      const { result } = await queryWithMetrics(pool, refreshQuery, [id, scanDuration]);

      sendSuccess(res, result[0], 'Worktree status refreshed successfully');
    })
  );

  // POST /api/worktrees/:id/pin - Pin/unpin worktree
  router.post('/:id/pin',
    standardRateLimit,
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const { pinned } = req.body as { pinned: boolean };

      const pinQuery = `
        UPDATE worktree_status 
        SET pinned = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, pinQuery, [id, pinned]);

      if (result.length === 0) {
        return sendNotFound(res, 'Worktree', id, req.headers['x-request-id'] as string);
      }

      const action = pinned ? 'pinned' : 'unpinned';
      sendSuccess(res, result[0], `Worktree ${action} successfully`);
    })
  );

  return router;
}