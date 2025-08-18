import { z } from 'zod';
import { 
  IdSchema, 
  UuidSchema, 
  TimestampSchema, 
  NonNegativeIntSchema,
  PositiveIntSchema,
  PaginationQuerySchema,
  DateRangeFilterSchema,
  SearchFilterSchema,
} from './common.js';

// Enums
export const AgentTypeSchema = z.enum(['claude', 'gemini', 'mock', 'custom']);
export const AgentStatusSchema = z.enum(['pending', 'running', 'completed', 'failed', 'cancelled', 'timeout']);
export const MessageTypeSchema = z.enum([
  'command', 'response', 'notification', 'error', 'log', 'status_update', 
  'file_change', 'git_event', 'system_event'
]);
export const MessagePrioritySchema = z.enum(['low', 'normal', 'high', 'critical']);

// Agent run schemas
export const AgentRunBaseSchema = z.object({
  id: z.number().int().positive(),
  agent_id: z.string().min(1).max(255),
  agent_type: AgentTypeSchema,
  run_id: UuidSchema,
  parent_run_id: UuidSchema.nullable(),
  repo_id: z.number().int().positive(),
  worktree_id: z.number().int().positive(),
  commit_sha: z.string().length(40),
  task_description: z.string().min(1).max(2000),
  task_type: z.string().min(1).max(100),
  instructions: z.record(z.any()),
  context_files: z.array(z.string()),
  status: AgentStatusSchema,
  started_at: TimestampSchema,
  completed_at: TimestampSchema.nullable(),
  duration_ms: z.number().int().min(0).nullable(),
  tmux_session: z.string().max(255).nullable(),
  tmux_window: z.number().int().min(0).nullable(),
  tmux_pane: z.number().int().min(0).nullable(),
  exit_code: z.number().int().nullable(),
  error_message: z.string().max(2000).nullable(),
  artifacts: z.record(z.any()).nullable(),
  evaluation_score: z.number().min(0).max(100).nullable(),
  tests_passed: z.boolean().nullable(),
  review_required: z.boolean(),
  auto_merge_eligible: z.boolean(),
  cpu_usage_avg: z.number().min(0).max(100).nullable(),
  memory_usage_peak: z.number().int().min(0).nullable(),
  disk_io_bytes: z.number().int().min(0).nullable(),
  network_requests: z.number().int().min(0).nullable(),
  stdout_log_path: z.string().max(1000).nullable(),
  stderr_log_path: z.string().max(1000).nullable(),
  log_summary: z.string().max(2000).nullable(),
  created_at: TimestampSchema,
  updated_at: TimestampSchema,
  competition_id: UuidSchema.nullable(),
  competition_rank: z.number().int().min(1).nullable(),
  user_feedback: z.record(z.any()).nullable(),
  bookmarked: z.boolean(),
  tags: z.array(z.string()),
});

export const AgentRunCreateSchema = z.object({
  agent_id: z.string().min(1).max(255),
  agent_type: AgentTypeSchema,
  parent_run_id: UuidSchema.optional(),
  repo_id: z.number().int().positive(),
  worktree_id: z.number().int().positive(),
  commit_sha: z.string().length(40),
  task_description: z.string().min(1).max(2000),
  task_type: z.string().min(1).max(100),
  instructions: z.record(z.any()).default({}),
  context_files: z.array(z.string()).default([]),
  tmux_session: z.string().max(255).optional(),
  tmux_window: z.number().int().min(0).optional(),
  tmux_pane: z.number().int().min(0).optional(),
  review_required: z.boolean().default(false),
  auto_merge_eligible: z.boolean().default(false),
  competition_id: UuidSchema.optional(),
  tags: z.array(z.string()).default([]),
});

export const AgentRunUpdateSchema = z.object({
  status: AgentStatusSchema.optional(),
  completed_at: TimestampSchema.optional(),
  duration_ms: z.number().int().min(0).optional(),
  exit_code: z.number().int().optional(),
  error_message: z.string().max(2000).nullable().optional(),
  artifacts: z.record(z.any()).optional(),
  evaluation_score: z.number().min(0).max(100).optional(),
  tests_passed: z.boolean().optional(),
  review_required: z.boolean().optional(),
  auto_merge_eligible: z.boolean().optional(),
  cpu_usage_avg: z.number().min(0).max(100).optional(),
  memory_usage_peak: z.number().int().min(0).optional(),
  disk_io_bytes: z.number().int().min(0).optional(),
  network_requests: z.number().int().min(0).optional(),
  stdout_log_path: z.string().max(1000).optional(),
  stderr_log_path: z.string().max(1000).optional(),
  log_summary: z.string().max(2000).optional(),
  competition_rank: z.number().int().min(1).optional(),
  user_feedback: z.record(z.any()).optional(),
  bookmarked: z.boolean().optional(),
  tags: z.array(z.string()).optional(),
});

// Agent message schemas
export const AgentMessageBaseSchema = z.object({
  id: z.number().int().positive(),
  message_id: UuidSchema,
  correlation_id: UuidSchema.nullable(),
  reply_to_id: UuidSchema.nullable(),
  run_id: UuidSchema,
  sender_agent_id: z.string().min(1).max(255),
  recipient_agent_id: z.string().max(255).nullable(),
  message_type: MessageTypeSchema,
  priority: MessagePrioritySchema,
  subject: z.string().max(500).nullable(),
  content: z.string().min(1),
  content_format: z.string().max(50).default('text'),
  metadata: z.record(z.any()).nullable(),
  attachments: z.record(z.any()).nullable(),
  broadcast: z.boolean(),
  delivered_at: TimestampSchema.nullable(),
  acknowledged_at: TimestampSchema.nullable(),
  created_at: TimestampSchema,
  expires_at: TimestampSchema.nullable(),
  processed: z.boolean(),
  processing_result: z.record(z.any()).nullable(),
  retry_count: NonNegativeIntSchema,
  max_retries: NonNegativeIntSchema,
  bus_topic: z.string().max(255).nullable(),
  bus_partition: z.number().int().min(0).nullable(),
  bus_offset: z.number().int().min(0).nullable(),
  tags: z.array(z.string()),
  size_bytes: z.number().int().min(0),
  processing_time_ms: z.number().int().min(0).nullable(),
});

export const AgentMessageCreateSchema = z.object({
  correlation_id: UuidSchema.optional(),
  reply_to_id: UuidSchema.optional(),
  run_id: UuidSchema,
  sender_agent_id: z.string().min(1).max(255),
  recipient_agent_id: z.string().max(255).optional(),
  message_type: MessageTypeSchema,
  priority: MessagePrioritySchema.default('normal'),
  subject: z.string().max(500).optional(),
  content: z.string().min(1),
  content_format: z.string().max(50).default('text'),
  metadata: z.record(z.any()).default({}),
  attachments: z.record(z.any()).default({}),
  broadcast: z.boolean().default(false),
  expires_at: TimestampSchema.optional(),
  max_retries: NonNegativeIntSchema.default(3),
  bus_topic: z.string().max(255).optional(),
  tags: z.array(z.string()).default([]),
});

// Query schemas
export const AgentRunQuerySchema = PaginationQuerySchema.extend({
  agent_id: z.string().optional(),
  agent_type: AgentTypeSchema.optional(),
  status: AgentStatusSchema.optional(),
  repo_id: z.coerce.number().int().positive().optional(),
  worktree_id: z.coerce.number().int().positive().optional(),
  task_type: z.string().optional(),
  parent_run_id: UuidSchema.optional(),
  competition_id: UuidSchema.optional(),
  bookmarked: z.coerce.boolean().optional(),
  review_required: z.coerce.boolean().optional(),
  auto_merge_eligible: z.coerce.boolean().optional(),
  tests_passed: z.coerce.boolean().optional(),
  min_evaluation_score: z.coerce.number().min(0).max(100).optional(),
  started_range: DateRangeFilterSchema.optional(),
  completed_range: DateRangeFilterSchema.optional(),
  search: SearchFilterSchema.optional(),
  tags: z.array(z.string()).optional(),
});

export const AgentMessageQuerySchema = PaginationQuerySchema.extend({
  run_id: UuidSchema.optional(),
  sender_agent_id: z.string().optional(),
  recipient_agent_id: z.string().optional(),
  message_type: MessageTypeSchema.optional(),
  priority: MessagePrioritySchema.optional(),
  correlation_id: UuidSchema.optional(),
  reply_to_id: UuidSchema.optional(),
  broadcast: z.coerce.boolean().optional(),
  processed: z.coerce.boolean().optional(),
  created_range: DateRangeFilterSchema.optional(),
  search: SearchFilterSchema.optional(),
  tags: z.array(z.string()).optional(),
});

export const AgentStatsQuerySchema = z.object({
  agent_type: AgentTypeSchema.optional(),
  repo_id: z.coerce.number().int().positive().optional(),
  worktree_id: z.coerce.number().int().positive().optional(),
  date_range: DateRangeFilterSchema.optional(),
});

// Response schemas
export const AgentStatsSchema = z.object({
  total_runs: z.number().int().min(0),
  by_status: z.record(AgentStatusSchema, z.number().int().min(0)),
  by_agent_type: z.record(AgentTypeSchema, z.number().int().min(0)),
  by_task_type: z.record(z.string(), z.number().int().min(0)),
  avg_duration_ms: z.number().min(0),
  success_rate: z.number().min(0).max(100),
  avg_evaluation_score: z.number().min(0).max(100),
  most_active_agents: z.array(z.object({
    agent_id: z.string(),
    run_count: z.number().int().min(0),
    success_rate: z.number().min(0).max(100),
    avg_score: z.number().min(0).max(100),
  })),
  recent_activity: z.array(z.object({
    run_id: UuidSchema,
    agent_id: z.string(),
    status: AgentStatusSchema,
    started_at: TimestampSchema,
    task_type: z.string(),
  })),
});

// Type exports
export type AgentRunBase = z.infer<typeof AgentRunBaseSchema>;
export type AgentRunCreate = z.infer<typeof AgentRunCreateSchema>;
export type AgentRunUpdate = z.infer<typeof AgentRunUpdateSchema>;
export type AgentRunQuery = z.infer<typeof AgentRunQuerySchema>;
export type AgentMessageBase = z.infer<typeof AgentMessageBaseSchema>;
export type AgentMessageCreate = z.infer<typeof AgentMessageCreateSchema>;
export type AgentMessageQuery = z.infer<typeof AgentMessageQuerySchema>;
export type AgentStatsQuery = z.infer<typeof AgentStatsQuerySchema>;
export type AgentStats = z.infer<typeof AgentStatsSchema>;
export type AgentType = z.infer<typeof AgentTypeSchema>;
export type AgentStatus = z.infer<typeof AgentStatusSchema>;
export type MessageType = z.infer<typeof MessageTypeSchema>;
export type MessagePriority = z.infer<typeof MessagePrioritySchema>;