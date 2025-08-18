# GraphQL Subscriptions for CrewChief Web UI

This document explains how to use the GraphQL subscriptions feature implemented in TICKET-021.

## Overview

GraphQL subscriptions provide real-time data updates via WebSocket connections. The implementation includes:

- **Server-side**: Apollo Server with WebSocket transport on port 3456
- **Client-side**: Apollo Client with subscription support  
- **PubSub**: Redis-backed event broadcasting with in-memory fallback
- **Authentication**: JWT-based auth with per-user filtering
- **Performance**: Support for 100+ concurrent connections

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   React Client  │    │  Apollo Server  │    │   Redis PubSub  │
│                 │    │                 │    │                 │
│ useSubscription │◄──►│ GraphQL WS      │◄──►│ Event Broadcast │
│ Apollo Client   │    │ Subscriptions   │    │ Channel Routing │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Available Subscriptions

### Worktree Subscriptions
```graphql
# Watch specific worktree updates
subscription($id: ID) {
  worktreeUpdated(id: $id) {
    id
    name
    status
    updatedAt
    isClean
    isSynced
    commitsAhead
    commitsBehind
  }
}

# Watch all worktree status changes
subscription {
  worktreeStatusChanged {
    id
    name
    status
    updatedAt
  }
}
```

### Agent Subscriptions
```graphql
# Watch agent status changes
subscription {
  agentStatusChanged {
    id
    name
    status
    worktreeId
    updatedAt
    evaluationScore
  }
}

# Watch agent messages
subscription($agentId: ID, $runId: ID) {
  agentMessageReceived(agentId: $agentId, runId: $runId) {
    id
    agentId
    runId
    messageType
    content
    timestamp
  }
}
```

### Maproom Subscriptions
```graphql
# Watch indexing progress
subscription($id: ID) {
  maproomIndexUpdated(id: $id) {
    id
    worktreeId
    status
    filesIndexed
    totalFiles
    indexingProgress
  }
}
```

### File System & Git Subscriptions
```graphql
# Watch file changes
subscription($worktreeId: ID, $pathPattern: String) {
  fileChanged(worktreeId: $worktreeId, pathPattern: $pathPattern) {
    path
    changeType
    worktreeId
    timestamp
  }
}

# Watch git operations
subscription($operationId: ID) {
  gitOperationProgress(operationId: $operationId) {
    id
    operationType
    progress
    status
    message
  }
}
```

## Client Usage

### Setup Apollo Client (already configured)

The Apollo Client is pre-configured with subscription support in `src/client/lib/apollo-client.ts`.

### Using Subscription Hooks

```tsx
import React from 'react';
import { useWorktreeUpdates, useAgentStatusUpdates } from '../hooks/useSubscriptions';

function Dashboard() {
  // Subscribe to worktree updates
  useWorktreeUpdates({
    worktreeId: 'specific-worktree-id', // optional filter
    onSubscriptionData: (data) => {
      console.log('Worktree updated:', data.subscriptionData.data.worktreeUpdated);
      // Update local state or refresh queries
    },
    onError: (error) => {
      console.error('Subscription error:', error);
    },
  });

  // Subscribe to agent status changes
  useAgentStatusUpdates({
    onSubscriptionData: (data) => {
      console.log('Agent status changed:', data.subscriptionData.data.agentStatusChanged);
    },
  });

  return <div>Dashboard with real-time updates</div>;
}
```

### Dashboard with Multiple Subscriptions

```tsx
import { useDashboardSubscriptions } from '../hooks/useSubscriptions';

function Dashboard() {
  const {
    worktreeSubscription,
    agentSubscription,
    runSubscription,
    systemSubscription,
    allData,
  } = useDashboardSubscriptions({
    onSubscriptionData: (data) => {
      // Handle multiple subscription updates
      if (data.worktrees) {
        // Refresh worktree data
      }
      if (data.agents) {
        // Refresh agent data
      }
    },
  });

  return <div>Real-time dashboard</div>;
}
```

## Server Usage

### Broadcasting Events

```typescript
import { publishEvent, SUBSCRIPTION_EVENTS } from '../server/graphql/subscriptions';

// Broadcast worktree update
await publishEvent(
  SUBSCRIPTION_EVENTS.WORKTREE_STATUS_CHANGED,
  {
    worktree: {
      id: 'worktree-123',
      name: 'my-worktree',
      status: 'ACTIVE',
      updatedAt: new Date().toISOString(),
    },
  },
  {
    userId: 'user-123', // Optional: user-specific filtering
    workspaceId: 'workspace-456', // Optional: workspace-specific filtering
  }
);

// Broadcast agent status change
await publishEvent(
  SUBSCRIPTION_EVENTS.AGENT_STATUS_CHANGED,
  {
    agent: {
      id: 'agent-789',
      status: 'RUNNING',
      worktreeId: 'worktree-123',
    },
  }
);
```

### Adding Custom Subscriptions

1. **Add subscription type to schema**:
```graphql
extend type Subscription {
  myCustomEvent: MyCustomType!
}
```

2. **Create subscription resolver**:
```typescript
export const myCustomSubscription = createSubscriptionResolver(
  SUBSCRIPTION_EVENTS.MY_CUSTOM_EVENT,
  'read:custom', // required permission
  (payload, variables, context) => {
    // Optional filtering logic
    return true;
  }
);
```

3. **Add to subscription resolvers**:
```typescript
export const subscriptionResolvers = {
  // ... existing resolvers
  myCustomEvent: myCustomSubscription,
};
```

## Authentication & Security

### JWT Authentication
Subscriptions require JWT authentication via WebSocket connection params:

```javascript
const client = createClient({
  url: 'ws://localhost:3456/graphql',
  connectionParams: {
    authorization: 'Bearer YOUR_JWT_TOKEN',
  },
});
```

### Permission-based Filtering
Each subscription checks user permissions:

- `read:worktrees` - Worktree subscriptions
- `read:agents` - Agent subscriptions  
- `read:maproom` - Maproom subscriptions
- `read:filesystem` - File system subscriptions
- `read:git` - Git operation subscriptions
- `read:system` - System status subscriptions

### User-specific Filtering
Events can be filtered by user ID automatically:

```typescript
// Only users with matching ID will receive this event
await publishEvent(
  SUBSCRIPTION_EVENTS.AGENT_STATUS_CHANGED,
  { agent: agentData },
  { userId: 'user-123' }
);
```

## Performance & Scalability

### Connection Limits
- **Max concurrent connections**: 1000
- **Connection timeout**: 30 seconds
- **Heartbeat interval**: 15 seconds
- **Message size limit**: 1MB

### Rate Limiting
- **Window**: 15 minutes
- **Max requests**: 1000 per IP
- **Subscription creation**: Rate limited per connection

### Redis PubSub
Production deployments use Redis for scalable event broadcasting:

```env
REDIS_URL=redis://localhost:6379
```

Falls back to in-memory PubSub if Redis is unavailable.

## Testing

### Manual Testing
Use the provided test script:

```bash
# Start the server
pnpm dev:server

# In another terminal, run the test
node test-subscriptions.js
```

### Integration Tests
Run the comprehensive test suite:

```bash
pnpm test:integration
```

### Client Testing
Test React components with subscription mocks:

```tsx
import { MockedProvider } from '@apollo/client/testing';

const mocks = [
  {
    request: {
      query: WORKTREE_UPDATED_SUBSCRIPTION,
      variables: { id: 'test-worktree' },
    },
    result: {
      data: {
        worktreeUpdated: {
          id: 'test-worktree',
          name: 'Test Worktree',
          status: 'ACTIVE',
        },
      },
    },
  },
];

function TestComponent() {
  return (
    <MockedProvider mocks={mocks}>
      <YourComponent />
    </MockedProvider>
  );
}
```

## Troubleshooting

### Common Issues

1. **Connection Failed**
   - Check server is running on port 3456
   - Verify WebSocket endpoint: `ws://localhost:3456/graphql`
   - Check authentication token

2. **No Subscription Data**
   - Ensure events are being published server-side
   - Check user permissions
   - Verify subscription filters

3. **Performance Issues**
   - Monitor connection count
   - Check Redis connection if using
   - Review subscription filtering logic

### Debug Mode
Enable debug logging:

```bash
DEBUG=graphql:* pnpm dev:server
```

### Monitoring
Check subscription health:

```bash
curl http://localhost:3456/api/health
```

## Configuration

### Environment Variables
```env
# GraphQL subscriptions
GRAPHQL_SUBSCRIPTIONS_ENABLED=true
GRAPHQL_WS_PATH=/graphql

# Redis (optional)
REDIS_URL=redis://localhost:6379

# Security
JWT_SECRET=your-secret-key
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173

# Performance
MAX_SUBSCRIPTION_CONNECTIONS=1000
SUBSCRIPTION_HEARTBEAT_INTERVAL=15000
SUBSCRIPTION_CONNECTION_TIMEOUT=30000
```

### Development vs Production

**Development**:
- Uses in-memory PubSub
- Allows all origins
- Detailed error messages
- GraphQL playground enabled

**Production**:
- Requires Redis PubSub
- Strict CORS policy
- Sanitized error messages
- Introspection disabled

## Next Steps

This implementation provides the foundation for real-time updates in CrewChief. Future enhancements could include:

1. **Subscription Metrics**: Track usage and performance
2. **Advanced Filtering**: Complex query-based filtering
3. **Subscription Batching**: Group related updates
4. **Offline Support**: Queue events for reconnection
5. **Custom PubSub**: Alternative message brokers

The subscription system is now ready for use across all CrewChief components!