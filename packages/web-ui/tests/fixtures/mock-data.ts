/**
 * Mock data fixtures for tests
 */

export const mockAgentRun = {
  id: 'test-agent-run-1',
  agent_id: 'claude-code',
  task_description: 'Create a comprehensive testing framework',
  status: 'running',
  created_at: '2024-01-01T00:00:00Z',
  started_at: '2024-01-01T00:01:00Z',
  completed_at: null,
  error_message: null,
  exit_code: null,
  metadata: {
    worktree_path: '/path/to/worktree',
    tmux_session: 'test-session',
    tmux_pane: 'test-pane',
  },
};

export const mockAgentMessage = {
  id: 'test-message-1',
  run_id: 'test-agent-run-1',
  role: 'user',
  content: 'Please create unit tests for the service layer',
  timestamp: '2024-01-01T00:02:00Z',
  metadata: {
    source: 'user',
    priority: 'normal',
  },
};

export const mockWorktreeStatus = {
  id: 'test-worktree-1',
  path: '/path/to/worktree',
  branch: 'feature/testing-framework',
  status: 'active',
  agent_id: 'claude-code',
  created_at: '2024-01-01T00:00:00Z',
  last_activity: '2024-01-01T00:05:00Z',
  git_status: {
    modified: ['package.json', 'src/test.ts'],
    added: ['tests/unit/service.test.ts'],
    deleted: [],
    untracked: ['tests/fixtures/'],
  },
};

export const mockWebSession = {
  id: 'test-session-1',
  user_id: 'test-user',
  expires_at: '2024-01-02T00:00:00Z',
  created_at: '2024-01-01T00:00:00Z',
  last_accessed: '2024-01-01T00:10:00Z',
  metadata: {
    user_agent: 'Mozilla/5.0 (Test Browser)',
    ip_address: '127.0.0.1',
  },
};

export const mockSearchHistory = {
  id: 'test-search-1',
  session_id: 'test-session-1',
  query: 'testing framework setup',
  results_count: 15,
  timestamp: '2024-01-01T00:05:00Z',
  metadata: {
    search_type: 'semantic',
    filters: ['typescript', 'vitest'],
  },
};

export const mockUIPreferences = {
  id: 'test-preferences-1',
  session_id: 'test-session-1',
  theme: 'dark',
  sidebar_collapsed: false,
  auto_refresh_interval: 5000,
  notifications_enabled: true,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:05:00Z',
};

/**
 * Factory functions for creating test data
 */
export const createMockAgentRun = (overrides: Partial<typeof mockAgentRun> = {}) => ({
  ...mockAgentRun,
  ...overrides,
});

export const createMockAgentMessage = (overrides: Partial<typeof mockAgentMessage> = {}) => ({
  ...mockAgentMessage,
  ...overrides,
});

export const createMockWorktreeStatus = (overrides: Partial<typeof mockWorktreeStatus> = {}) => ({
  ...mockWorktreeStatus,
  ...overrides,
});

export const createMockWebSession = (overrides: Partial<typeof mockWebSession> = {}) => ({
  ...mockWebSession,
  ...overrides,
});

export const createMockSearchHistory = (overrides: Partial<typeof mockSearchHistory> = {}) => ({
  ...mockSearchHistory,
  ...overrides,
});

export const createMockUIPreferences = (overrides: Partial<typeof mockUIPreferences> = {}) => ({
  ...mockUIPreferences,
  ...overrides,
});

/**
 * Arrays of mock data for list scenarios
 */
export const mockAgentRuns = [
  createMockAgentRun({ id: 'run-1', status: 'completed' }),
  createMockAgentRun({ id: 'run-2', status: 'running' }),
  createMockAgentRun({ id: 'run-3', status: 'failed', error_message: 'Build failed' }),
];

export const mockAgentMessages = [
  createMockAgentMessage({ id: 'msg-1', role: 'user', content: 'Start the task' }),
  createMockAgentMessage({ id: 'msg-2', role: 'assistant', content: 'I\'ll begin working on this task' }),
  createMockAgentMessage({ id: 'msg-3', role: 'user', content: 'Please provide status updates' }),
];

export const mockWorktreeStatuses = [
  createMockWorktreeStatus({ id: 'wt-1', status: 'active', branch: 'main' }),
  createMockWorktreeStatus({ id: 'wt-2', status: 'inactive', branch: 'feature/tests' }),
  createMockWorktreeStatus({ id: 'wt-3', status: 'active', branch: 'hotfix/bug-123' }),
];