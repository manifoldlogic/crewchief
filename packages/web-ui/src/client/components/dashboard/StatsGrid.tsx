import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import type { DashboardStats, PerformanceMetrics } from '../../hooks/useWebSocket';

interface StatsGridProps {
  stats: DashboardStats | null;
  performance: PerformanceMetrics | null;
  loading?: boolean;
  error?: string | null;
}

interface StatCardProps {
  title: string;
  value: string | number;
  icon: React.ReactNode;
  trend?: {
    value: number;
    direction: 'up' | 'down' | 'neutral';
  };
  status?: 'healthy' | 'warning' | 'critical';
  loading?: boolean;
}

const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  icon,
  trend,
  status,
  loading = false,
}) => {
  const statusColors = {
    healthy: 'text-green-600 bg-green-100 dark:text-green-400 dark:bg-green-900/20',
    warning: 'text-yellow-600 bg-yellow-100 dark:text-yellow-400 dark:bg-yellow-900/20',
    critical: 'text-red-600 bg-red-100 dark:text-red-400 dark:bg-red-900/20',
  };

  const trendColors = {
    up: 'text-green-600 dark:text-green-400',
    down: 'text-red-600 dark:text-red-400',
    neutral: 'text-gray-600 dark:text-gray-400',
  };

  const trendIcon = {
    up: '↗',
    down: '↘',
    neutral: '→',
  };

  if (loading) {
    return (
      <Card className="animate-pulse">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">
            <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-20"></div>
          </CardTitle>
          <div className="h-4 w-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </CardHeader>
        <CardContent>
          <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-16 mb-2"></div>
          <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-12"></div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="transition-all duration-200 hover:shadow-md">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
          {title}
        </CardTitle>
        <div className={`p-2 rounded-full ${status ? statusColors[status] : 'text-gray-400 bg-gray-100 dark:bg-gray-800'}`}>
          {icon}
        </div>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold text-gray-900 dark:text-white mb-1">
          {value}
        </div>
        {trend && (
          <div className={`text-xs flex items-center ${trendColors[trend.direction]}`}>
            <span className="mr-1">{trendIcon[trend.direction]}</span>
            <span>{Math.abs(trend.value)}%</span>
          </div>
        )}
      </CardContent>
    </Card>
  );
};

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
};

const formatResponseTime = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
};

export const StatsGrid: React.FC<StatsGridProps> = ({
  stats,
  performance,
  loading = false,
  error,
}) => {
  if (error) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4">
        {Array.from({ length: 6 }).map((_, index) => (
          <Card key={index} className="border-red-200 dark:border-red-800">
            <CardContent className="p-6">
              <div className="text-center text-red-600 dark:text-red-400">
                <div className="text-2xl mb-2">⚠️</div>
                <div className="text-sm">Error loading stats</div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  const statCards = [
    {
      title: 'Total Worktrees',
      value: stats?.totalWorktrees ?? '—',
      icon: (
        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2z" />
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 1v6m8-6v6" />
        </svg>
      ),
      trend: stats ? { value: 5.2, direction: 'up' as const } : undefined,
    },
    {
      title: 'Active Agents',
      value: stats?.activeAgents ?? '—',
      icon: (
        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
      ),
      status: stats ? (stats.activeAgents > 0 ? 'healthy' : 'warning') : undefined,
      trend: stats ? { value: 12.3, direction: 'up' as const } : undefined,
    },
    {
      title: 'Indexed Files',
      value: stats?.indexedFiles ? stats.indexedFiles.toLocaleString() : '—',
      icon: (
        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
      ),
      trend: stats ? { value: 8.7, direction: 'up' as const } : undefined,
    },
    {
      title: 'System Health',
      value: stats?.systemHealth ? stats.systemHealth.charAt(0).toUpperCase() + stats.systemHealth.slice(1) : '—',
      icon: (
        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
        </svg>
      ),
      status: stats?.systemHealth,
    },
    {
      title: 'API Response',
      value: performance?.apiResponseTime ? formatResponseTime(performance.apiResponseTime) : stats?.apiResponseTime ? formatResponseTime(stats.apiResponseTime) : '—',
      icon: (
        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      ),
      status: (() => {
        const responseTime = performance?.apiResponseTime ?? stats?.apiResponseTime;
        if (!responseTime) return undefined;
        if (responseTime < 200) return 'healthy';
        if (responseTime < 500) return 'warning';
        return 'critical';
      })(),
      trend: performance ? { value: 2.1, direction: 'down' as const } : undefined,
    },
    {
      title: 'Disk Usage',
      value: stats?.diskUsage ? formatBytes(stats.diskUsage) : '—',
      icon: (
        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
        </svg>
      ),
      status: (() => {
        if (!stats?.diskUsage) return undefined;
        const gb = stats.diskUsage / (1024 * 1024 * 1024);
        if (gb < 50) return 'healthy';
        if (gb < 100) return 'warning';
        return 'critical';
      })(),
      trend: stats ? { value: 15.4, direction: 'up' as const } : undefined,
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4">
      {statCards.map((card, index) => (
        <StatCard
          key={index}
          title={card.title}
          value={card.value}
          icon={card.icon}
          trend={card.trend}
          status={card.status}
          loading={loading}
        />
      ))}
    </div>
  );
};