import { z } from 'zod';
import { 
  IdSchema, 
  UuidSchema, 
  TimestampSchema,
  PaginationQuerySchema,
  DateRangeFilterSchema,
  SearchFilterSchema,
} from './common.js';

// Enums
export const PreferenceScope = z.enum(['global', 'repository', 'worktree', 'page']);

export const PreferenceKeySchema = z.enum([
  // UI Preferences
  'theme',
  'layout',
  'sidebar_collapsed',
  'table_page_size',
  'default_sort',
  'auto_refresh',
  'notifications_enabled',
  
  // Search Preferences
  'search_default_type',
  'search_result_limit',
  'search_auto_complete',
  'search_history_enabled',
  
  // Agent Preferences
  'agent_auto_start',
  'agent_timeout_seconds',
  'agent_retry_count',
  'agent_parallel_limit',
  
  // Worktree Preferences
  'worktree_auto_cleanup',
  'worktree_sync_frequency',
  'worktree_backup_enabled',
  
  // System Preferences
  'log_level',
  'api_rate_limit',
  'session_timeout_minutes',
  'cache_enabled',
]);

// Configuration schemas
export const PreferenceBaseSchema = z.object({
  id: z.number().int().positive(),
  session_id: UuidSchema,
  user_id: z.string().min(1).max(255),
  preference_key: PreferenceKeySchema,
  preference_value: z.record(z.any()),
  created_at: TimestampSchema,
  updated_at: TimestampSchema,
  scope: PreferenceScope,
  context_id: z.string().max(255).nullable(),
  version: z.number().int().min(1),
});

export const PreferenceCreateSchema = z.object({
  session_id: UuidSchema,
  user_id: z.string().min(1).max(255),
  preference_key: PreferenceKeySchema,
  preference_value: z.record(z.any()),
  scope: PreferenceScope.default('global'),
  context_id: z.string().max(255).optional(),
});

export const PreferenceUpdateSchema = z.object({
  preference_value: z.record(z.any()),
  scope: PreferenceScope.optional(),
  context_id: z.string().max(255).nullable().optional(),
});

// Specific preference value schemas
export const ThemePreferenceSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']),
  primary_color: z.string().regex(/^#[0-9A-Fa-f]{6}$/),
  accent_color: z.string().regex(/^#[0-9A-Fa-f]{6}$/),
});

export const LayoutPreferenceSchema = z.object({
  layout: z.enum(['grid', 'list', 'table']),
  density: z.enum(['compact', 'comfortable', 'spacious']),
  columns_visible: z.array(z.string()),
});

export const SearchPreferenceSchema = z.object({
  default_type: z.enum(['semantic', 'fulltext', 'symbol', 'path']),
  result_limit: z.number().int().min(10).max(100),
  auto_complete: z.boolean(),
  history_enabled: z.boolean(),
  filters: z.record(z.any()),
});

export const AgentPreferenceSchema = z.object({
  auto_start: z.boolean(),
  timeout_seconds: z.number().int().min(30).max(3600),
  retry_count: z.number().int().min(0).max(10),
  parallel_limit: z.number().int().min(1).max(20),
  default_agent_type: z.enum(['claude', 'gemini', 'mock', 'custom']),
});

// System configuration schemas
export const SystemConfigSchema = z.object({
  id: z.number().int().positive(),
  config_key: z.string().min(1).max(255),
  config_value: z.record(z.any()),
  description: z.string().max(500).nullable(),
  is_sensitive: z.boolean(),
  requires_restart: z.boolean(),
  category: z.string().max(100),
  validation_schema: z.string().max(2000).nullable(),
  default_value: z.record(z.any()).nullable(),
  created_at: TimestampSchema,
  updated_at: TimestampSchema,
  version: z.number().int().min(1),
});

export const SystemConfigCreateSchema = z.object({
  config_key: z.string().min(1).max(255),
  config_value: z.record(z.any()),
  description: z.string().max(500).optional(),
  is_sensitive: z.boolean().default(false),
  requires_restart: z.boolean().default(false),
  category: z.string().max(100).default('general'),
  validation_schema: z.string().max(2000).optional(),
  default_value: z.record(z.any()).optional(),
});

export const SystemConfigUpdateSchema = z.object({
  config_value: z.record(z.any()).optional(),
  description: z.string().max(500).nullable().optional(),
  is_sensitive: z.boolean().optional(),
  requires_restart: z.boolean().optional(),
  category: z.string().max(100).optional(),
  validation_schema: z.string().max(2000).nullable().optional(),
  default_value: z.record(z.any()).nullable().optional(),
});

// Query schemas
export const PreferenceQuerySchema = PaginationQuerySchema.extend({
  session_id: UuidSchema.optional(),
  user_id: z.string().optional(),
  preference_key: PreferenceKeySchema.optional(),
  scope: PreferenceScope.optional(),
  context_id: z.string().optional(),
  updated_range: DateRangeFilterSchema.optional(),
  search: SearchFilterSchema.optional(),
});

export const SystemConfigQuerySchema = PaginationQuerySchema.extend({
  config_key: z.string().optional(),
  category: z.string().optional(),
  is_sensitive: z.coerce.boolean().optional(),
  requires_restart: z.coerce.boolean().optional(),
  updated_range: DateRangeFilterSchema.optional(),
  search: SearchFilterSchema.optional(),
});

// Bulk operation schemas
export const BulkPreferenceUpdateSchema = z.object({
  preferences: z.array(z.object({
    preference_key: PreferenceKeySchema,
    preference_value: z.record(z.any()),
    scope: PreferenceScope.default('global'),
    context_id: z.string().max(255).optional(),
  })),
  session_id: UuidSchema,
  user_id: z.string().min(1).max(255),
});

export const BulkSystemConfigUpdateSchema = z.object({
  configs: z.array(z.object({
    config_key: z.string().min(1).max(255),
    config_value: z.record(z.any()),
  })),
});

// Response schemas
export const ConfigStatsSchema = z.object({
  total_preferences: z.number().int().min(0),
  by_scope: z.record(PreferenceScope, z.number().int().min(0)),
  by_user: z.record(z.string(), z.number().int().min(0)),
  total_system_configs: z.number().int().min(0),
  by_category: z.record(z.string(), z.number().int().min(0)),
  sensitive_configs: z.number().int().min(0),
  restart_required_configs: z.number().int().min(0),
  recent_changes: z.array(z.object({
    type: z.enum(['preference', 'system_config']),
    key: z.string(),
    user_id: z.string().optional(),
    updated_at: TimestampSchema,
  })),
});

export const PreferenceExportSchema = z.object({
  preferences: z.array(PreferenceBaseSchema),
  exported_at: TimestampSchema,
  total_count: z.number().int().min(0),
  filters_applied: z.record(z.any()),
});

// Type exports
export type PreferenceBase = z.infer<typeof PreferenceBaseSchema>;
export type PreferenceCreate = z.infer<typeof PreferenceCreateSchema>;
export type PreferenceUpdate = z.infer<typeof PreferenceUpdateSchema>;
export type PreferenceQuery = z.infer<typeof PreferenceQuerySchema>;
export type SystemConfigBase = z.infer<typeof SystemConfigSchema>;
export type SystemConfigCreate = z.infer<typeof SystemConfigCreateSchema>;
export type SystemConfigUpdate = z.infer<typeof SystemConfigUpdateSchema>;
export type SystemConfigQuery = z.infer<typeof SystemConfigQuerySchema>;
export type BulkPreferenceUpdate = z.infer<typeof BulkPreferenceUpdateSchema>;
export type BulkSystemConfigUpdate = z.infer<typeof BulkSystemConfigUpdateSchema>;
export type ConfigStats = z.infer<typeof ConfigStatsSchema>;
export type PreferenceExport = z.infer<typeof PreferenceExportSchema>;
export type PreferenceScope = z.infer<typeof PreferenceScope>;
export type PreferenceKey = z.infer<typeof PreferenceKeySchema>;