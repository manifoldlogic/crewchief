import { z } from 'zod';
import { 
  IdSchema, 
  UuidSchema, 
  TimestampSchema, 
  NonNegativeIntSchema,
  PaginationQuerySchema,
  DateRangeFilterSchema,
  SearchFilterSchema,
} from './common.js';

// Enums
export const WorktreeStateSchema = z.enum(['active', 'stale', 'merging', 'archived', 'error']);
export const GitFileStatusSchema = z.enum([
  'unmodified', 'modified', 'added', 'deleted', 'renamed', 'copied', 'unmerged', 'untracked', 'ignored'
]);

// Core worktree schemas
export const WorktreeBaseSchema = z.object({
  id: z.number().int().positive(),
  worktree_id: z.number().int().positive(),
  repo_id: z.number().int().positive(),
  worktree_name: z.string().min(1).max(255),
  worktree_path: z.string().min(1),
  current_branch: z.string().min(1).max(255),
  upstream_branch: z.string().max(255).nullable(),
  state: WorktreeStateSchema,
  is_clean: z.boolean(),
  is_synced: z.boolean(),
  head_commit_sha: z.string().length(40),
  head_commit_message: z.string().max(1000).nullable(),
  head_commit_author: z.string().max(255).nullable(),
  head_commit_date: TimestampSchema.nullable(),
  commits_ahead: NonNegativeIntSchema,
  commits_behind: NonNegativeIntSchema,
  modified_files: NonNegativeIntSchema,
  added_files: NonNegativeIntSchema,
  deleted_files: NonNegativeIntSchema,
  untracked_files: NonNegativeIntSchema,
  staged_files: NonNegativeIntSchema,
  file_changes: z.record(z.any()).nullable(),
  total_files: NonNegativeIntSchema,
  total_size_bytes: z.number().int().min(0),
  programming_languages: z.record(z.any()).nullable(),
  active_agents: z.record(z.any()).nullable(),
  tmux_sessions: z.array(z.string()),
  disk_usage_bytes: z.number().int().min(0),
  last_build_status: z.string().max(50).nullable(),
  last_build_time: TimestampSchema.nullable(),
  test_status: z.string().max(50).nullable(),
  test_coverage: z.number().min(0).max(100).nullable(),
  maproom_indexed_at: TimestampSchema.nullable(),
  maproom_index_status: z.string().max(50).nullable(),
  chunk_count: NonNegativeIntSchema,
  last_scan_at: TimestampSchema.nullable(),
  scan_duration_ms: NonNegativeIntSchema,
  cache_version: NonNegativeIntSchema,
  last_error: z.string().max(1000).nullable(),
  error_count: NonNegativeIntSchema,
  last_accessed_at: TimestampSchema.nullable(),
  pinned: z.boolean(),
  tags: z.array(z.string()),
  notes: z.string().max(2000).nullable(),
  created_at: TimestampSchema,
  updated_at: TimestampSchema,
});

export const WorktreeCreateSchema = z.object({
  worktree_id: z.number().int().positive(),
  repo_id: z.number().int().positive(),
  worktree_name: z.string().min(1).max(255),
  worktree_path: z.string().min(1),
  current_branch: z.string().min(1).max(255),
  upstream_branch: z.string().max(255).optional(),
  state: WorktreeStateSchema.default('active'),
  notes: z.string().max(2000).optional(),
  tags: z.array(z.string()).default([]),
  pinned: z.boolean().default(false),
});

export const WorktreeUpdateSchema = z.object({
  worktree_name: z.string().min(1).max(255).optional(),
  current_branch: z.string().min(1).max(255).optional(),
  upstream_branch: z.string().max(255).nullable().optional(),
  state: WorktreeStateSchema.optional(),
  notes: z.string().max(2000).nullable().optional(),
  tags: z.array(z.string()).optional(),
  pinned: z.boolean().optional(),
  last_accessed_at: TimestampSchema.optional(),
});

// Query schemas
export const WorktreeQuerySchema = PaginationQuerySchema.extend({
  repo_id: z.coerce.number().int().positive().optional(),
  state: WorktreeStateSchema.optional(),
  is_clean: z.coerce.boolean().optional(),
  is_synced: z.coerce.boolean().optional(),
  pinned: z.coerce.boolean().optional(),
  has_active_agents: z.coerce.boolean().optional(),
  created_range: DateRangeFilterSchema.optional(),
  updated_range: DateRangeFilterSchema.optional(),
  search: SearchFilterSchema.optional(),
  tags: z.array(z.string()).optional(),
});

export const WorktreeStatsQuerySchema = z.object({
  repo_id: z.coerce.number().int().positive().optional(),
  state: WorktreeStateSchema.optional(),
  date_range: DateRangeFilterSchema.optional(),
});

// Response schemas
export const WorktreeStatsSchema = z.object({
  total_worktrees: z.number().int().min(0),
  by_state: z.record(WorktreeStateSchema, z.number().int().min(0)),
  by_repo: z.record(z.string(), z.number().int().min(0)),
  total_disk_usage: z.number().int().min(0),
  avg_files_per_worktree: z.number().min(0),
  most_active_worktrees: z.array(z.object({
    id: z.number().int().positive(),
    name: z.string(),
    access_count: z.number().int().min(0),
    last_accessed: TimestampSchema.nullable(),
  })),
  recent_activity: z.array(z.object({
    worktree_id: z.number().int().positive(),
    event_type: z.string(),
    timestamp: TimestampSchema,
    details: z.record(z.any()).optional(),
  })),
});

// Type exports
export type WorktreeBase = z.infer<typeof WorktreeBaseSchema>;
export type WorktreeCreate = z.infer<typeof WorktreeCreateSchema>;
export type WorktreeUpdate = z.infer<typeof WorktreeUpdateSchema>;
export type WorktreeQuery = z.infer<typeof WorktreeQuerySchema>;
export type WorktreeStatsQuery = z.infer<typeof WorktreeStatsQuerySchema>;
export type WorktreeStats = z.infer<typeof WorktreeStatsSchema>;
export type WorktreeState = z.infer<typeof WorktreeStateSchema>;
export type GitFileStatus = z.infer<typeof GitFileStatusSchema>;