import { Router } from 'express';
import { type Pool } from 'pg';
import {
  AgentRunCreateSchema,
  AgentRunUpdateSchema,
  AgentRunQuerySchema,
  AgentStatsQuerySchema,
  type AgentRunBase,
  type AgentRunCreate,
  type AgentRunUpdate,
  type AgentRunQuery,
  type AgentStats,
} from '../schemas/agent.js';
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
  searchRateLimit,
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
import { IdSchema, UuidSchema } from '../schemas/common.js';
import { v4 as uuidv4 } from 'uuid';
import { z } from 'zod';

// Additional schemas specific to run management
const RunActionSchema = z.object({
  action: z.enum(['start', 'pause', 'resume', 'stop', 'restart']),
  reason: z.string().max(500).optional(),
});

const RunEvaluationSchema = z.object({
  score: z.number().min(0).max(100),
  criteria: z.record(z.number().min(0).max(100)),
  feedback: z.string().max(2000).optional(),
  reviewer: z.string().max(255).optional(),
});

const CompetitionCreateSchema = z.object({
  name: z.string().min(1).max(255),
  description: z.string().max(1000).optional(),
  task_description: z.string().min(1).max(2000),
  repo_id: z.number().int().positive(),
  worktree_id: z.number().int().positive(),
  agent_types: z.array(z.string().min(1).max(255)),
  max_participants: z.number().int().min(2).max(10).default(3),
  timeout_minutes: z.number().int().min(5).max(480).default(60),
  evaluation_criteria: z.record(z.number().min(0).max(100)),
});

export function createRunsRoutes(pool: Pool): Router {
  const router = Router();

  // Apply common middleware
  router.use(authenticateRequest());
  router.use(validateContentType());

  // GET /api/runs - List all runs with advanced filtering
  router.get('/',
    readOnlyRateLimit,
    validateQuery(AgentRunQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as AgentRunQuery;
      
      const baseQuery = new ApiQueryBuilder('agent_runs')
        .select([
          'ar.*',
          'mr.name as repo_name',
          'mw.name as worktree_name',
          'ws.state as worktree_state',
          'parent_run.agent_id as parent_agent_id',
          'parent_run.task_description as parent_task'
        ])
        .leftJoin('maproom_repos mr', 'mr.id = ar.repo_id')
        .leftJoin('maproom_worktrees mw', 'mw.id = ar.worktree_id')
        .leftJoin('worktree_status ws', 'ws.worktree_id = ar.worktree_id')
        .leftJoin('agent_runs parent_run', 'parent_run.run_id = ar.parent_run_id');

      // Apply filters
      applyFilters(baseQuery, query);

      // Default sort by created_at DESC if no sort specified
      if (!query.sort) {
        baseQuery.orderBy('ar.created_at', 'DESC');
      }

      // Build paginated response
      const result = await buildPaginatedQuery<AgentRunBase>(pool, baseQuery, query);
      
      sendPaginatedSuccess(res, result, 'Runs retrieved successfully');
    })
  );

  // GET /api/runs/active - Get currently active runs
  router.get('/active',
    readOnlyRateLimit,
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const activeQuery = `
        SELECT 
          ar.*,
          mr.name as repo_name,
          mw.name as worktree_name,
          ws.state as worktree_state,
          EXTRACT(EPOCH FROM (NOW() - ar.started_at)) as runtime_seconds
        FROM agent_runs ar
        LEFT JOIN maproom_repos mr ON mr.id = ar.repo_id
        LEFT JOIN maproom_worktrees mw ON mw.id = ar.worktree_id
        LEFT JOIN worktree_status ws ON ws.worktree_id = ar.worktree_id
        WHERE ar.status IN ('pending', 'running')
        ORDER BY ar.started_at ASC
      `;

      const { result } = await queryWithMetrics(pool, activeQuery);

      sendSuccess(res, result, 'Active runs retrieved successfully');
    })
  );

  // GET /api/runs/stats - Get comprehensive run statistics
  router.get('/stats',
    readOnlyRateLimit,
    validateQuery(AgentStatsQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as any;
      
      const statsQuery = `
        SELECT 
          COUNT(*) as total_runs,
          COUNT(*) FILTER (WHERE status = 'completed') as completed_count,
          COUNT(*) FILTER (WHERE status = 'failed') as failed_count,
          COUNT(*) FILTER (WHERE status = 'running') as running_count,
          COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
          COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled_count,
          COUNT(*) FILTER (WHERE status = 'timeout') as timeout_count,
          AVG(duration_ms) FILTER (WHERE duration_ms IS NOT NULL) as avg_duration_ms,
          MIN(duration_ms) FILTER (WHERE duration_ms IS NOT NULL) as min_duration_ms,
          MAX(duration_ms) FILTER (WHERE duration_ms IS NOT NULL) as max_duration_ms,
          AVG(evaluation_score) FILTER (WHERE evaluation_score IS NOT NULL) as avg_evaluation_score,
          COUNT(DISTINCT agent_id) as unique_agents,
          COUNT(DISTINCT repo_id) as unique_repos,
          COUNT(DISTINCT worktree_id) as unique_worktrees,
          COUNT(*) FILTER (WHERE tests_passed = true) as tests_passed_count,
          COUNT(*) FILTER (WHERE auto_merge_eligible = true) as auto_merge_eligible_count,
          COUNT(*) FILTER (WHERE review_required = true) as review_required_count,
          COUNT(*) FILTER (WHERE bookmarked = true) as bookmarked_count,
          SUM(cpu_usage_avg * duration_ms / 1000) FILTER (WHERE cpu_usage_avg IS NOT NULL AND duration_ms IS NOT NULL) as total_cpu_seconds,
          MAX(memory_usage_peak) FILTER (WHERE memory_usage_peak IS NOT NULL) as max_memory_usage,
          SUM(disk_io_bytes) FILTER (WHERE disk_io_bytes IS NOT NULL) as total_disk_io,
          SUM(network_requests) FILTER (WHERE network_requests IS NOT NULL) as total_network_requests
        FROM agent_runs
        WHERE ($1::text IS NULL OR agent_type = $1)
        AND ($2::int IS NULL OR repo_id = $2)
        AND ($3::int IS NULL OR worktree_id = $3)
        AND ($4::timestamptz IS NULL OR started_at >= $4)
        AND ($5::timestamptz IS NULL OR started_at <= $5)
      `;

      const { result: statsResult } = await queryWithMetrics(
        pool,
        statsQuery,
        [
          query.agent_type || null,
          query.repo_id || null,
          query.worktree_id || null,
          query.date_range?.from || null,
          query.date_range?.to || null,
        ]
      );

      const stats = statsResult[0];

      // Get task type distribution
      const taskTypeQuery = `
        SELECT 
          task_type,
          COUNT(*) as count,
          AVG(duration_ms) FILTER (WHERE duration_ms IS NOT NULL) as avg_duration,
          COUNT(*) FILTER (WHERE status = 'completed') as success_count
        FROM agent_runs
        WHERE ($1::timestamptz IS NULL OR started_at >= $1)
        AND ($2::timestamptz IS NULL OR started_at <= $2)
        GROUP BY task_type
        ORDER BY count DESC
        LIMIT 20
      `;

      const { result: taskTypes } = await queryWithMetrics(
        pool,
        taskTypeQuery,
        [query.date_range?.from || null, query.date_range?.to || null]
      );

      // Get agent type distribution
      const agentTypeQuery = `
        SELECT 
          agent_type,
          COUNT(*) as count,
          AVG(evaluation_score) FILTER (WHERE evaluation_score IS NOT NULL) as avg_score,
          COUNT(*) FILTER (WHERE status = 'completed') as success_count
        FROM agent_runs
        WHERE ($1::timestamptz IS NULL OR started_at >= $1)
        AND ($2::timestamptz IS NULL OR started_at <= $2)
        GROUP BY agent_type
        ORDER BY count DESC
      `;

      const { result: agentTypes } = await queryWithMetrics(
        pool,
        agentTypeQuery,
        [query.date_range?.from || null, query.date_range?.to || null]
      );

      // Calculate success rate
      const totalRuns = parseInt(stats.total_runs);
      const completedRuns = parseInt(stats.completed_count);
      const successRate = totalRuns > 0 ? (completedRuns / totalRuns) * 100 : 0;

      const response: AgentStats = {
        total_runs: totalRuns,
        by_status: {
          pending: parseInt(stats.pending_count),
          running: parseInt(stats.running_count),
          completed: parseInt(stats.completed_count),
          failed: parseInt(stats.failed_count),
          cancelled: parseInt(stats.cancelled_count),
          timeout: parseInt(stats.timeout_count),
        },
        by_agent_type: agentTypes.reduce((acc: any, item: any) => {
          acc[item.agent_type] = parseInt(item.count);
          return acc;
        }, {}),
        by_task_type: taskTypes.reduce((acc: any, item: any) => {
          acc[item.task_type] = parseInt(item.count);
          return acc;
        }, {}),
        avg_duration_ms: parseFloat(stats.avg_duration_ms || 0),
        success_rate: successRate,
        avg_evaluation_score: parseFloat(stats.avg_evaluation_score || 0),
        most_active_agents: [], // Populated below
        recent_activity: [], // Populated below
      };

      sendSuccess(res, response, 'Run statistics retrieved successfully');
    })
  );

  // GET /api/runs/search - Search runs with full-text search
  router.get('/search',
    searchRateLimit,
    validateQuery(z.object({
      q: z.string().min(1).max(255),
      limit: z.coerce.number().int().min(1).max(50).default(20),
      offset: z.coerce.number().int().min(0).default(0),
    })),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { q, limit, offset } = req.query as any;

      const searchQuery = `
        SELECT 
          ar.*,
          mr.name as repo_name,
          mw.name as worktree_name,
          ts_rank(
            to_tsvector('english', ar.task_description || ' ' || ar.agent_id || ' ' || COALESCE(ar.error_message, '')),
            plainto_tsquery('english', $1)
          ) as relevance_score
        FROM agent_runs ar
        LEFT JOIN maproom_repos mr ON mr.id = ar.repo_id
        LEFT JOIN maproom_worktrees mw ON mw.id = ar.worktree_id
        WHERE to_tsvector('english', ar.task_description || ' ' || ar.agent_id || ' ' || COALESCE(ar.error_message, ''))
              @@ plainto_tsquery('english', $1)
        ORDER BY relevance_score DESC, ar.created_at DESC
        LIMIT $2 OFFSET $3
      `;

      const countQuery = `
        SELECT COUNT(*) as total
        FROM agent_runs ar
        WHERE to_tsvector('english', ar.task_description || ' ' || ar.agent_id || ' ' || COALESCE(ar.error_message, ''))
              @@ plainto_tsquery('english', $1)
      `;

      const [{ result: searchResults }, { result: countResult }] = await Promise.all([
        queryWithMetrics(pool, searchQuery, [q, limit, offset]),
        queryWithMetrics(pool, countQuery, [q]),
      ]);

      const total = parseInt(countResult[0].total);
      const hasMore = offset + limit < total;

      const response = {
        items: searchResults,
        pagination: {
          total,
          limit,
          offset,
          hasMore,
          nextCursor: hasMore ? btoa(`offset:${offset + limit}`) : undefined,
          prevCursor: offset > 0 ? btoa(`offset:${Math.max(0, offset - limit)}`) : undefined,
        },
        query: q,
      };

      sendSuccess(res, response, 'Search results retrieved successfully');
    })
  );

  // GET /api/runs/:id - Get single run with detailed information
  router.get('/:id',
    readOnlyRateLimit,
    validateParams(UuidSchema.transform(id => ({ id }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      const query = `
        SELECT 
          ar.*,
          mr.name as repo_name,
          mw.name as worktree_name,
          mw.abs_path as worktree_path,
          ws.state as worktree_state,
          parent_run.agent_id as parent_agent_id,
          parent_run.task_description as parent_task,
          (
            SELECT COUNT(*) 
            FROM agent_runs child_runs 
            WHERE child_runs.parent_run_id = ar.run_id
          ) as child_run_count
        FROM agent_runs ar
        LEFT JOIN maproom_repos mr ON mr.id = ar.repo_id
        LEFT JOIN maproom_worktrees mw ON mw.id = ar.worktree_id
        LEFT JOIN worktree_status ws ON ws.worktree_id = ar.worktree_id
        LEFT JOIN agent_runs parent_run ON parent_run.run_id = ar.parent_run_id
        WHERE ar.run_id = $1
      `;

      const { result } = await queryWithMetrics(pool, query, [id]);

      if (result.length === 0) {
        return sendNotFound(res, 'Run', id, req.headers['x-request-id'] as string);
      }

      const run = result[0];

      // Get child runs if any
      if (run.child_run_count > 0) {
        const childRunsQuery = `
          SELECT run_id, agent_id, status, started_at, task_type, evaluation_score
          FROM agent_runs
          WHERE parent_run_id = $1
          ORDER BY started_at ASC
        `;

        const { result: childRuns } = await queryWithMetrics(pool, childRunsQuery, [id]);
        run.child_runs = childRuns;
      }

      sendSuccess(res, run, 'Run retrieved successfully');
    })
  );

  // POST /api/runs - Create new run
  router.post('/',
    standardRateLimit,
    requirePermissions(['run_create']),
    validateBody(AgentRunCreateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const runData = req.body as AgentRunCreate;

      const result = await withTransaction(pool, async (client) => {
        const runId = uuidv4();
        
        const insertQuery = `
          INSERT INTO agent_runs (
            agent_id,
            agent_type,
            run_id,
            parent_run_id,
            repo_id,
            worktree_id,
            commit_sha,
            task_description,
            task_type,
            instructions,
            context_files,
            status,
            started_at,
            tmux_session,
            tmux_window,
            tmux_pane,
            review_required,
            auto_merge_eligible,
            competition_id,
            tags,
            created_at,
            updated_at,
            bookmarked
          ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23
          ) RETURNING *
        `;

        const values = [
          runData.agent_id,
          runData.agent_type,
          runId,
          runData.parent_run_id || null,
          runData.repo_id,
          runData.worktree_id,
          runData.commit_sha,
          runData.task_description,
          runData.task_type,
          JSON.stringify(runData.instructions),
          JSON.stringify(runData.context_files),
          'pending',
          new Date().toISOString(),
          runData.tmux_session || null,
          runData.tmux_window || null,
          runData.tmux_pane || null,
          runData.review_required,
          runData.auto_merge_eligible,
          runData.competition_id || null,
          JSON.stringify(runData.tags),
          new Date().toISOString(),
          new Date().toISOString(),
          false,
        ];

        const result = await client.query(insertQuery, values);
        return result.rows[0];
      });

      sendCreated(res, result, 'Run created successfully');
    })
  );

  // POST /api/runs/:id/action - Perform action on run (start, stop, etc.)
  router.post('/:id/action',
    standardRateLimit,
    requirePermissions(['run_control']),
    validateParams(UuidSchema.transform(id => ({ id }))),
    validateBody(RunActionSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const { action, reason } = req.body as any;

      // Get current run status
      const currentRun = await pool.query(
        'SELECT status, agent_id FROM agent_runs WHERE run_id = $1',
        [id]
      );

      if (currentRun.rows.length === 0) {
        return sendNotFound(res, 'Run', id, req.headers['x-request-id'] as string);
      }

      const currentStatus = currentRun.rows[0].status;
      let newStatus: string;
      let updateFields: string[] = ['updated_at = NOW()'];
      let message: string;

      switch (action) {
        case 'start':
          if (currentStatus !== 'pending') {
            return sendBadRequest(res, `Cannot start run with status: ${currentStatus}`);
          }
          newStatus = 'running';
          updateFields.push('status = $2', 'started_at = NOW()');
          message = 'Run started successfully';
          break;

        case 'stop':
        case 'pause':
          if (!['running', 'pending'].includes(currentStatus)) {
            return sendBadRequest(res, `Cannot ${action} run with status: ${currentStatus}`);
          }
          newStatus = action === 'stop' ? 'cancelled' : 'paused';
          updateFields.push('status = $2');
          if (action === 'stop') {
            updateFields.push('completed_at = NOW()');
            updateFields.push('duration_ms = EXTRACT(EPOCH FROM (NOW() - started_at)) * 1000');
          }
          message = `Run ${action}ped successfully`;
          break;

        case 'resume':
          if (currentStatus !== 'paused') {
            return sendBadRequest(res, `Cannot resume run with status: ${currentStatus}`);
          }
          newStatus = 'running';
          updateFields.push('status = $2');
          message = 'Run resumed successfully';
          break;

        case 'restart':
          if (!['completed', 'failed', 'cancelled', 'timeout'].includes(currentStatus)) {
            return sendBadRequest(res, `Cannot restart run with status: ${currentStatus}`);
          }
          newStatus = 'pending';
          updateFields.push('status = $2', 'started_at = NULL', 'completed_at = NULL', 'duration_ms = NULL', 'error_message = NULL');
          message = 'Run restarted successfully';
          break;

        default:
          return sendBadRequest(res, `Unknown action: ${action}`);
      }

      const updateQuery = `
        UPDATE agent_runs 
        SET ${updateFields.join(', ')}
        WHERE run_id = $1
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, updateQuery, [id, newStatus]);

      sendSuccess(res, result[0], message);
    })
  );

  // POST /api/runs/:id/evaluate - Add evaluation to run
  router.post('/:id/evaluate',
    standardRateLimit,
    requirePermissions(['run_evaluate']),
    validateParams(UuidSchema.transform(id => ({ id }))),
    validateBody(RunEvaluationSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const { score, criteria, feedback, reviewer } = req.body as any;

      const exists = await recordExists(pool, 'agent_runs', { run_id: id });
      if (!exists) {
        return sendNotFound(res, 'Run', id, req.headers['x-request-id'] as string);
      }

      const updateQuery = `
        UPDATE agent_runs 
        SET 
          evaluation_score = $2,
          user_feedback = COALESCE(user_feedback, '{}') || $3,
          updated_at = NOW()
        WHERE run_id = $1
        RETURNING *
      `;

      const evaluationData = {
        score,
        criteria,
        feedback,
        reviewer: reviewer || req.user?.id,
        evaluated_at: new Date().toISOString(),
      };

      const { result } = await queryWithMetrics(pool, updateQuery, [
        id,
        score,
        JSON.stringify(evaluationData),
      ]);

      sendSuccess(res, result[0], 'Run evaluated successfully');
    })
  );

  // POST /api/runs/:id/bookmark - Toggle bookmark status
  router.post('/:id/bookmark',
    standardRateLimit,
    validateParams(UuidSchema.transform(id => ({ id }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const { bookmarked } = req.body as { bookmarked: boolean };

      const updateQuery = `
        UPDATE agent_runs 
        SET bookmarked = $2, updated_at = NOW()
        WHERE run_id = $1
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, updateQuery, [id, bookmarked]);

      if (result.length === 0) {
        return sendNotFound(res, 'Run', id, req.headers['x-request-id'] as string);
      }

      const action = bookmarked ? 'bookmarked' : 'unbookmarked';
      sendSuccess(res, result[0], `Run ${action} successfully`);
    })
  );

  // DELETE /api/runs/:id - Delete run (admin only)
  router.delete('/:id',
    strictRateLimit,
    requirePermissions(['admin']),
    validateParams(UuidSchema.transform(id => ({ id }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      // Check if run has child runs
      const childRunsQuery = `
        SELECT COUNT(*) as count 
        FROM agent_runs 
        WHERE parent_run_id = $1
      `;

      const { result: childResult } = await queryWithMetrics(pool, childRunsQuery, [id]);
      const childCount = parseInt(childResult[0].count);

      if (childCount > 0) {
        return sendBadRequest(
          res,
          'Cannot delete run with child runs',
          { childRuns: childCount },
          req.headers['x-request-id'] as string
        );
      }

      const deleteQuery = 'DELETE FROM agent_runs WHERE run_id = $1';
      const deleteResult = await pool.query(deleteQuery, [id]);

      if (deleteResult.rowCount === 0) {
        return sendNotFound(res, 'Run', id, req.headers['x-request-id'] as string);
      }

      sendNoContent(res);
    })
  );

  return router;
}