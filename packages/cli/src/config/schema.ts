import { z } from 'zod'

export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  /**
   * Base path for worktrees. Supports:
   * - Tilde expansion: ~/worktrees → /home/user/worktrees
   * - Repository placeholder: <repo-name> → actual repo name
   * - Absolute paths: /custom/path
   * - Relative paths: .crewchief/worktrees (legacy)
   *
   * @default '~/.crewchief/worktrees/<repo-name>' (changed from '.crewchief/worktrees' in v1.x)
   */
  worktreeBasePath: z.string().default('~/.crewchief/worktrees/<repo-name>'),
  /**
   * Custom path to the maproom binary.
   * If not specified, uses default discovery (PATH, bundled binaries).
   */
  maproomBinaryPath: z.string().optional(),
})

// Removed: orchestrator fields are not used in current iTerm-based orchestration

// Removed: agents command config is not used by current orchestration

export const ITermSchema = z.object({
  sessionName: z.string().default('crewchief'),
})

export const TmuxSchema = z.object({
  sessionName: z.string().default('crewchief'),
})

export const TerminalSchema = z.object({
  backend: z.enum(['iterm', 'tmux', 'headless', 'auto']).default('auto'),
  maxConcurrentAgents: z
    .number()
    .int()
    .min(1)
    .max(1000)
    .default(20)
    .describe('Maximum number of concurrent headless agents (prevents resource exhaustion)'),
  iterm: ITermSchema.optional(),
  tmux: TmuxSchema.optional(),
})

export const EvaluationSchema = z.object({
  autoMergeThreshold: z.number().min(0).max(1).default(0.95),
  requireTestsPass: z.boolean().default(true),
  requireReview: z.boolean().default(false),
  qualityChecks: z
    .array(
      z.object({
        type: z.enum(['tests', 'linting', 'build', 'custom']).default('custom'),
        command: z.string(),
        successCriteria: z.string().optional(),
      }),
    )
    .optional(),
})

export const LaunchSchema = z.object({
  autoRunDefaultAgents: z.boolean().default(false),
  askToUpdateLlmGuides: z.boolean().default(true),
})

export const DefaultsSchema = z.object({
  rootAgents: z
    .array(
      z.object({
        id: z.string(),
        platform: z.string().optional(),
      }),
    )
    .default([]),
})

export const WorktreeSchema = z.object({
  copyIgnoredFiles: z.array(z.string()).optional(),
  copyFromPath: z.string().default('.'),
  overwriteStrategy: z.enum(['skip', 'overwrite', 'backup']).default('skip'),
  /**
   * Automatically trigger maproom indexing when creating worktrees.
   *
   * When enabled, new worktrees are immediately searchable but creation is slower (5-30s).
   * When disabled (default), worktree creation is fast (<1s) but requires manual scanning.
   *
   * @default false
   * @example
   * // Enable auto-scan for immediate searchability
   * export default {
   *   worktree: {
   *     autoScanOnWorktreeUse: true,
   *   },
   * }
   */
  autoScanOnWorktreeUse: z.boolean().default(false),
})

export const ConfigSchema = z.object({
  repository: RepositorySchema,
  // removed orchestrator
  // removed agents
  terminal: TerminalSchema.optional(),
  evaluation: EvaluationSchema,
  launch: LaunchSchema.optional(),
  defaults: DefaultsSchema.optional(),
  worktree: WorktreeSchema.optional(),
})

export type CrewChiefConfig = z.infer<typeof ConfigSchema>
