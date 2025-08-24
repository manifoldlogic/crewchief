import { z } from 'zod'

export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  worktreeBasePath: z.string().default('.crewchief/worktrees'),
})

// Removed: orchestrator fields are not used in current iTerm-based orchestration

// Removed: agents command config is not used by current orchestration

// Removed: tmux config no longer supported

export const ITermSchema = z.object({
  sessionName: z.string().default('crewchief'),
})

export const TerminalSchema = z.object({
  backend: z.enum(['iterm', 'auto']).default('iterm'),
  iterm: ITermSchema.optional(),
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
