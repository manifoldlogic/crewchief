

export const maproomIndexTypeDefs = `
  enum IndexStatus {
    PENDING
    INDEXING
    COMPLETED
    FAILED
    STALE
    UPDATING
  }

  type MaproomIndex implements Node & Timestamped {
    id: ID!
    worktreeId: ID!
    status: IndexStatus!
    filesIndexed: Int!
    lastUpdated: DateTime!
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Extended indexing information
    repoId: ID!
    commitSha: String!
    totalFiles: Int!
    skippedFiles: Int!
    errorFiles: Int!
    chunkCount: Int!
    totalSizeBytes: Int!
    indexSizeBytes: Int!
    indexingDurationMs: Int
    programmingLanguages: JSON!
    fileTypeBreakdown: JSON!
    errorMessages: [String!]!
    warningMessages: [String!]!
    indexingStartedAt: DateTime
    indexingCompletedAt: DateTime
    lastScanDurationMs: Int
    version: String!
    maproomVersion: String!
    
    # Relations
    worktree: Worktree!
    searchHistory: [SearchHistory!]!
    
    # Computed fields
    indexingProgress: Float
    isHealthy: Boolean!
    needsReindex: Boolean!
    timeSinceLastUpdate: String!
  }

  type MaproomIndexConnection {
    edges: [MaproomIndexEdge!]!
    pageInfo: PageInfo!
  }

  type MaproomIndexEdge {
    node: MaproomIndex!
    cursor: String!
  }

  # Search functionality
  type SearchHistory implements Node & Timestamped {
    id: ID!
    query: String!
    searchType: String!
    resultCount: Int!
    executionTimeMs: Int!
    relevanceThreshold: Float
    topResults: JSON
    performanceMetrics: JSON
    searchedAt: DateTime!
    clickedResults: [Int!]!
    saved: Boolean!
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Relations
    maproomIndex: MaproomIndex!
    worktree: Worktree!
  }

  type SearchResult {
    id: ID!
    score: Float!
    path: String!
    lineNumber: Int
    content: String!
    context: String
    language: String
    symbolType: String
    symbolName: String
    metadata: JSON
  }

  type SearchResponse {
    results: [SearchResult!]!
    totalCount: Int!
    executionTimeMs: Int!
    query: String!
    searchType: String!
    performanceMetrics: JSON
  }

  # Index statistics
  type IndexStatistics {
    totalWorktrees: Int!
    indexedWorktrees: Int!
    pendingWorktrees: Int!
    failedWorktrees: Int!
    totalFiles: Int!
    totalChunks: Int!
    totalSizeBytes: Int!
    averageIndexingTime: Float!
    languageBreakdown: [LanguageStats!]!
    recentIndexingActivity: [IndexingActivity!]!
  }

  type LanguageStats {
    language: String!
    fileCount: Int!
    chunkCount: Int!
    sizeBytes: Int!
    percentage: Float!
  }

  type IndexingActivity {
    date: DateTime!
    indexedFiles: Int!
    durationMs: Int!
    worktreeCount: Int!
  }

  # Input types
  input MaproomIndexCreateInput {
    worktreeId: ID!
    repoId: ID!
    commitSha: String!
    force: Boolean = false
  }

  input MaproomIndexUpdateInput {
    id: ID!
    status: IndexStatus
    commitSha: String
  }

  input MaproomIndexFilterInput {
    status: IndexStatus
    worktreeId: ID
    repoId: ID
    needsReindex: Boolean
    isHealthy: Boolean
    dateRange: DateRangeInput
  }

  input SearchInput {
    query: String!
    worktreeId: ID
    searchType: String = "semantic"
    limit: Int = 20
    relevanceThreshold: Float = 0.5
    includeMetadata: Boolean = true
    fileTypes: [String!]
    paths: [String!]
  }

  input SearchHistoryFilterInput {
    worktreeId: ID
    searchType: String
    saved: Boolean
    dateRange: DateRangeInput
  }

  # Response types
  type MaproomIndexResponse implements Response {
    success: Boolean!
    errors: [Error!]
    index: MaproomIndex
  }

  type MaproomIndexDeleteResponse implements Response {
    success: Boolean!
    errors: [Error!]
    deletedId: ID
  }

  extend type Query {
    maproomIndex(id: ID!): MaproomIndex
    maproomIndices(
      filter: MaproomIndexFilterInput
      sort: SortInput
      pagination: PaginationInput
    ): MaproomIndexConnection!
    maproomIndexByWorktree(worktreeId: ID!): MaproomIndex
    indexStatistics: IndexStatistics!
    
    # Search queries
    search(input: SearchInput!): SearchResponse!
    searchHistory(
      filter: SearchHistoryFilterInput
      pagination: PaginationInput
    ): [SearchHistory!]!
    popularSearches(limit: Int = 10, days: Int = 7): [SearchHistory!]!
  }

  extend type Mutation {
    createMaproomIndex(input: MaproomIndexCreateInput!): MaproomIndexResponse!
    updateMaproomIndex(input: MaproomIndexUpdateInput!): MaproomIndexResponse!
    deleteMaproomIndex(id: ID!): MaproomIndexDeleteResponse!
    reindexWorktree(worktreeId: ID!, force: Boolean = false): MaproomIndexResponse!
    markIndexStale(id: ID!): MaproomIndexResponse!
    
    # Search mutations
    saveSearch(id: ID!): SearchHistory!
    unsaveSearch(id: ID!): SearchHistory!
    deleteSearchHistory(id: ID!): Boolean!
    clearSearchHistory(worktreeId: ID): Boolean!
  }

  extend type Subscription {
    maproomIndexUpdated(id: ID): MaproomIndex!
    indexingStatusChanged: MaproomIndex!
    indexingStarted: MaproomIndex!
    indexingCompleted: MaproomIndex!
    searchPerformed: SearchHistory!
  }
`;