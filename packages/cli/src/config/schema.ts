import { z } from 'zod'

export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  worktreeBasePath: z.string().default('.crewchief/worktrees'),
})

export const OrchestratorSchema = z.object({
  model: z.string().default('claude-opus-4-1'),
  maxConcurrentAgents: z.number().int().positive().default(5),
  defaultTimeout: z
    .number()
    .int()
    .positive()
    .default(30 * 60 * 1000),
})

export const ClaudeSchema = z.object({
  command: z.string().default('claude'),
  defaultArgs: z.array(z.string()).default(['--model', 'claude-3-opus']),
  agentsDir: z.string().default('.claude/agents/'),
  commandsDir: z.string().default('.claude/commands/'),
})

export const GeminiSchema = z.object({
  command: z.string().default('gemini'),
  defaultArgs: z.array(z.string()).default(['--model', 'gemini-pro']),
  agentsDir: z.string().default('.gemini/agents/'),
})

export const AgentsSchema = z.object({
  claude: ClaudeSchema,
  gemini: GeminiSchema,
})

export const TmuxSchema = z.object({
  sessionName: z.string().default('crewchief'),
  orchestratorPaneSize: z.number().int().min(10).max(90).default(40),
  agentPaneArrangement: z.enum(['tiled', 'vertical', 'horizontal']).default('tiled'),
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
  autoStartOpsdeck: z.boolean().default(false),
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
  orchestrator: OrchestratorSchema,
  agents: AgentsSchema,
  tmux: TmuxSchema,
  evaluation: EvaluationSchema,
  launch: LaunchSchema.optional(),
  defaults: DefaultsSchema.optional(),
  worktree: WorktreeSchema.optional(),
})

export type CrewChiefConfig = z.infer<typeof ConfigSchema>
