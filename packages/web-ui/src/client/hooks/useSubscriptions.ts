import { useSubscription, gql } from '@apollo/client';
import { useCallback, useEffect, useRef } from 'react';

// GraphQL subscription documents
const WORKTREE_UPDATED_SUBSCRIPTION = gql`
  subscription WorktreeUpdated($id: ID) {
    worktreeUpdated(id: $id) {
      id
      name
      branch
      path
      status
      updatedAt
      isClean
      isSynced
      commitsAhead
      commitsBehind
      modifiedFiles
      addedFiles
      deletedFiles
      untrackedFiles
      activeAgents
      maproomIndexStatus
      lastError
    }
  }
`;

const AGENT_STATUS_CHANGED_SUBSCRIPTION = gql`
  subscription AgentStatusChanged {
    agentStatusChanged {
      id
      name
      type
      status
      worktreeId
      updatedAt
      startedAt
      completedAt
      durationMs
      errorMessage
      evaluationScore
      testsPassed
      exitCode
    }
  }
`;

const AGENT_MESSAGE_RECEIVED_SUBSCRIPTION = gql`
  subscription AgentMessageReceived($agentId: ID, $runId: ID) {
    agentMessageReceived(agentId: $agentId, runId: $runId) {
      id
      agentId
      runId
      messageType
      content
      metadata
      timestamp
      createdAt
    }
  }
`;

const MAPROOM_INDEX_UPDATED_SUBSCRIPTION = gql`
  subscription MaproomIndexUpdated($id: ID) {
    maproomIndexUpdated(id: $id) {
      id
      worktreeId
      status
      filesIndexed
      totalFiles
      chunkCount
      lastUpdated
      indexingProgress
      errorMessages
      warningMessages
      indexingDurationMs
    }
  }
`;

const RUN_STATUS_CHANGED_SUBSCRIPTION = gql`
  subscription RunStatusChanged {
    runStatusChanged {
      id
      agentId
      status
      startedAt
      completedAt
      durationMs
      errorMessage
      exitCode
      evaluationScore
      testsPassed
      artifacts
    }
  }
`;

const FILE_CHANGED_SUBSCRIPTION = gql`
  subscription FileChanged($worktreeId: ID, $pathPattern: String) {
    fileChanged(worktreeId: $worktreeId, pathPattern: $pathPattern) {
      path
      changeType
      worktreeId
      timestamp
      size
      mimeType
    }
  }
`;

const GIT_OPERATION_PROGRESS_SUBSCRIPTION = gql`
  subscription GitOperationProgress($operationId: ID) {
    gitOperationProgress(operationId: $operationId) {
      id
      operationType
      progress
      status
      message
      worktreeId
      timestamp
    }
  }
`;

const SYSTEM_STATUS_CHANGED_SUBSCRIPTION = gql`
  subscription SystemStatusChanged {
    systemStatusChanged {
      component
      status
      message
      timestamp
      metadata
    }
  }
`;

// Hook types
interface SubscriptionOptions {
  skip?: boolean;
  onSubscriptionData?: (data: any) => void;
  onError?: (error: any) => void;
}

interface WorktreeSubscriptionOptions extends SubscriptionOptions {
  worktreeId?: string;
}

interface AgentMessageSubscriptionOptions extends SubscriptionOptions {
  agentId?: string;
  runId?: string;
}

interface FileChangeSubscriptionOptions extends SubscriptionOptions {
  worktreeId?: string;
  pathPattern?: string;
}

interface GitOperationSubscriptionOptions extends SubscriptionOptions {
  operationId?: string;
}

interface MaproomIndexSubscriptionOptions extends SubscriptionOptions {
  indexId?: string;
}

// Worktree subscriptions
export function useWorktreeUpdates(options: WorktreeSubscriptionOptions = {}) {
  const { worktreeId, skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(WORKTREE_UPDATED_SUBSCRIPTION, {
    variables: { id: worktreeId },
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('🏠 Worktree updated:', subscriptionData.data?.worktreeUpdated);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('Worktree subscription error:', error);
      onError?.(error);
    },
  });
}

// Agent subscriptions
export function useAgentStatusUpdates(options: SubscriptionOptions = {}) {
  const { skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(AGENT_STATUS_CHANGED_SUBSCRIPTION, {
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('🤖 Agent status changed:', subscriptionData.data?.agentStatusChanged);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('Agent status subscription error:', error);
      onError?.(error);
    },
  });
}

export function useAgentMessages(options: AgentMessageSubscriptionOptions = {}) {
  const { agentId, runId, skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(AGENT_MESSAGE_RECEIVED_SUBSCRIPTION, {
    variables: { agentId, runId },
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('💬 Agent message received:', subscriptionData.data?.agentMessageReceived);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('Agent message subscription error:', error);
      onError?.(error);
    },
  });
}

// Run subscriptions
export function useRunStatusUpdates(options: SubscriptionOptions = {}) {
  const { skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(RUN_STATUS_CHANGED_SUBSCRIPTION, {
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('🏃 Run status changed:', subscriptionData.data?.runStatusChanged);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('Run status subscription error:', error);
      onError?.(error);
    },
  });
}

// Maproom subscriptions
export function useMaproomIndexUpdates(options: MaproomIndexSubscriptionOptions = {}) {
  const { indexId, skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(MAPROOM_INDEX_UPDATED_SUBSCRIPTION, {
    variables: { id: indexId },
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('🗺️ Maproom index updated:', subscriptionData.data?.maproomIndexUpdated);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('Maproom index subscription error:', error);
      onError?.(error);
    },
  });
}

// File system subscriptions
export function useFileChanges(options: FileChangeSubscriptionOptions = {}) {
  const { worktreeId, pathPattern, skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(FILE_CHANGED_SUBSCRIPTION, {
    variables: { worktreeId, pathPattern },
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('📁 File changed:', subscriptionData.data?.fileChanged);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('File change subscription error:', error);
      onError?.(error);
    },
  });
}

// Git subscriptions
export function useGitOperationProgress(options: GitOperationSubscriptionOptions = {}) {
  const { operationId, skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(GIT_OPERATION_PROGRESS_SUBSCRIPTION, {
    variables: { operationId },
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('🔀 Git operation progress:', subscriptionData.data?.gitOperationProgress);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('Git operation subscription error:', error);
      onError?.(error);
    },
  });
}

// System subscriptions
export function useSystemStatusUpdates(options: SubscriptionOptions = {}) {
  const { skip = false, onSubscriptionData, onError } = options;
  
  return useSubscription(SYSTEM_STATUS_CHANGED_SUBSCRIPTION, {
    skip,
    onSubscriptionData: ({ subscriptionData }) => {
      console.log('⚙️ System status changed:', subscriptionData.data?.systemStatusChanged);
      onSubscriptionData?.(subscriptionData);
    },
    onError: (error) => {
      console.error('System status subscription error:', error);
      onError?.(error);
    },
  });
}

// Compound hook for dashboard that subscribes to multiple events
export function useDashboardSubscriptions(options: SubscriptionOptions = {}) {
  const { skip = false, onSubscriptionData, onError } = options;
  
  // Keep track of subscription data
  const dataRef = useRef<any>({});
  
  const handleSubscriptionData = useCallback((type: string) => (data: any) => {
    dataRef.current[type] = data;
    onSubscriptionData?.(dataRef.current);
  }, [onSubscriptionData]);
  
  // Subscribe to multiple events
  const worktreeSubscription = useWorktreeUpdates({
    skip,
    onSubscriptionData: handleSubscriptionData('worktrees'),
    onError,
  });
  
  const agentSubscription = useAgentStatusUpdates({
    skip,
    onSubscriptionData: handleSubscriptionData('agents'),
    onError,
  });
  
  const runSubscription = useRunStatusUpdates({
    skip,
    onSubscriptionData: handleSubscriptionData('runs'),
    onError,
  });
  
  const systemSubscription = useSystemStatusUpdates({
    skip,
    onSubscriptionData: handleSubscriptionData('system'),
    onError,
  });
  
  return {
    worktreeSubscription,
    agentSubscription,
    runSubscription,
    systemSubscription,
    allData: dataRef.current,
  };
}

// Connection status hook
export function useSubscriptionConnectionStatus() {
  const connectionStatus = useRef<'connecting' | 'connected' | 'disconnected'>('connecting');
  
  useEffect(() => {
    // Listen for WebSocket connection events
    const handleOnline = () => {
      connectionStatus.current = 'connected';
    };
    
    const handleOffline = () => {
      connectionStatus.current = 'disconnected';
    };
    
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    
    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);
  
  return connectionStatus.current;
}