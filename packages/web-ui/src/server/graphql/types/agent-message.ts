

export const agentMessageTypeDefs = `
  type AgentMessage implements Node & Timestamped {
    id: ID!
    messageId: String!
    correlationId: String
    replyToId: String
    runId: String!
    senderAgentId: String!
    recipientAgentId: String
    type: MessageType!
    priority: MessagePriority!
    subject: String
    content: String!
    contentFormat: String!
    metadata: JSON
    attachments: JSON
    broadcast: Boolean!
    deliveredAt: DateTime
    acknowledgedAt: DateTime
    createdAt: DateTime!
    expiresAt: DateTime
    processed: Boolean!
    processingResult: JSON
    retryCount: Int!
    maxRetries: Int!
    busTopic: String
    busPartition: Int
    busOffset: Int
    tags: [String!]!
    sizeBytes: Int!
    processingTimeMs: Int
    updatedAt: DateTime!
    
    # Relations
    run: Run!
    replyTo: AgentMessage
    replies: [AgentMessage!]!
    
    # Computed fields
    isExpired: Boolean!
    canRetry: Boolean!
    threadLength: Int!
    age: String!
  }

  type AgentMessageConnection {
    edges: [AgentMessageEdge!]!
    pageInfo: PageInfo!
  }

  type AgentMessageEdge {
    node: AgentMessage!
    cursor: String!
  }

  # Message thread structure
  type MessageThread {
    rootMessage: AgentMessage!
    messages: [AgentMessage!]!
    totalCount: Int!
    participantCount: Int!
    lastActivity: DateTime!
  }

  # Input types
  input AgentMessageCreateInput {
    runId: String!
    senderAgentId: String!
    recipientAgentId: String
    type: MessageType!
    priority: MessagePriority = NORMAL
    subject: String
    content: String!
    contentFormat: String = "text"
    metadata: JSON
    attachments: JSON
    broadcast: Boolean = false
    replyToId: String
    correlationId: String
    expiresAt: DateTime
    tags: [String!]
  }

  input AgentMessageFilterInput {
    type: MessageType
    priority: MessagePriority
    senderAgentId: String
    recipientAgentId: String
    runId: String
    processed: Boolean
    broadcast: Boolean
    expired: Boolean
    tags: [String!]
    search: String
    dateRange: DateRangeInput
  }

  # Response types
  type AgentMessageResponse implements Response {
    success: Boolean!
    errors: [Error!]
    message: AgentMessage
  }

  extend type Query {
    agentMessage(id: ID!): AgentMessage
    agentMessages(
      filter: AgentMessageFilterInput
      sort: SortInput
      pagination: PaginationInput
    ): AgentMessageConnection!
    messageThread(messageId: String!): MessageThread!
    agentConversation(agentId: String!, otherAgentId: String): [AgentMessage!]!
    recentMessages(limit: Int = 20, agentId: String): [AgentMessage!]!
    unprocessedMessages(agentId: String): [AgentMessage!]!
  }

  extend type Mutation {
    sendAgentMessage(input: AgentMessageCreateInput!): AgentMessageResponse!
    acknowledgeMessage(id: ID!): AgentMessageResponse!
    markMessageProcessed(id: ID!, result: JSON): AgentMessageResponse!
    retryMessage(id: ID!): AgentMessageResponse!
  }

  extend type Subscription {
    messageReceived(agentId: String!): AgentMessage!
    messageAcknowledged(messageId: String!): AgentMessage!
    messageProcessed(runId: String): AgentMessage!
    broadcastMessage: AgentMessage!
  }
`;