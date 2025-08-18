

export const runTypeDefs = `
  type Run implements Node & Timestamped {
    id: ID!
    agentId: ID!
    status: AgentStatus!
    startedAt: DateTime
    completedAt: DateTime
    result: JSON
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Extended fields
    runId: String!
    parentRunId: String
    repoId: ID!
    worktreeId: ID!
    commitSha: String
    taskDescription: String
    taskType: String
    instructions: JSON
    contextFiles: [String!]!
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
    agent: Agent!
    worktree: Worktree!
    parentRun: Run
    childRuns: [Run!]!
    messages: [AgentMessage!]!
    
    # Computed fields
    duration: String
    isRunning: Boolean!
    isCompleted: Boolean!
    isFailed: Boolean!
    performanceGrade: String
  }

  type RunConnection {
    edges: [RunEdge!]!
    pageInfo: PageInfo!
  }

  type RunEdge {
    node: Run!
    cursor: String!
  }

  # Statistics and analytics
  type RunStatistics {
    totalRuns: Int!
    successfulRuns: Int!
    failedRuns: Int!
    averageRunTime: Float!
    averageEvaluationScore: Float
    totalDuration: Int!
    runsByStatus: [StatusCount!]!
    runsByType: [TypeCount!]!
    runsByAgent: [AgentCount!]!
    performanceTrend: [PerformanceDataPoint!]!
  }

  type StatusCount {
    status: AgentStatus!
    count: Int!
  }

  type TypeCount {
    type: String!
    count: Int!
  }

  type AgentCount {
    agentId: ID!
    agentName: String!
    count: Int!
  }

  type PerformanceDataPoint {
    date: DateTime!
    averageScore: Float
    runCount: Int!
    successRate: Float!
  }

  # Input types
  input RunCreateInput {
    agentId: ID!
    taskDescription: String!
    taskType: String
    instructions: JSON
    contextFiles: [String!]
    parentRunId: String
    tags: [String!]
  }

  input RunUpdateInput {
    id: ID!
    status: AgentStatus
    result: JSON
    errorMessage: String
    evaluationScore: Float
    testsPassed: Boolean
    reviewRequired: Boolean
    userFeedback: JSON
    bookmarked: Boolean
    tags: [String!]
  }

  input RunFilterInput {
    status: AgentStatus
    agentId: ID
    worktreeId: ID
    repoId: ID
    taskType: String
    bookmarked: Boolean
    testsPassed: Boolean
    competitionId: String
    tags: [String!]
    search: String
    dateRange: DateRangeInput
    evaluationScoreRange: ScoreRangeInput
  }

  input ScoreRangeInput {
    min: Float
    max: Float
  }

  # Response types
  type RunResponse implements Response {
    success: Boolean!
    errors: [Error!]
    run: Run
  }

  type RunDeleteResponse implements Response {
    success: Boolean!
    errors: [Error!]
    deletedId: ID
  }

  extend type Query {
    run(id: ID!): Run
    runs(
      filter: RunFilterInput
      sort: SortInput
      pagination: PaginationInput
    ): RunConnection!
    runByRunId(runId: String!): Run
    recentRuns(limit: Int = 10, agentId: ID): [Run!]!
    runStatistics(
      dateRange: DateRangeInput
      agentId: ID
      worktreeId: ID
    ): RunStatistics!
    competitionRuns(competitionId: String!): [Run!]!
  }

  extend type Mutation {
    createRun(input: RunCreateInput!): RunResponse!
    updateRun(input: RunUpdateInput!): RunResponse!
    deleteRun(id: ID!): RunDeleteResponse!
    cancelRun(id: ID!): RunResponse!
    retryRun(id: ID!): RunResponse!
    toggleRunBookmark(id: ID!): RunResponse!
    evaluateRun(id: ID!, score: Float!, feedback: JSON): RunResponse!
  }

  extend type Subscription {
    runUpdated(id: ID): Run!
    runStatusChanged: Run!
    runStarted: Run!
    runCompleted: Run!
    competitionRunUpdated(competitionId: String!): Run!
  }
`;