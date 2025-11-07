export interface AcceptanceCriteria {
  description: string
}

export interface Task {
  id: string
  description: string
  requirements: string[]
  acceptanceCriteria: AcceptanceCriteria[]
  competitionMode?: {
    enabled: boolean
    agentCount: number
    evaluationStrategy: 'automatic' | 'manual' | 'hybrid'
  }
}

export interface TaskAssignment {
  taskId: string
  agentId: string // agent type id
  worktreeId: string // path for now
  startTime: string
  deadline?: string
  status: 'assigned' | 'in-progress' | 'complete' | 'failed'
  runId?: string
}

/**
 * Search-specific task for agent competitions
 * Used to test tool description variants
 */
export interface SearchTask extends Task {
  /** Target files/functions/classes the agent should find */
  targets: string[]

  /** Validation function to check if agent succeeded */
  validate?: (result: any) => boolean

  /** Additional context provided to the agent */
  context?: string
}
