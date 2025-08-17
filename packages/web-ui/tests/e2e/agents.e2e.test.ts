import { test, expect } from '@playwright/test';

test.describe('Agents Management', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the agents page
    await page.goto('/agents');
    await page.waitForLoadState('networkidle');
  });

  test('should display agents page', async ({ page }) => {
    // Check page title
    await expect(page.getByRole('heading', { name: 'Agents' })).toBeVisible();
    
    // Check that agents list container exists
    await expect(page.getByTestId('agents-list')).toBeVisible();
  });

  test('should show active agent runs', async ({ page }) => {
    // Mock API response for agent runs
    await page.route('**/api/agents/runs', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'run-1',
            agent_id: 'claude-code',
            task_description: 'Create testing framework',
            status: 'running',
            created_at: '2024-01-01T10:00:00Z',
            started_at: '2024-01-01T10:01:00Z',
            metadata: {
              worktree_path: '/path/to/worktree',
              tmux_session: 'test-session',
            },
          },
          {
            id: 'run-2',
            agent_id: 'cursor-gpt',
            task_description: 'Fix authentication bug',
            status: 'completed',
            created_at: '2024-01-01T09:00:00Z',
            started_at: '2024-01-01T09:01:00Z',
            completed_at: '2024-01-01T09:30:00Z',
            exit_code: 0,
          },
        ]),
      });
    });

    // Wait for agent runs to load
    await page.waitForSelector('[data-testid="agent-run"]');
    
    // Check that agent runs are displayed
    await expect(page.getByText('Create testing framework')).toBeVisible();
    await expect(page.getByText('Fix authentication bug')).toBeVisible();
    
    // Check status indicators
    await expect(page.getByText('running')).toBeVisible();
    await expect(page.getByText('completed')).toBeVisible();
  });

  test('should start a new agent run', async ({ page }) => {
    // Mock API endpoints
    await page.route('**/api/agents/available', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { id: 'claude-code', name: 'Claude Code', status: 'available' },
          { id: 'cursor-gpt', name: 'Cursor GPT', status: 'available' },
        ]),
      });
    });

    await page.route('**/api/agents/start', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'new-run-1',
          agent_id: 'claude-code',
          task_description: 'Test task',
          status: 'starting',
        }),
      });
    });

    // Click start new agent button
    const startButton = page.getByRole('button', { name: /start new agent/i });
    await startButton.click();
    
    // Fill in the form
    const agentSelect = page.getByRole('combobox', { name: /select agent/i });
    await agentSelect.selectOption('claude-code');
    
    const taskInput = page.getByRole('textbox', { name: /task description/i });
    await taskInput.fill('Test task for E2E testing');
    
    // Submit the form
    const submitButton = page.getByRole('button', { name: /start agent/i });
    await submitButton.click();
    
    // Should show success message
    await expect(page.getByText(/agent started successfully/i)).toBeVisible();
  });

  test('should stop a running agent', async ({ page }) => {
    // Mock running agent
    await page.route('**/api/agents/runs', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'run-1',
            agent_id: 'claude-code',
            task_description: 'Long running task',
            status: 'running',
            created_at: '2024-01-01T10:00:00Z',
          },
        ]),
      });
    });

    await page.route('**/api/agents/runs/run-1/stop', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });

    // Wait for agent run to appear
    await page.waitForSelector('[data-testid="agent-run"]');
    
    // Click stop button
    const stopButton = page.getByRole('button', { name: /stop/i });
    await stopButton.click();
    
    // Confirm the action
    const confirmButton = page.getByRole('button', { name: /confirm/i });
    await confirmButton.click();
    
    // Should show success message
    await expect(page.getByText(/agent stopped/i)).toBeVisible();
  });

  test('should show agent logs', async ({ page }) => {
    // Mock agent with logs
    await page.route('**/api/agents/runs', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'run-1',
            agent_id: 'claude-code',
            task_description: 'Task with logs',
            status: 'running',
          },
        ]),
      });
    });

    await page.route('**/api/agents/runs/run-1/logs', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            timestamp: '2024-01-01T10:00:00Z',
            level: 'info',
            message: 'Agent started',
          },
          {
            timestamp: '2024-01-01T10:01:00Z',
            level: 'info',
            message: 'Processing task...',
          },
        ]),
      });
    });

    // Wait for agent run to appear
    await page.waitForSelector('[data-testid="agent-run"]');
    
    // Click view logs button
    const logsButton = page.getByRole('button', { name: /view logs/i });
    await logsButton.click();
    
    // Should show logs modal/panel
    await expect(page.getByText('Agent started')).toBeVisible();
    await expect(page.getByText('Processing task...')).toBeVisible();
  });

  test('should filter agents by status', async ({ page }) => {
    // Mock agents with different statuses
    await page.route('**/api/agents/runs', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'run-1',
            agent_id: 'agent-1',
            task_description: 'Running task',
            status: 'running',
          },
          {
            id: 'run-2',
            agent_id: 'agent-2',
            task_description: 'Completed task',
            status: 'completed',
          },
          {
            id: 'run-3',
            agent_id: 'agent-3',
            task_description: 'Failed task',
            status: 'failed',
          },
        ]),
      });
    });

    // Wait for agents to load
    await page.waitForSelector('[data-testid="agent-run"]');
    
    // Apply status filter
    const statusFilter = page.getByRole('combobox', { name: /status filter/i });
    await statusFilter.selectOption('running');
    
    // Should only show running agents
    await expect(page.getByText('Running task')).toBeVisible();
    await expect(page.getByText('Completed task')).not.toBeVisible();
    await expect(page.getByText('Failed task')).not.toBeVisible();
  });

  test('should show agent performance metrics', async ({ page }) => {
    // Mock metrics API
    await page.route('**/api/agents/metrics', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          totalRuns: 25,
          successRate: 0.88,
          averageExecutionTime: 1800000, // 30 minutes in ms
          activeAgents: 3,
        }),
      });
    });

    // Wait for metrics to load
    await page.waitForSelector('[data-testid="agent-metrics"]');
    
    // Check metrics are displayed
    await expect(page.getByText('25')).toBeVisible(); // Total runs
    await expect(page.getByText('88%')).toBeVisible(); // Success rate
    await expect(page.getByText('30m')).toBeVisible(); // Avg execution time
    await expect(page.getByText('3')).toBeVisible(); // Active agents
  });

  test('should handle real-time updates', async ({ page }) => {
    let agentData = [
      {
        id: 'run-1',
        agent_id: 'claude-code',
        task_description: 'Updating task',
        status: 'running',
      },
    ];

    await page.route('**/api/agents/runs', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(agentData),
      });
    });

    // Wait for initial load
    await page.waitForSelector('[data-testid="agent-run"]');
    await expect(page.getByText('running')).toBeVisible();

    // Update the data to simulate real-time change
    agentData[0].status = 'completed';
    
    // Trigger refresh (this would normally happen via WebSocket or polling)
    await page.reload();
    await page.waitForSelector('[data-testid="agent-run"]');
    
    // Should show updated status
    await expect(page.getByText('completed')).toBeVisible();
  });

  test('should handle agent errors gracefully', async ({ page }) => {
    // Mock API error
    await page.route('**/api/agents/runs', route => {
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'Failed to fetch agent runs',
        }),
      });
    });

    // Should show error state
    await expect(page.getByText(/failed to fetch agent runs/i)).toBeVisible();
    
    // Should show retry button
    const retryButton = page.getByRole('button', { name: /retry/i });
    await expect(retryButton).toBeVisible();
  });

  test('should be accessible', async ({ page }) => {
    // Check main elements have proper roles
    await expect(page.getByRole('heading', { name: 'Agents' })).toBeVisible();
    await expect(page.getByRole('main')).toBeVisible();
    
    // Check that agent list is accessible
    const agentsList = page.getByRole('list', { name: /agents/i });
    if (await agentsList.isVisible()) {
      await expect(agentsList).toBeVisible();
    }
    
    // Check keyboard navigation
    const firstButton = page.getByRole('button').first();
    await firstButton.focus();
    await expect(firstButton).toBeFocused();
  });

  test('should support bulk operations', async ({ page }) => {
    // Mock multiple agents
    await page.route('**/api/agents/runs', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { id: 'run-1', status: 'running', task_description: 'Task 1' },
          { id: 'run-2', status: 'running', task_description: 'Task 2' },
          { id: 'run-3', status: 'running', task_description: 'Task 3' },
        ]),
      });
    });

    await page.route('**/api/agents/bulk-stop', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ stopped: 2 }),
      });
    });

    // Select multiple agents
    const checkboxes = page.getByRole('checkbox');
    await checkboxes.nth(0).check();
    await checkboxes.nth(1).check();
    
    // Use bulk action
    const bulkStopButton = page.getByRole('button', { name: /stop selected/i });
    await bulkStopButton.click();
    
    // Confirm bulk action
    const confirmButton = page.getByRole('button', { name: /confirm/i });
    await confirmButton.click();
    
    // Should show success message
    await expect(page.getByText(/2 agents stopped/i)).toBeVisible();
  });
});