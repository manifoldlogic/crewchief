import { Router } from 'express';
import { type Pool } from 'pg';
import {
  PreferenceCreateSchema,
  PreferenceUpdateSchema,
  PreferenceQuerySchema,
  SystemConfigCreateSchema,
  SystemConfigUpdateSchema,
  SystemConfigQuerySchema,
  BulkPreferenceUpdateSchema,
  BulkSystemConfigUpdateSchema,
  type PreferenceBase,
  type PreferenceCreate,
  type PreferenceUpdate,
  type PreferenceQuery,
  type SystemConfigBase,
  type SystemConfigCreate,
  type SystemConfigUpdate,
  type SystemConfigQuery,
  type BulkPreferenceUpdate,
  type BulkSystemConfigUpdate,
  type ConfigStats,
} from '../schemas/config.js';
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
  bulkOperationRateLimit,
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
  upsert,
} from '../utils/database.js';
import { IdSchema } from '../schemas/common.js';

export function createConfigRoutes(pool: Pool): Router {
  const router = Router();

  // Apply common middleware
  router.use(authenticateRequest());
  router.use(validateContentType());

  // GET /api/config/preferences - List user preferences
  router.get('/preferences',
    readOnlyRateLimit,
    validateQuery(PreferenceQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as PreferenceQuery;
      
      const baseQuery = new ApiQueryBuilder('web_ui_preferences')
        .select([
          'id',
          'session_id',
          'user_id',
          'preference_key',
          'preference_value',
          'created_at',
          'updated_at',
          'scope',
          'context_id',
          'version'
        ]);

      // Apply filters
      applyFilters(baseQuery, query);

      // Filter by user if not admin
      if (!req.user?.permissions?.includes('admin')) {
        baseQuery.where('user_id = ?', req.user?.id);
      }

      // Default sort by updated_at DESC if no sort specified
      if (!query.sort) {
        baseQuery.orderBy('updated_at', 'DESC');
      }

      // Build paginated response
      const result = await buildPaginatedQuery<PreferenceBase>(pool, baseQuery, query);
      
      sendPaginatedSuccess(res, result, 'Preferences retrieved successfully');
    })
  );

  // GET /api/config/preferences/stats - Get preference statistics
  router.get('/preferences/stats',
    readOnlyRateLimit,
    requirePermissions(['admin']),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const statsQuery = `
        SELECT 
          COUNT(*) as total_preferences,
          COUNT(DISTINCT user_id) as unique_users,
          COUNT(DISTINCT session_id) as unique_sessions,
          COUNT(*) FILTER (WHERE scope = 'global') as global_count,
          COUNT(*) FILTER (WHERE scope = 'repository') as repository_count,
          COUNT(*) FILTER (WHERE scope = 'worktree') as worktree_count,
          COUNT(*) FILTER (WHERE scope = 'page') as page_count
        FROM web_ui_preferences
      `;

      const { result: statsResult } = await queryWithMetrics(pool, statsQuery);
      const stats = statsResult[0];

      // Get system config stats
      const systemStatsQuery = `
        SELECT 
          COUNT(*) as total_system_configs,
          COUNT(*) FILTER (WHERE is_sensitive = true) as sensitive_configs,
          COUNT(*) FILTER (WHERE requires_restart = true) as restart_required_configs,
          COUNT(DISTINCT category) as unique_categories
        FROM system_config
      `;

      const { result: systemStats } = await queryWithMetrics(pool, systemStatsQuery);
      const sysStats = systemStats[0] || { total_system_configs: 0, sensitive_configs: 0, restart_required_configs: 0 };

      // Get recent changes
      const recentChangesQuery = `
        (
          SELECT 'preference' as type, preference_key as key, user_id, updated_at
          FROM web_ui_preferences
          WHERE updated_at >= NOW() - INTERVAL '7 days'
        )
        UNION ALL
        (
          SELECT 'system_config' as type, config_key as key, NULL as user_id, updated_at
          FROM system_config
          WHERE updated_at >= NOW() - INTERVAL '7 days'
        )
        ORDER BY updated_at DESC
        LIMIT 20
      `;

      const { result: recentChanges } = await queryWithMetrics(pool, recentChangesQuery);

      const response: ConfigStats = {
        total_preferences: parseInt(stats.total_preferences),
        by_scope: {
          global: parseInt(stats.global_count),
          repository: parseInt(stats.repository_count),
          worktree: parseInt(stats.worktree_count),
          page: parseInt(stats.page_count),
        },
        by_user: {}, // Would need additional query to group by user
        total_system_configs: parseInt(sysStats.total_system_configs),
        by_category: {}, // Would need additional query to group by category
        sensitive_configs: parseInt(sysStats.sensitive_configs),
        restart_required_configs: parseInt(sysStats.restart_required_configs),
        recent_changes: recentChanges,
      };

      sendSuccess(res, response, 'Configuration statistics retrieved successfully');
    })
  );

  // GET /api/config/preferences/:id - Get single preference
  router.get('/preferences/:id',
    readOnlyRateLimit,
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      const query = new ApiQueryBuilder('web_ui_preferences')
        .where('id = ?', id);

      // Non-admin users can only see their own preferences
      if (!req.user?.permissions?.includes('admin')) {
        query.where('user_id = ?', req.user?.id);
      }

      const preferences = await query.execute(pool);

      if (preferences.length === 0) {
        return sendNotFound(res, 'Preference', id, req.headers['x-request-id'] as string);
      }

      sendSuccess(res, preferences[0], 'Preference retrieved successfully');
    })
  );

  // POST /api/config/preferences - Create new preference
  router.post('/preferences',
    standardRateLimit,
    validateBody(PreferenceCreateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const prefData = req.body as PreferenceCreate;

      // Ensure user can only create preferences for themselves (unless admin)
      if (!req.user?.permissions?.includes('admin') && prefData.user_id !== req.user?.id) {
        return sendBadRequest(
          res,
          'Cannot create preferences for other users',
          undefined,
          req.headers['x-request-id'] as string
        );
      }

      const result = await withTransaction(pool, async (client) => {
        // Use upsert to handle conflicts on (user_id, preference_key, scope, context_id)
        const upsertResult = await upsert(
          pool,
          'web_ui_preferences',
          {
            session_id: prefData.session_id,
            user_id: prefData.user_id,
            preference_key: prefData.preference_key,
            preference_value: JSON.stringify(prefData.preference_value),
            scope: prefData.scope,
            context_id: prefData.context_id || null,
            version: 1,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
          ['user_id', 'preference_key', 'scope', 'context_id'],
          ['preference_value', 'session_id', 'updated_at', 'version']
        );

        return upsertResult;
      });

      sendCreated(res, result, 'Preference created successfully');
    })
  );

  // PUT /api/config/preferences/:id - Update preference
  router.put('/preferences/:id',
    standardRateLimit,
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    validateBody(PreferenceUpdateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const updateData = req.body as PreferenceUpdate;

      // Check if preference exists and user has permission
      const existingPref = await pool.query(
        'SELECT user_id FROM web_ui_preferences WHERE id = $1',
        [id]
      );

      if (existingPref.rows.length === 0) {
        return sendNotFound(res, 'Preference', id, req.headers['x-request-id'] as string);
      }

      // Non-admin users can only update their own preferences
      if (!req.user?.permissions?.includes('admin') && 
          existingPref.rows[0].user_id !== req.user?.id) {
        return sendBadRequest(
          res,
          'Cannot update preferences for other users',
          undefined,
          req.headers['x-request-id'] as string
        );
      }

      const updateQuery = `
        UPDATE web_ui_preferences 
        SET 
          preference_value = $2,
          scope = COALESCE($3, scope),
          context_id = COALESCE($4, context_id),
          version = version + 1,
          updated_at = NOW()
        WHERE id = $1
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, updateQuery, [
        id,
        JSON.stringify(updateData.preference_value),
        updateData.scope,
        updateData.context_id,
      ]);

      sendSuccess(res, result[0], 'Preference updated successfully');
    })
  );

  // DELETE /api/config/preferences/:id - Delete preference
  router.delete('/preferences/:id',
    standardRateLimit,
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      // Check permission
      const existingPref = await pool.query(
        'SELECT user_id FROM web_ui_preferences WHERE id = $1',
        [id]
      );

      if (existingPref.rows.length === 0) {
        return sendNotFound(res, 'Preference', id, req.headers['x-request-id'] as string);
      }

      if (!req.user?.permissions?.includes('admin') && 
          existingPref.rows[0].user_id !== req.user?.id) {
        return sendBadRequest(
          res,
          'Cannot delete preferences for other users',
          undefined,
          req.headers['x-request-id'] as string
        );
      }

      await pool.query('DELETE FROM web_ui_preferences WHERE id = $1', [id]);
      sendNoContent(res);
    })
  );

  // POST /api/config/preferences/bulk - Bulk update preferences
  router.post('/preferences/bulk',
    bulkOperationRateLimit,
    validateBody(BulkPreferenceUpdateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const bulkData = req.body as BulkPreferenceUpdate;

      // Ensure user can only update their own preferences (unless admin)
      if (!req.user?.permissions?.includes('admin') && bulkData.user_id !== req.user?.id) {
        return sendBadRequest(
          res,
          'Cannot bulk update preferences for other users',
          undefined,
          req.headers['x-request-id'] as string
        );
      }

      const results = await withTransaction(pool, async (client) => {
        const upsertResults = [];
        
        for (const pref of bulkData.preferences) {
          const result = await upsert(
            pool,
            'web_ui_preferences',
            {
              session_id: bulkData.session_id,
              user_id: bulkData.user_id,
              preference_key: pref.preference_key,
              preference_value: JSON.stringify(pref.preference_value),
              scope: pref.scope,
              context_id: pref.context_id || null,
              version: 1,
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            },
            ['user_id', 'preference_key', 'scope', 'context_id'],
            ['preference_value', 'session_id', 'updated_at', 'version']
          );
          
          upsertResults.push(result);
        }
        
        return upsertResults;
      });

      sendSuccess(res, results, `${results.length} preferences updated successfully`);
    })
  );

  // GET /api/config/system - List system configuration
  router.get('/system',
    readOnlyRateLimit,
    requirePermissions(['admin']),
    validateQuery(SystemConfigQuerySchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const query = req.query as SystemConfigQuery;
      
      const baseQuery = new ApiQueryBuilder('system_config')
        .select([
          'id',
          'config_key',
          'config_value',
          'description',
          'is_sensitive',
          'requires_restart',
          'category',
          'default_value',
          'created_at',
          'updated_at',
          'version'
        ]);

      // Apply filters
      applyFilters(baseQuery, query);

      // Default sort by category, then config_key
      if (!query.sort) {
        baseQuery.orderBy('category', 'ASC').orderBy('config_key', 'ASC');
      }

      // Build paginated response
      const result = await buildPaginatedQuery<SystemConfigBase>(pool, baseQuery, query);
      
      // Mask sensitive values in response
      result.items = result.items.map(config => ({
        ...config,
        config_value: config.is_sensitive ? '***REDACTED***' : config.config_value,
      }));
      
      sendPaginatedSuccess(res, result, 'System configuration retrieved successfully');
    })
  );

  // GET /api/config/system/:id - Get single system config
  router.get('/system/:id',
    readOnlyRateLimit,
    requirePermissions(['admin']),
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      const query = new ApiQueryBuilder('system_config')
        .where('id = ?', id);

      const configs = await query.execute(pool);

      if (configs.length === 0) {
        return sendNotFound(res, 'System configuration', id, req.headers['x-request-id'] as string);
      }

      const config = configs[0];
      
      // Mask sensitive values
      if (config.is_sensitive) {
        config.config_value = '***REDACTED***';
      }

      sendSuccess(res, config, 'System configuration retrieved successfully');
    })
  );

  // POST /api/config/system - Create system configuration
  router.post('/system',
    strictRateLimit,
    requirePermissions(['admin']),
    validateBody(SystemConfigCreateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const configData = req.body as SystemConfigCreate;

      const result = await withTransaction(pool, async (client) => {
        const insertQuery = `
          INSERT INTO system_config (
            config_key,
            config_value,
            description,
            is_sensitive,
            requires_restart,
            category,
            validation_schema,
            default_value,
            version,
            created_at,
            updated_at
          ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
          ) RETURNING *
        `;

        const values = [
          configData.config_key,
          JSON.stringify(configData.config_value),
          configData.description || null,
          configData.is_sensitive,
          configData.requires_restart,
          configData.category,
          configData.validation_schema || null,
          configData.default_value ? JSON.stringify(configData.default_value) : null,
          1, // version
          new Date().toISOString(),
          new Date().toISOString(),
        ];

        const result = await client.query(insertQuery, values);
        return result.rows[0];
      });

      // Mask sensitive values in response
      if (result.is_sensitive) {
        result.config_value = '***REDACTED***';
      }

      sendCreated(res, result, 'System configuration created successfully');
    })
  );

  // PUT /api/config/system/:id - Update system configuration
  router.put('/system/:id',
    strictRateLimit,
    requirePermissions(['admin']),
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    validateBody(SystemConfigUpdateSchema),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;
      const updateData = req.body as SystemConfigUpdate;

      // Check if config exists
      const exists = await recordExists(pool, 'system_config', { id });
      if (!exists) {
        return sendNotFound(res, 'System configuration', id, req.headers['x-request-id'] as string);
      }

      // Build update query dynamically
      const updateFields: string[] = [];
      const updateValues: any[] = [];
      let paramIndex = 1;

      for (const [key, value] of Object.entries(updateData)) {
        if (value !== undefined) {
          updateFields.push(`${key} = $${paramIndex++}`);
          if (key === 'config_value' || key === 'default_value') {
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

      // Add version increment and updated_at
      updateFields.push(`version = version + 1`);
      updateFields.push(`updated_at = $${paramIndex++}`);
      updateValues.push(new Date().toISOString());
      
      // Add ID for WHERE clause
      updateValues.push(id);

      const updateQuery = `
        UPDATE system_config 
        SET ${updateFields.join(', ')}
        WHERE id = $${paramIndex}
        RETURNING *
      `;

      const { result } = await queryWithMetrics(pool, updateQuery, updateValues);

      if (result.length === 0) {
        return sendNotFound(res, 'System configuration', id, req.headers['x-request-id'] as string);
      }

      const config = result[0];
      
      // Mask sensitive values in response
      if (config.is_sensitive) {
        config.config_value = '***REDACTED***';
      }

      sendSuccess(res, config, 'System configuration updated successfully');
    })
  );

  // DELETE /api/config/system/:id - Delete system configuration
  router.delete('/system/:id',
    strictRateLimit,
    requirePermissions(['admin']),
    validateParams(IdSchema.transform(id => ({ id: parseInt(id, 10) }))),
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const { id } = req.params as any;

      const exists = await recordExists(pool, 'system_config', { id });
      if (!exists) {
        return sendNotFound(res, 'System configuration', id, req.headers['x-request-id'] as string);
      }

      await pool.query('DELETE FROM system_config WHERE id = $1', [id]);
      sendNoContent(res);
    })
  );

  // POST /api/config/reset - Reset user preferences to defaults
  router.post('/reset',
    standardRateLimit,
    asyncHandler(async (req: AuthenticatedRequest, res) => {
      const userId = req.user?.id;
      
      if (!userId) {
        return sendBadRequest(
          res,
          'User authentication required',
          undefined,
          req.headers['x-request-id'] as string
        );
      }

      // Delete all user preferences (they will fall back to system defaults)
      const deleteQuery = `
        DELETE FROM web_ui_preferences 
        WHERE user_id = $1
        RETURNING count(*)
      `;

      const { result } = await queryWithMetrics(pool, deleteQuery, [userId]);
      const deletedCount = result.length;

      sendSuccess(res, { deletedCount }, `${deletedCount} preferences reset to defaults`);
    })
  );

  return router;
}