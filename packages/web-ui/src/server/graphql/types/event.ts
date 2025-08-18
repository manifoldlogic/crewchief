

export const eventTypeDefs = `
  enum EventSeverity {
    DEBUG
    INFO
    WARNING
    ERROR
    CRITICAL
  }

  enum EventSource {
    SYSTEM
    AGENT
    WORKTREE
    MAPROOM
    USER
    API
    WEBHOOK
    SCHEDULER
    DATABASE
    FILESYSTEM
  }

  type Event implements Node & Timestamped {
    id: ID!
    type: EventType!
    entityId: ID
    data: JSON!
    timestamp: DateTime!
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Extended event fields
    source: EventSource!
    severity: EventSeverity!
    message: String!
    description: String
    entityType: String
    userId: String
    sessionId: String
    ipAddress: String
    userAgent: String
    correlationId: String
    traceId: String
    spanId: String
    parentEventId: ID
    rootEventId: ID
    sequence: Int!
    version: String!
    schema: String!
    checksum: String!
    
    # Metadata and context
    metadata: JSON!
    context: JSON!
    tags: [String!]!
    fingerprint: String!
    
    # Processing information
    processed: Boolean!
    processedAt: DateTime
    processingDurationMs: Int
    retryCount: Int!
    maxRetries: Int!
    nextRetryAt: DateTime
    errorMessage: String
    
    # Relations
    parentEvent: Event
    childEvents: [Event!]!
    relatedEntity: EventEntity
    
    # Computed fields
    age: String!
    isRecent: Boolean!
    needsAttention: Boolean!
    canRetry: Boolean!
  }

  # Union type for entity references
  union EventEntity = Worktree | Agent | Run | MaproomIndex | Configuration

  type EventConnection {
    edges: [EventEdge!]!
    pageInfo: PageInfo!
  }

  type EventEdge {
    node: Event!
    cursor: String!
  }

  # Event aggregation and analytics
  type EventStatistics {
    totalEvents: Int!
    eventsByType: [EventTypeCount!]!
    eventsBySeverity: [EventSeverityCount!]!
    eventsBySource: [EventSourceCount!]!
    recentActivity: [EventActivity!]!
    errorRate: Float!
    processingRate: Float!
    averageProcessingTime: Float!
  }

  type EventTypeCount {
    type: EventType!
    count: Int!
    percentage: Float!
  }

  type EventSeverityCount {
    severity: EventSeverity!
    count: Int!
    percentage: Float!
  }

  type EventSourceCount {
    source: EventSource!
    count: Int!
    percentage: Float!
  }

  type EventActivity {
    hour: DateTime!
    count: Int!
    errorCount: Int!
    warningCount: Int!
  }

  # Event stream for real-time monitoring
  type EventStream {
    events: [Event!]!
    cursor: String!
    hasMore: Boolean!
    totalCount: Int!
  }

  # Alert configuration
  type EventAlert {
    id: ID!
    name: String!
    description: String!
    conditions: JSON!
    actions: JSON!
    enabled: Boolean!
    cooldownMs: Int!
    lastTriggered: DateTime
    triggerCount: Int!
    createdAt: DateTime!
    updatedAt: DateTime!
  }

  # Input types
  input EventCreateInput {
    type: EventType!
    entityId: ID
    entityType: String
    data: JSON!
    source: EventSource!
    severity: EventSeverity = INFO
    message: String!
    description: String
    userId: String
    sessionId: String
    correlationId: String
    traceId: String
    parentEventId: ID
    metadata: JSON
    context: JSON
    tags: [String!]
  }

  input EventUpdateInput {
    id: ID!
    processed: Boolean
    errorMessage: String
    metadata: JSON
    tags: [String!]
  }

  input EventFilterInput {
    types: [EventType!]
    sources: [EventSource!]
    severities: [EventSeverity!]
    entityId: ID
    entityType: String
    userId: String
    sessionId: String
    correlationId: String
    traceId: String
    processed: Boolean
    needsAttention: Boolean
    tags: [String!]
    search: String
    dateRange: DateRangeInput
  }

  input EventStreamInput {
    filter: EventFilterInput
    limit: Int = 50
    cursor: String
    includeProcessed: Boolean = true
    realTime: Boolean = false
  }

  input EventAlertInput {
    name: String!
    description: String!
    conditions: JSON!
    actions: JSON!
    enabled: Boolean = true
    cooldownMs: Int = 300000
  }

  # Response types
  type EventResponse implements Response {
    success: Boolean!
    errors: [Error!]
    event: Event
  }

  type EventDeleteResponse implements Response {
    success: Boolean!
    errors: [Error!]
    deletedId: ID
  }

  type EventBulkDeleteResponse implements Response {
    success: Boolean!
    errors: [Error!]
    deletedCount: Int!
  }

  extend type Query {
    event(id: ID!): Event
    events(
      filter: EventFilterInput
      sort: SortInput
      pagination: PaginationInput
    ): EventConnection!
    eventsByEntity(entityId: ID!, entityType: String!): [Event!]!
    eventsByCorrelation(correlationId: String!): [Event!]!
    eventsByTrace(traceId: String!): [Event!]!
    eventThread(rootEventId: ID!): [Event!]!
    
    # Streaming and real-time
    eventStream(input: EventStreamInput!): EventStream!
    recentEvents(limit: Int = 20, severities: [EventSeverity!]): [Event!]!
    unprocessedEvents(limit: Int = 100): [Event!]!
    errorEvents(limit: Int = 50, hours: Int = 24): [Event!]!
    
    # Analytics
    eventStatistics(
      dateRange: DateRangeInput
      filter: EventFilterInput
    ): EventStatistics!
    eventTimeline(
      entityId: ID
      entityType: String
      hours: Int = 24
    ): [EventActivity!]!
    
    # Alerts
    eventAlerts: [EventAlert!]!
    eventAlert(id: ID!): EventAlert
  }

  extend type Mutation {
    createEvent(input: EventCreateInput!): EventResponse!
    updateEvent(input: EventUpdateInput!): EventResponse!
    deleteEvent(id: ID!): EventDeleteResponse!
    bulkDeleteEvents(filter: EventFilterInput!): EventBulkDeleteResponse!
    
    # Processing
    markEventProcessed(id: ID!, processingResult: JSON): EventResponse!
    retryEventProcessing(id: ID!): EventResponse!
    bulkMarkProcessed(ids: [ID!]!): [Event!]!
    
    # Maintenance
    cleanupOldEvents(olderThan: DateTime!, keepCritical: Boolean = true): Int!
    archiveEvents(filter: EventFilterInput!): Int!
    
    # Alerts
    createEventAlert(input: EventAlertInput!): EventAlert!
    updateEventAlert(id: ID!, input: EventAlertInput!): EventAlert!
    deleteEventAlert(id: ID!): Boolean!
    enableEventAlert(id: ID!): EventAlert!
    disableEventAlert(id: ID!): EventAlert!
  }

  extend type Subscription {
    eventCreated(filter: EventFilterInput): Event!
    eventUpdated(id: ID): Event!
    eventProcessed: Event!
    eventsStream(filter: EventFilterInput): Event!
    eventAlertTriggered: EventAlert!
    systemEvents: Event!
    errorEvents: Event!
    userEvents(userId: String!): Event!
    entityEvents(entityId: ID!, entityType: String!): Event!
  }
`;