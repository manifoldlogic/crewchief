import React, { useEffect, useCallback } from 'react';
import { useWebSocket } from '../hooks/useWebSocket';
import { useDashboardData } from '../hooks/useDashboardData';
import { useToast } from '../components/ui/use-toast';
import { usePerformanceMonitoring, measureDashboardLoad } from '../utils/performanceMonitor';
import { useDashboardSubscriptions } from '../hooks/useSubscriptions';
import { handleSubscriptionError } from '../contexts/apollo/provider';
import {
  StatsGrid,
  ActivityFeed,
  QuickActions,
  AgentStatusCards,
  PerformanceMonitoring,
} from '../components/dashboard';

interface HealthStatus {
  status: string;
  timestamp: string;
  uptime: number;
  version: string;
  environment: string;
}

const Dashboard: React.FC = () => {
  const {
    connected: websocketConnected,
    connecting: websocketConnecting,
    error: websocketError,
    stats: realtimeStats,
    activities,
    agents,
    performance,
    refreshStats,
    clearActivities,
    filterActivities,
  } = useWebSocket({
    autoConnect: true,
    maxReconnectAttempts: 5,
    reconnectDelay: 1000,
  });

  const {
    data: dashboardData,
    loading,
    error,
    lastFetch,
    fetchDashboardData,
    refreshData,
    clearError,
    isStale,
    cleanup,
  } = useDashboardData({
    retryAttempts: 3,
    retryDelay: 1000,
    onError: (error) => {
      console.error('Dashboard data fetch error:', error);
    },
    onSuccess: (data) => {
      console.log('Dashboard data loaded successfully:', data);
    },
  });

  const { toast } = useToast();
  const { measureQuickAction, getReport } = usePerformanceMonitoring();

  // GraphQL subscriptions for real-time updates
  const {
    worktreeSubscription,
    agentSubscription,
    runSubscription,
    systemSubscription,
    allData: subscriptionData,
  } = useDashboardSubscriptions({
    onSubscriptionData: (data) => {
      console.log('📡 Dashboard subscription data received:', data);
      
      // Refresh dashboard data when updates arrive
      if (data.worktrees || data.agents || data.runs) {
        refreshData();
      }
    },
    onError: handleSubscriptionError,
  });

  // Initial data fetch with performance monitoring
  useEffect(() => {
    const loadDashboard = async () => {
      try {
        const loadMetric = await measureDashboardLoad();
        console.log('Dashboard load performance:', loadMetric);
        
        if (!loadMetric.passed) {
          toast({
            title: 'Performance Warning',
            description: `Dashboard loaded slowly (${loadMetric.duration?.toFixed(0)}ms). Check network connection.`,
            variant: 'destructive',
          });
        }
      } catch (error) {
        console.error('Dashboard load measurement failed:', error);
      }
    };

    fetchDashboardData();
    loadDashboard();
    
    // Cleanup on unmount
    return cleanup;
  }, [fetchDashboardData, cleanup, toast]);

  // Refresh data every 5 minutes if not connected to WebSocket
  useEffect(() => {
    if (!websocketConnected) {
      const interval = setInterval(() => {
        // Only refresh if data is stale
        if (isStale()) {
          refreshData();
        }
      }, 5 * 60 * 1000);
      return () => clearInterval(interval);
    }
  }, [websocketConnected, refreshData, isStale]);

  // Show connection status notifications
  useEffect(() => {
    if (websocketError) {
      toast({
        title: 'WebSocket Error',
        description: 'Real-time updates may be unavailable',
        variant: 'destructive',
      });
    }
  }, [websocketError, toast]);

  // Merge real-time stats with API data
  const mergedStats = realtimeStats || {
    totalWorktrees: dashboardData.worktreeStats?.total_worktrees || 0,
    activeAgents: dashboardData.agentStats?.by_status?.running || 0,
    indexedFiles: 12500, // Mock data - would come from maproom API
    systemHealth: dashboardData.health?.status === 'ok' ? 'healthy' : 'critical',
    apiResponseTime: dashboardData.health ? 150 : 0, // Mock calculation
    diskUsage: 45 * 1024 * 1024 * 1024, // 45GB mock
    lastUpdated: new Date().toISOString(),
  };

  const handleActionComplete = useCallback((action: string, success: boolean) => {
    if (success) {
      // Refresh relevant data
      refreshData();
      refreshStats();
      
      toast({
        title: 'Action Completed',
        description: `${action.replace('-', ' ')} completed successfully`,
      });
    }
  }, [refreshData, refreshStats, toast]);

  // Enhanced action handler with performance monitoring
  const handleQuickAction = useCallback((actionName: string, actionFn: () => Promise<void> | void) => {
    const actionTimer = measureQuickAction(actionName);
    actionTimer.start();
    
    const executeAction = async () => {
      try {
        await actionFn();
        const metric = actionTimer.end();
        
        if (metric && !metric.passed) {
          console.warn(`Quick action '${actionName}' took ${metric.duration?.toFixed(0)}ms (threshold: ${metric.threshold}ms)`);
        }
      } catch (error) {
        actionTimer.end();
        throw error;
      }
    };
    
    return executeAction();
  }, [measureQuickAction]);

  const handleAgentAction = (agentId: string, action: string, success: boolean) => {
    if (success) {
      toast({
        title: 'Agent Action',
        description: `Agent ${agentId} ${action} completed`,
      });
    }
  };

  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    
    if (hours > 0) {
      return `${hours}h ${minutes}m ${secs}s`;
    } else if (minutes > 0) {
      return `${minutes}m ${secs}s`;
    } else {
      return `${secs}s`;
    }
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto p-6" data-testid="dashboard-container">
      {/* Welcome Section */}
      <div className="bg-white dark:bg-gray-800 overflow-hidden shadow rounded-lg">
        <div className="px-4 py-5 sm:p-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center">
              <div className="flex-shrink-0">
                <div className="w-12 h-12 bg-primary-100 dark:bg-primary-900 rounded-lg flex items-center justify-center">
                  <span className="text-primary-600 dark:text-primary-300 text-2xl font-bold">🚀</span>
                </div>
              </div>
              <div className="ml-4">
                <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
                  CrewChief Dashboard
                </h2>
                <p className="text-gray-600 dark:text-gray-300">
                  Multi-agent orchestration system
                </p>
              </div>
            </div>
            
            {/* Connection Status */}
            <div className="flex items-center space-x-4">
              <div className="flex items-center space-x-2">
                <div className={`w-3 h-3 rounded-full ${websocketConnected ? 'bg-green-500' : 'bg-yellow-500'} ${websocketConnecting ? 'animate-pulse' : ''}`} />
                <span className="text-sm text-gray-600 dark:text-gray-300">
                  {websocketConnecting ? 'Connecting...' : websocketConnected ? 'Live Updates' : 'Offline Mode'}
                </span>
              </div>
              
              {dashboardData.health && (
                <div className="text-sm text-gray-500 dark:text-gray-400">
                  Uptime: {formatUptime(dashboardData.health.uptime)}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Stats Grid */}
      <StatsGrid
        stats={mergedStats}
        performance={performance}
        loading={loading}
        error={error}
      />

      {/* Quick Actions */}
      <QuickActions 
        onActionComplete={handleActionComplete}
        onPerformAction={handleQuickAction}
      />

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
        {/* Activity Feed */}
        <div className="xl:col-span-2">
          <ActivityFeed
            activities={activities}
            loading={loading}
            error={websocketError}
            onClear={clearActivities}
          />
        </div>

        {/* Performance Monitoring */}
        <div>
          <PerformanceMonitoring
            performance={performance}
            websocketConnected={websocketConnected}
            loading={loading}
            error={websocketError}
            onRefresh={() => {
              refreshData();
              refreshStats();
            }}
          />
        </div>
      </div>

      {/* Agent Status Cards */}
      <AgentStatusCards
        agents={agents}
        loading={loading}
        error={websocketError}
        onAgentAction={handleAgentAction}
      />
    </div>
  );
};

export default Dashboard;