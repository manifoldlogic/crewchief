import { Router } from 'express';
import { type Pool } from 'pg';
import {
  AgentRunCreateSchema,
  AgentRunUpdateSchema,
  AgentRunQuerySchema,
  AgentMessageCreateSchema,
  AgentMessageQuerySchema,
  AgentStatsQuerySchema,
  type AgentRunBase,
  type AgentRunCreate,
  type AgentRunUpdate,
  type AgentRunQuery,
  type AgentMessageBase,
  type AgentMessageCreate,
  type AgentMessageQuery,
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

export function createAgentRoutes(pool: Pool): Router {
  const router = Router();

  // Apply common middleware
  router.use(authenticateRequest());
  router.use(validateContentType());

  // GET /api/agents/runs - List agent runs with pagination and filtering
  router.get('/runs',
    readOnlyRateLimit,
    validateQuery(AgentRunQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as AgentRunQuery;
      
      const baseQuery = new ApiQueryBuilder('agent_runs')
        .select([
          'ar.*',
          'mr.name as repo_name',
          'mw.name as worktree_name',
          'ws.state as worktree_state'
        ])
        .leftJoin('maproom_repos mr', 'mr.id = ar.repo_id')
        .leftJoin('maproom_worktrees mw', 'mw.id = ar.worktree_id')
        .leftJoin('worktree_status ws', 'ws.worktree_id = ar.worktree_id');

      // Apply filters
      applyFilters(baseQuery, query);

      // Default sort by created_at DESC if no sort specified
      if (!query.sort) {
        baseQuery.orderBy('ar.created_at', 'DESC');
      }

      // Build paginated response
      const result = await buildPaginatedQuery<AgentRunBase>(pool, baseQuery, query);
      
      sendPaginatedSuccess(res, result, 'Agent runs retrieved successfully');
    })
  );

  // GET /api/agents/runs/stats - Get agent run statistics
  router.get('/runs/stats',
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
          AVG(evaluation_score) FILTER (WHERE evaluation_score IS NOT NULL) as avg_evaluation_score,
          COUNT(DISTINCT agent_id) as unique_agents,
          COUNT(*) FILTER (WHERE tests_passed = true) as tests_passed_count,
          COUNT(*) FILTER (WHERE auto_merge_eligible = true) as auto_merge_eligible_count
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
      const totalRuns = parseInt(stats.total_runs);
      const completedRuns = parseInt(stats.completed_count);
      const successRate = totalRuns > 0 ? (completedRuns / totalRuns) * 100 : 0;

      // Get most active agents
      const activeAgentsQuery = `
        SELECT 
          agent_id,
          COUNT(*) as run_count,
          COUNT(*) FILTER (WHERE status = 'completed') as completed_count,
          AVG(evaluation_score) FILTER (WHERE evaluation_score IS NOT NULL) as avg_score
        FROM agent_runs
        WHERE ($1::timestamptz IS NULL OR started_at >= $1)
        AND ($2::timestamptz IS NULL OR started_at <= $2)
        GROUP BY agent_id
        ORDER BY run_count DESC
        LIMIT 10
      `;

      const { result: activeAgents } = await queryWithMetrics(
        pool,
        activeAgentsQuery,
        [query.date_range?.from || null, query.date_range?.to || null]
      );

      // Get recent activity
      const recentActivityQuery = `
        SELECT 
          run_id,
          agent_id,
          status,
          started_at,
          task_type
        FROM agent_runs
        WHERE started_at >= NOW() - INTERVAL '7 days'
        ORDER BY started_at DESC
        LIMIT 20
      `;

      const { result: recentActivity } = await queryWithMetrics(pool, recentActivityQuery);

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
        by_agent_type: {}, // Would need additional query to get counts by agent type
        by_task_type: {}, // Would need additional query to get counts by task type
        avg_duration_ms: parseFloat(stats.avg_duration_ms || 0),
        success_rate: successRate,
        avg_evaluation_score: parseFloat(stats.avg_evaluation_score || 0),
        most_active_agents: activeAgents.map(agent => ({
          agent_id: agent.agent_id,
          run_count: parseInt(agent.run_count),
          success_rate: agent.completed_count > 0 ? (agent.completed_count / agent.run_count) * 100 : 0,
          avg_score: parseFloat(agent.avg_score || 0),
        })),
        recent_activity: recentActivity,
      };

      sendSuccess(res, response, 'Agent statistics retrieved successfully');
    })
  );

  // GET /api/agents/runs/:id - Get single agent run
  router.get('/runs/:id',
    readOnlyRateLimit,
    validateParams(UuidSchema.transform(id => ({ id }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      const query = new ApiQueryBuilder('agent_runs')
        .select([
          'ar.*',
          'mr.name as repo_name',
          'mw.name as worktree_name',
          'ws.state as worktree_state'
        ])
        .leftJoin('maproom_repos mr', 'mr.id = ar.repo_id')
        .leftJoin('maproom_worktrees mw', 'mw.id = ar.worktree_id')
        .leftJoin('worktree_status ws', 'ws.worktree_id = ar.worktree_id')
        .where('ar.run_id = ?', id);

      const runs = await query.execute(pool);

      if (runs.length === 0) {
        return sendNotFound(res, 'Agent run', id, req.headers['x-request-id'] as string);
      }

      sendSuccess(res, runs[0], 'Agent run retrieved successfully');
    })
  );

  // POST /api/agents/runs - Create new agent run
  router.post('/runs',
    standardRateLimit,
    requirePermissions(['agent_run_create']),
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
          'pending', // Initial status
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
          false, // bookmarked default
        ];

        const result = await client.query(insertQuery, values);
        return result.rows[0];
      });

      sendCreated(res, result, 'Agent run created successfully');
    })
  );

  // PUT /api/agents/runs/:id - Update agent run
  router.put('/runs/:id',
    standardRateLimit,
    requirePermissions(['agent_run_update']),
    validateParams(UuidSchema.transform(id => ({ id }))),
    validateBody(AgentRunUpdateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const updateData = req.body as AgentRunUpdate;

      // Check if run exists
      const exists = await recordExists(pool, 'agent_runs', { run_id: id });
      if (!exists) {
        return sendNotFound(res, 'Agent run', id, req.headers['x-request-id'] as string);
      }

      // Build update query dynamically
      const updateFields: string[] = [];
      const updateValues: any[] = [];
      let paramIndex = 1;

      for (const [key, value] of Object.entries(updateData)) {
        if (value !== undefined) {
          updateFields.push(`${key} = $${paramIndex++}`);
          if (key === 'artifacts' || key === 'user_feedback' || key === 'tags') {
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
      
      // Add duration calculation if status is completed
      if (updateData.status === 'completed' && updateData.completed_at) {
        updateFields.push(`duration_ms = EXTRACT(EPOCH FROM ($${paramIndex}::timestamptz - started_at)) * 1000`);
        updateValues.push(updateData.completed_at);
        paramIndex++;
      }
      
      // Add ID for WHERE clause
      updateValues.push(id);

      const updateQuery = `
        UPDATE agent_runs 
        SET ${updateFields.join(', ')}
        WHERE run_id = $${paramIndex}
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, updateQuery, updateValues);

      if (result.length === 0) {
        return sendNotFound(res, 'Agent run', id, req.headers['x-request-id'] as string);
      }

      sendSuccess(res, result[0], 'Agent run updated successfully');
    })
  );

  // DELETE /api/agents/runs/:id - Cancel/delete agent run
  router.delete('/runs/:id',
    strictRateLimit,
    requirePermissions(['agent_run_delete']),
    validateParams(UuidSchema.transform(id => ({ id }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      // Check if run exists and get current status
      const currentRun = await pool.query(
        'SELECT status FROM agent_runs WHERE run_id = $1',
        [id]
      );

      if (currentRun.rows.length === 0) {
        return sendNotFound(res, 'Agent run', id, req.headers['x-request-id'] as string);
      }

      const currentStatus = currentRun.rows[0].status;

      // Only allow cancellation of pending or running jobs
      if (!['pending', 'running'].includes(currentStatus)) {
        return sendBadRequest(
          res,
          `Cannot cancel agent run with status: ${currentStatus}`,
          { currentStatus },
          req.headers['x-request-id'] as string
        );
      }

      // Update status to cancelled
      const cancelQuery = `
        UPDATE agent_runs 
        SET 
          status = 'cancelled',
          completed_at = NOW(),
          duration_ms = EXTRACT(EPOCH FROM (NOW() - started_at)) * 1000,
          updated_at = NOW()
        WHERE run_id = $1
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, cancelQuery, [id]);

      sendSuccess(res, result[0], 'Agent run cancelled successfully');
    })
  );

  // GET /api/agents/messages - List agent messages
  router.get('/messages',
    readOnlyRateLimit,
    validateQuery(AgentMessageQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as AgentMessageQuery;
      
      const baseQuery = new ApiQueryBuilder('agent_messages')
        .select([
          'am.*',
          'ar.agent_id as run_agent_id',
          'ar.task_type as run_task_type'
        ])
        .leftJoin('agent_runs ar', 'ar.run_id = am.run_id');

      // Apply filters
      applyFilters(baseQuery, query);

      // Default sort by created_at DESC if no sort specified
      if (!query.sort) {
        baseQuery.orderBy('am.created_at', 'DESC');
      }

      // Build paginated response
      const result = await buildPaginatedQuery<AgentMessageBase>(pool, baseQuery, query);
      
      sendPaginatedSuccess(res, result, 'Agent messages retrieved successfully');
    })
  );

  // POST /api/agents/messages - Create new agent message
  router.post('/messages',
    standardRateLimit,
    requirePermissions(['agent_message_create']),
    validateBody(AgentMessageCreateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const messageData = req.body as AgentMessageCreate;

      const result = await withTransaction(pool, async (client) => {
        const messageId = uuidv4();
        
        const insertQuery = `
          INSERT INTO agent_messages (
            message_id,
            correlation_id,
            reply_to_id,
            run_id,
            sender_agent_id,
            recipient_agent_id,
            message_type,
            priority,
            subject,
            content,
            content_format,
            metadata,
            attachments,
            broadcast,
            created_at,
            expires_at,
            processed,
            retry_count,
            max_retries,
            bus_topic,
            tags,
            size_bytes
          ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22
          ) RETURNING *
        `;

        const sizeBytes = Buffer.byteLength(messageData.content, 'utf8');

        const values = [
          messageId,
          messageData.correlation_id || null,
          messageData.reply_to_id || null,
          messageData.run_id,
          messageData.sender_agent_id,
          messageData.recipient_agent_id || null,
          messageData.message_type,
          messageData.priority,
          messageData.subject || null,
          messageData.content,
          messageData.content_format,
          JSON.stringify(messageData.metadata),
          JSON.stringify(messageData.attachments),
          messageData.broadcast,
          new Date().toISOString(),
          messageData.expires_at || null,
          false, // processed default
          0, // retry_count default
          messageData.max_retries,
          messageData.bus_topic || null,
          JSON.stringify(messageData.tags),
          sizeBytes,
        ];

        const result = await client.query(insertQuery, values);
        return result.rows[0];
      });

      sendCreated(res, result, 'Agent message created successfully');
    })
  );

  // PUT /api/agents/messages/:id/acknowledge - Acknowledge message
  router.put('/messages/:id/acknowledge',
    standardRateLimit,
    validateParams(UuidSchema.transform(id => ({ id }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      const ackQuery = `
        UPDATE agent_messages 
        SET acknowledged_at = NOW()
        WHERE message_id = $1
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, ackQuery, [id]);

      if (result.length === 0) {
        return sendNotFound(res, 'Agent message', id, req.headers['x-request-id'] as string);
      }

      sendSuccess(res, result[0], 'Message acknowledged successfully');
    })
  );

  return router;
}