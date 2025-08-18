export const baseTypeDefs = `
  # Custom scalars
  scalar DateTime
  scalar JSON

  # Common enums
  enum AgentStatus {
    PENDING
    RUNNING
    COMPLETED
    FAILED
    CANCELLED
    TIMEOUT
  }

  enum AgentType {
    CLAUDE
    GEMINI
    MOCK
    CUSTOM
  }

  enum MessageType {
    COMMAND
    RESPONSE
    NOTIFICATION
    ERROR
    LOG
    STATUS_UPDATE
    FILE_CHANGE
    GIT_EVENT
    SYSTEM_EVENT
  }

  enum MessagePriority {
    LOW
    NORMAL
    HIGH
    CRITICAL
  }

  enum WorktreeState {
    ACTIVE
    STALE
    MERGING
    ARCHIVED
    ERROR
  }

  enum EventType {
    WORKTREE_CREATED
    WORKTREE_UPDATED
    WORKTREE_DELETED
    AGENT_STARTED
    AGENT_COMPLETED
    AGENT_FAILED
    RUN_CREATED
    RUN_UPDATED
    RUN_COMPLETED
    MAPROOM_INDEXED
    MAPROOM_UPDATED
    CONFIG_CHANGED
    SYSTEM_EVENT
  }

  # Common interfaces
  interface Node {
    id: ID!
  }

  interface Timestamped {
    createdAt: DateTime!
    updatedAt: DateTime!
  }

  # Pagination types
  type PageInfo {
    hasNextPage: Boolean!
    hasPreviousPage: Boolean!
    startCursor: String
    endCursor: String
    totalCount: Int!
    pageSize: Int!
    page: Int!
  }

  # Common input types
  input PaginationInput {
    limit: Int = 50
    offset: Int = 0
    page: Int = 1
    pageSize: Int = 50
  }

  input SortInput {
    field: String!
    direction: SortDirection = ASC
  }

  enum SortDirection {
    ASC
    DESC
  }

  # Error types
  type Error {
    message: String!
    code: String
    field: String
    details: JSON
  }

  type ValidationError {
    field: String!
    message: String!
    code: String!
  }

  # Common response wrapper
  interface Response {
    success: Boolean!
    errors: [Error!]
  }
`;