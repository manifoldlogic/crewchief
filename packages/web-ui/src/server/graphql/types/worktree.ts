

export const worktreeTypeDefs = `
  type Worktree implements Node & Timestamped {
    id: ID!
    name: String!
    branch: String!
    path: String!
    status: WorktreeState!
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Extended fields from worktree_status table
    repoId: ID!
    currentBranch: String!
    upstreamBranch: String
    isClean: Boolean!
    isSynced: Boolean!
    headCommitSha: String!
    headCommitMessage: String
    headCommitAuthor: String
    headCommitDate: DateTime
    commitsAhead: Int!
    commitsBehind: Int!
    modifiedFiles: Int!
    addedFiles: Int!
    deletedFiles: Int!
    untrackedFiles: Int!
    stagedFiles: Int!
    fileChanges: JSON
    totalFiles: Int!
    totalSizeBytes: Int!
    programmingLanguages: JSON
    activeAgents: JSON
    tmuxSessions: [String!]!
    diskUsageBytes: Int!
    lastBuildStatus: String
    lastBuildTime: DateTime
    testStatus: String
    testCoverage: Float
    maproomIndexedAt: DateTime
    maproomIndexStatus: String
    chunkCount: Int!
    lastScanAt: DateTime
    scanDurationMs: Int
    cacheVersion: Int!
    lastError: String
    errorCount: Int!
    lastAccessedAt: DateTime
    pinned: Boolean!
    tags: [String!]!
    notes: String
    
    # Relations
    agents: [Agent!]!
    runs: [Run!]!
    maproomIndex: MaproomIndex
  }

  type WorktreeConnection {
    edges: [WorktreeEdge!]!
    pageInfo: PageInfo!
  }

  type WorktreeEdge {
    node: Worktree!
    cursor: String!
  }

  # Input types
  input WorktreeCreateInput {
    name: String!
    branch: String!
    path: String!
    repoId: ID!
    notes: String
    tags: [String!]
  }

  input WorktreeUpdateInput {
    id: ID!
    name: String
    branch: String
    path: String
    status: WorktreeState
    pinned: Boolean
    notes: String
    tags: [String!]
  }

  input WorktreeFilterInput {
    status: WorktreeState
    branch: String
    repoId: ID
    isClean: Boolean
    isSynced: Boolean
    pinned: Boolean
    tags: [String!]
    search: String
  }

  # Response types
  type WorktreeResponse implements Response {
    success: Boolean!
    errors: [Error!]
    worktree: Worktree
  }

  type WorktreeDeleteResponse implements Response {
    success: Boolean!
    errors: [Error!]
    deletedId: ID
  }

  extend type Query {
    worktree(id: ID!): Worktree
    worktrees(
      filter: WorktreeFilterInput
      sort: SortInput
      pagination: PaginationInput
    ): WorktreeConnection!
    worktreeByPath(path: String!): Worktree
    worktreeByBranch(branch: String!, repoId: ID!): Worktree
  }

  extend type Mutation {
    createWorktree(input: WorktreeCreateInput!): WorktreeResponse!
    updateWorktree(input: WorktreeUpdateInput!): WorktreeResponse!
    deleteWorktree(id: ID!): WorktreeDeleteResponse!
    refreshWorktreeStatus(id: ID!): WorktreeResponse!
    toggleWorktreePin(id: ID!): WorktreeResponse!
  }

  extend type Subscription {
    worktreeUpdated(id: ID): Worktree!
    worktreeStatusChanged: Worktree!
  }
`;