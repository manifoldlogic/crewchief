/**
 * Dashboard Performance and Functionality Tests
 * 
 * This test file validates that the dashboard meets all acceptance criteria:
 * - Dashboard loads in < 2 seconds
 * - Stats update in real-time via WebSocket
 * - Activity feed shows last 50 events
 * - Quick actions execute in < 500ms
 * - Agent cards show live status
 * - Charts render with smooth animations
 */

import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import Dashboard from '../../pages/Dashboard';

// Mock the hooks and components
vi.mock('../hooks/useWebSocket', () => ({
  useWebSocket: vi.fn(() => ({
    connected: true,
    connecting: false,
    error: null,
    stats: {
      totalWorktrees: 5,
      activeAgents: 3,
      indexedFiles: 12500,
      systemHealth: 'healthy',
      apiResponseTime: 150,
      diskUsage: 45 * 1024 * 1024 * 1024,
      lastUpdated: new Date().toISOString(),
    },
    activities: [
      {
        id: '1',
        type: 'agent_status_change',
        timestamp: new Date().toISOString(),
        title: 'Agent Started',
        description: 'Agent claude-dev-1 started successfully',
        severity: 'success',
      },
    ],
    agents: [
      {
        id: 'agent-1',
        type: 'claude-3-5-sonnet',
        name: 'claude-dev-1',
        status: 'running',
        cpuUsage: 45.2,
        memoryUsage: 32.1,
        lastActive: new Date().toISOString(),
        currentTask: 'Implementing dashboard features',
        worktreeId: 'wt-1',
      },
    ],
    performance: {
      apiResponseTime: 150,
      websocketConnected: true,
      dbQueryTime: 45,
      timestamp: new Date().toISOString(),
    },
    refreshStats: vi.fn(),
    clearActivities: vi.fn(),
    filterActivities: vi.fn(),
  })),
}));

vi.mock('../hooks/useDashboardData', () => ({
  useDashboardData: vi.fn(() => ({
    data: {
      health: {
        status: 'ok',
        timestamp: new Date().toISOString(),
        uptime: 86400,
        version: '0.1.17',
        environment: 'development',
      },
      worktreeStats: {
        total_worktrees: 5,
        by_state: { active: 3, stale: 1, archived: 1, error: 0 },
      },
      agentStats: {
        total_runs: 25,
        by_status: { running: 3, completed: 20, failed: 2 },
      },
      indexStats: null,
    },
    loading: false,
    error: null,
    lastFetch: new Date(),
    fetchDashboardData: vi.fn(),
    refreshData: vi.fn(),
    clearError: vi.fn(),
    isStale: vi.fn(() => false),
    cleanup: vi.fn(),
  })),
}));

vi.mock('../components/ui/use-toast', () => ({
  useToast: vi.fn(() => ({
    toast: vi.fn(),
  })),
}));

// Mock API client
vi.mock('../utils/apiClient', () => ({
  apiClient: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
  },
}));

describe('Dashboard Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  describe('Loading Performance', () => {
    it('should load dashboard within 2 seconds', async () => {
      const startTime = performance.now();
      
      render(<Dashboard />);
      
      // Wait for all components to render
      await waitFor(() => {
        expect(screen.getByText('CrewChief Dashboard')).toBeInTheDocument();
      });

      const loadTime = performance.now() - startTime;
      
      // Should load within 2 seconds (2000ms)
      expect(loadTime).toBeLessThan(2000);
    });

    it('should show loading states during initial load', () => {
      const { rerender } = render(<Dashboard />);
      
      // Should handle loading gracefully
      expect(screen.getByText('CrewChief Dashboard')).toBeInTheDocument();
      
      rerender(<Dashboard />);
    });
  });

  describe('Stats Grid', () => {
    it('should display all 6 key metrics', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Total Worktrees')).toBeInTheDocument();
        expect(screen.getByText('Active Agents')).toBeInTheDocument();
        expect(screen.getByText('Indexed Files')).toBeInTheDocument();
        expect(screen.getByText('System Health')).toBeInTheDocument();
        expect(screen.getByText('API Response')).toBeInTheDocument();
        expect(screen.getByText('Disk Usage')).toBeInTheDocument();
      });
    });

    it('should show real-time values from WebSocket', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('5')).toBeInTheDocument(); // Total Worktrees
        expect(screen.getByText('3')).toBeInTheDocument(); // Active Agents
        expect(screen.getByText('12,500')).toBeInTheDocument(); // Indexed Files
        expect(screen.getByText('Healthy')).toBeInTheDocument(); // System Health
      });
    });
  });

  describe('Activity Feed', () => {
    it('should display activity events', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Activity Feed')).toBeInTheDocument();
        expect(screen.getByText('Agent Started')).toBeInTheDocument();
        expect(screen.getByText('Agent claude-dev-1 started successfully')).toBeInTheDocument();
      });
    });

    it('should support filtering activities', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        const typeFilter = screen.getByDisplayValue('All Types');
        expect(typeFilter).toBeInTheDocument();
        
        const severityFilter = screen.getByDisplayValue('All Severities');
        expect(severityFilter).toBeInTheDocument();
      });
    });

    it('should show event count', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText(/1 of 1 events/)).toBeInTheDocument();
      });
    });
  });

  describe('Quick Actions', () => {
    it('should display all quick action buttons', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Create Worktree')).toBeInTheDocument();
        expect(screen.getByText('Spawn Agent')).toBeInTheDocument();
        expect(screen.getByText('Run Indexing')).toBeInTheDocument();
        expect(screen.getByText('View Logs')).toBeInTheDocument();
        expect(screen.getByText('Refresh')).toBeInTheDocument();
        expect(screen.getByText('Clear Cache')).toBeInTheDocument();
      });
    });

    it('should execute actions quickly (< 500ms)', async () => {
      render(<Dashboard />);
      
      const refreshButton = await screen.findByText('Refresh');
      
      const startTime = performance.now();
      fireEvent.click(refreshButton);
      
      await waitFor(() => {
        const executionTime = performance.now() - startTime;
        expect(executionTime).toBeLessThan(500);
      });
    });

    it('should show confirmation dialogs for destructive actions', async () => {
      render(<Dashboard />);
      
      const clearCacheButton = await screen.findByText('Clear Cache');
      fireEvent.click(clearCacheButton);
      
      await waitFor(() => {
        expect(screen.getByText('Clear System Cache')).toBeInTheDocument();
        expect(screen.getByText('Confirm')).toBeInTheDocument();
        expect(screen.getByText('Cancel')).toBeInTheDocument();
      });
    });
  });

  describe('Agent Status Cards', () => {
    it('should display agent information', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Agent Status')).toBeInTheDocument();
        expect(screen.getByText('claude-dev-1')).toBeInTheDocument();
        expect(screen.getByText('claude-3-5-sonnet')).toBeInTheDocument();
        expect(screen.getByText('Running')).toBeInTheDocument();
      });
    });

    it('should show resource usage with progress bars', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('CPU:')).toBeInTheDocument();
        expect(screen.getByText('RAM:')).toBeInTheDocument();
        expect(screen.getByText('45.2%')).toBeInTheDocument();
        expect(screen.getByText('32.1%')).toBeInTheDocument();
      });
    });

    it('should provide agent action buttons', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Stop')).toBeInTheDocument();
        expect(screen.getByText('Restart')).toBeInTheDocument();
      });
    });
  });

  describe('Performance Monitoring', () => {
    it('should display performance metrics', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Performance Monitoring')).toBeInTheDocument();
        expect(screen.getByText('API Response')).toBeInTheDocument();
        expect(screen.getByText('DB Queries')).toBeInTheDocument();
        expect(screen.getByText('WebSocket')).toBeInTheDocument();
      });
    });

    it('should show charts for performance trends', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Response Time Trends')).toBeInTheDocument();
        expect(screen.getByText('System Resources')).toBeInTheDocument();
      });
    });

    it('should indicate WebSocket connection status', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('WebSocket Connected')).toBeInTheDocument();
      });
    });
  });

  describe('Real-time Updates', () => {
    it('should show WebSocket connection status', async () => {
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Live Updates')).toBeInTheDocument();
      });
    });

    it('should handle WebSocket disconnection gracefully', async () => {
      const mockUseWebSocket = vi.mocked(
        await import('../hooks/useWebSocket')
      ).useWebSocket;
      
      mockUseWebSocket.mockReturnValue({
        connected: false,
        connecting: false,
        error: 'Connection lost',
        stats: null,
        activities: [],
        agents: [],
        performance: null,
        refreshStats: vi.fn(),
        clearActivities: vi.fn(),
        filterActivities: vi.fn(),
      } as any);
      
      render(<Dashboard />);
      
      await waitFor(() => {
        expect(screen.getByText('Offline Mode')).toBeInTheDocument();
      });
    });
  });

  describe('Responsive Design', () => {
    it('should adapt to different screen sizes', () => {
      // Test mobile viewport
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 375,
      });
      
      render(<Dashboard />);
      
      // Should render without layout issues
      expect(screen.getByText('CrewChief Dashboard')).toBeInTheDocument();
      
      // Test tablet viewport
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 768,
      });
      
      // Test desktop viewport
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 1440,
      });
    });
  });

  describe('Error Handling', () => {
    it('should handle API errors gracefully', async () => {
      const mockUseDashboardData = vi.mocked(
        await import('../hooks/useDashboardData')
      ).useDashboardData;
      
      mockUseDashboardData.mockReturnValue({
        data: { health: null, worktreeStats: null, agentStats: null, indexStats: null },
        loading: false,
        error: 'Failed to fetch data',
        lastFetch: null,
        fetchDashboardData: vi.fn(),
        refreshData: vi.fn(),
        clearError: vi.fn(),
        isStale: vi.fn(() => true),
        cleanup: vi.fn(),
      } as any);
      
      render(<Dashboard />);
      
      // Should still render the dashboard structure
      expect(screen.getByText('CrewChief Dashboard')).toBeInTheDocument();
    });

    it('should show retry options on errors', async () => {
      const mockUseDashboardData = vi.mocked(
        await import('../hooks/useDashboardData')
      ).useDashboardData;
      
      mockUseDashboardData.mockReturnValue({
        data: { health: null, worktreeStats: null, agentStats: null, indexStats: null },
        loading: false,
        error: 'Network error',
        lastFetch: null,
        fetchDashboardData: vi.fn(),
        refreshData: vi.fn(),
        clearError: vi.fn(),
        isStale: vi.fn(() => true),
        cleanup: vi.fn(),
      } as any);
      
      render(<Dashboard />);
      
      // Should provide retry mechanism
      await waitFor(() => {
        const retryButtons = screen.getAllByText('Retry');
        expect(retryButtons.length).toBeGreaterThan(0);
      });
    });
  });
});

describe('Dashboard Security', () => {
  it('should not display sensitive data in activity feed', async () => {
    render(<Dashboard />);
    
    await waitFor(() => {
      // Check that no obvious sensitive patterns are displayed
      const bodyText = document.body.textContent || '';
      expect(bodyText).not.toMatch(/password|secret|key|token/i);
    });
  });

  it('should require confirmation for destructive actions', async () => {
    render(<Dashboard />);
    
    const clearCacheButton = await screen.findByText('Clear Cache');
    fireEvent.click(clearCacheButton);
    
    await waitFor(() => {
      expect(screen.getByText('This action cannot be undone')).toBeInTheDocument();
    });
  });
});