

export const agentTypeDefs = `
  type Agent implements Node & Timestamped {
    id: ID!
    name: String!
    type: AgentType!
    status: AgentStatus!
    worktreeId: ID!
    config: JSON!
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Extended fields from agent_runs table
    agentId: String!
    runId: String!
    parentRunId: String
    repoId: ID!
    commitSha: String
    taskDescription: String
    taskType: String
    instructions: JSON
    contextFiles: [String!]!
    startedAt: DateTime
    completedAt: DateTime
    durationMs: Int
    tmuxSession: String
    tmuxWindow: Int
    tmuxPane: Int
    exitCode: Int
    errorMessage: String
    artifacts: JSON
    evaluationScore: Float
    testsPassed: Boolean
    reviewRequired: Boolean
    autoMergeEligible: Boolean
    cpuUsageAvg: Float
    memoryUsagePeak: Int
    diskIoBytes: Int
    networkRequests: Int
    stdoutLogPath: String
    stderrLogPath: String
    logSummary: String
    competitionId: String
    competitionRank: Int
    userFeedback: JSON
    bookmarked: Boolean!
    tags: [String!]!
    
    # Relations
    worktree: Worktree!
    runs: [Run!]!
    messages: [AgentMessage!]!
    currentRun: Run
    parentRun: Agent
    childRuns: [Agent!]!
  }

  type AgentConnection {
    edges: [AgentEdge!]!
    pageInfo: PageInfo!
  }

  type AgentEdge {
    node: Agent!
    cursor: String!
  }

  # Performance metrics
  type AgentPerformanceMetrics {
    averageRunTime: Float!
    successRate: Float!
    totalRuns: Int!
    averageEvaluationScore: Float
    cpuUsageAverage: Float
    memoryUsageAverage: Float
    networkRequestsAverage: Float
  }

  # Input types
  input AgentCreateInput {
    name: String!
    type: AgentType!
    worktreeId: ID!
    config: JSON!
    taskDescription: String
    taskType: String
    instructions: JSON
    contextFiles: [String!]
    tags: [String!]
  }

  input AgentUpdateInput {
    id: ID!
    name: String
    config: JSON
    status: AgentStatus
    taskDescription: String
    instructions: JSON
    contextFiles: [String!]
    bookmarked: Boolean
    tags: [String!]
  }

  input AgentFilterInput {
    type: AgentType
    status: AgentStatus
    worktreeId: ID
    repoId: ID
    bookmarked: Boolean
    tags: [String!]
    search: String
    competitionId: String
    dateRange: DateRangeInput
  }

  input DateRangeInput {
    from: DateTime
    to: DateTime
  }

  # Response types
  type AgentResponse implements Response {
    success: Boolean!
    errors: [Error!]
    agent: Agent
  }

  type AgentDeleteResponse implements Response {
    success: Boolean!
    errors: [Error!]
    deletedId: ID
  }

  extend type Query {
    agent(id: ID!): Agent
    agents(
      filter: AgentFilterInput
      sort: SortInput
      pagination: PaginationInput
    ): AgentConnection!
    agentByRunId(runId: String!): Agent
    agentPerformanceMetrics(
      agentId: ID
      type: AgentType
      dateRange: DateRangeInput
    ): AgentPerformanceMetrics!
    activeAgents: [Agent!]!
  }

  extend type Mutation {
    createAgent(input: AgentCreateInput!): AgentResponse!
    updateAgent(input: AgentUpdateInput!): AgentResponse!
    deleteAgent(id: ID!): AgentDeleteResponse!
    startAgent(id: ID!, taskDescription: String): AgentResponse!
    stopAgent(id: ID!): AgentResponse!
    toggleAgentBookmark(id: ID!): AgentResponse!
  }

  extend type Subscription {
    agentUpdated(id: ID): Agent!
    agentStatusChanged: Agent!
    agentStarted: Agent!
    agentCompleted: Agent!
  }
`;