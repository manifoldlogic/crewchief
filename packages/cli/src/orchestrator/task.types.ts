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
