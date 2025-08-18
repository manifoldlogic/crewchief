import React, { useMemo, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Button } from '../ui/button';
import type { PerformanceMetrics } from '../../hooks/useWebSocket';

interface PerformanceMonitoringProps {
  performance: PerformanceMetrics | null;
  websocketConnected: boolean;
  loading?: boolean;
  error?: string | null;
  onRefresh?: () => void;
}

interface MetricHistoryEntry {
  timestamp: string;
  value: number;
}

interface SimpleLineChartProps {
  data: MetricHistoryEntry[];
  height?: number;
  color?: string;
  label?: string;
  unit?: string;
}

const SimpleLineChart: React.FC<SimpleLineChartProps> = ({ 
  data, 
  height = 100, 
  color = '#3b82f6',
  label = '',
  unit = 'ms'
}) => {
  const { path, minValue, maxValue, viewBox } = useMemo(() => {
    if (data.length === 0) {
      return { 
        path: '', 
        minValue: 0, 
        maxValue: 100, 
        viewBox: `0 0 300 ${height}` 
      };
    }

    const values = data.map(d => d.value);
    const minVal = Math.min(...values);
    const maxVal = Math.max(...values);
    const range = maxVal - minVal || 1;
    
    const width = 300;
    const padding = 10;
    
    const points = data.map((point, index) => {
      const x = (index / (data.length - 1)) * (width - 2 * padding) + padding;
      const y = height - padding - ((point.value - minVal) / range) * (height - 2 * padding);
      return `${x},${y}`;
    });

    const pathData = `M ${points.join(' L ')}`;
    
    return {
      path: pathData,
      minValue: minVal,
      maxValue: maxVal,
      viewBox: `0 0 ${width} ${height}`
    };
  }, [data, height]);

  const formatValue = (value: number) => {
    if (unit === 'ms' && value > 1000) {
      return `${(value / 1000).toFixed(1)}s`;
    }
    return `${value.toFixed(1)}${unit}`;
  };

  return (
    <div className="relative">
      {label && (
        <div className="text-xs text-gray-500 dark:text-gray-400 mb-2">
          {label}: {data.length > 0 ? formatValue(data[data.length - 1].value) : 'N/A'}
        </div>
      )}
      <svg viewBox={viewBox} className="w-full" style={{ height }}>
        {/* Grid lines */}
        <defs>
          <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
            <path
              d="M 20 0 L 0 0 0 20"
              fill="none"
              stroke="currentColor"
              strokeWidth="0.5"
              className="text-gray-200 dark:text-gray-700"
            />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#grid)" />
        
        {/* Data line */}
        {path && (
          <path
            d={path}
            fill="none"
            stroke={color}
            strokeWidth="2"
            className="drop-shadow-sm"
          />
        )}
        
        {/* Data points */}
        {data.map((point, index) => {
          const x = (index / (data.length - 1)) * (300 - 20) + 10;
          const y = height - 10 - ((point.value - minValue) / (maxValue - minValue || 1)) * (height - 20);
          return (
            <circle
              key={index}
              cx={x}
              cy={y}
              r="2"
              fill={color}
              className="drop-shadow-sm"
            />
          );
        })}
      </svg>
      
      {data.length === 0 && (
        <div className="absolute inset-0 flex items-center justify-center text-gray-400 dark:text-gray-600 text-sm">
          No data available
        </div>
      )}
    </div>
  );
};

const ConnectionStatusIndicator: React.FC<{ connected: boolean }> = ({ connected }) => {
  return (
    <div className="flex items-center space-x-2">
      <div className={`w-3 h-3 rounded-full ${connected ? 'bg-green-500' : 'bg-red-500'} animate-pulse`} />
      <span className="text-sm text-gray-600 dark:text-gray-300">
        WebSocket {connected ? 'Connected' : 'Disconnected'}
      </span>
    </div>
  );
};

export const PerformanceMonitoring: React.FC<PerformanceMonitoringProps> = ({
  performance,
  websocketConnected,
  loading = false,
  error,
  onRefresh,
}) => {
  // Mock historical data for demonstration
  const [apiResponseHistory] = useState<MetricHistoryEntry[]>(() => {
    const now = Date.now();
    return Array.from({ length: 20 }, (_, i) => ({
      timestamp: new Date(now - (19 - i) * 60000).toISOString(),
      value: Math.random() * 500 + 100, // 100-600ms
    }));
  });

  const [dbQueryHistory] = useState<MetricHistoryEntry[]>(() => {
    const now = Date.now();
    return Array.from({ length: 20 }, (_, i) => ({
      timestamp: new Date(now - (19 - i) * 60000).toISOString(),
      value: Math.random() * 100 + 20, // 20-120ms
    }));
  });

  const [cpuUsageHistory] = useState<MetricHistoryEntry[]>(() => {
    const now = Date.now();
    return Array.from({ length: 20 }, (_, i) => ({
      timestamp: new Date(now - (19 - i) * 60000).toISOString(),
      value: Math.random() * 80 + 10, // 10-90%
    }));
  });

  const getResponseTimeStatus = (responseTime: number | undefined) => {
    if (!responseTime) return 'unknown';
    if (responseTime < 200) return 'excellent';
    if (responseTime < 500) return 'good';
    if (responseTime < 1000) return 'warning';
    return 'poor';
  };

  const getDbQueryStatus = (queryTime: number | undefined) => {
    if (!queryTime) return 'unknown';
    if (queryTime < 50) return 'excellent';
    if (queryTime < 100) return 'good';
    if (queryTime < 200) return 'warning';
    return 'poor';
  };

  const statusColors = {
    excellent: 'text-green-600 dark:text-green-400',
    good: 'text-blue-600 dark:text-blue-400',
    warning: 'text-yellow-600 dark:text-yellow-400',
    poor: 'text-red-600 dark:text-red-400',
    unknown: 'text-gray-600 dark:text-gray-400',
  };

  const statusLabels = {
    excellent: 'Excellent',
    good: 'Good',
    warning: 'Slow',
    poor: 'Poor',
    unknown: 'Unknown',
  };

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Performance Monitoring</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-6 animate-pulse">
            {Array.from({ length: 3 }).map((_, index) => (
              <div key={index} className="space-y-2">
                <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/3"></div>
                <div className="h-24 bg-gray-200 dark:bg-gray-700 rounded"></div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="border-red-200 dark:border-red-800">
        <CardHeader>
          <CardTitle className="text-red-600 dark:text-red-400">Performance Monitoring</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-red-600 dark:text-red-400 py-8">
            <div className="text-2xl mb-2">⚠️</div>
            <div>Error loading performance data: {error}</div>
            {onRefresh && (
              <Button variant="outline" onClick={onRefresh} className="mt-4">
                Retry
              </Button>
            )}
          </div>
        </CardContent>
      </Card>
    );
  }

  const apiResponseTime = performance?.apiResponseTime ?? apiResponseHistory[apiResponseHistory.length - 1]?.value;
  const dbQueryTime = performance?.dbQueryTime ?? dbQueryHistory[dbQueryHistory.length - 1]?.value;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Performance Monitoring</CardTitle>
          <div className="flex items-center space-x-4">
            <ConnectionStatusIndicator connected={websocketConnected} />
            {onRefresh && (
              <Button variant="ghost" size="sm" onClick={onRefresh}>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      
      <CardContent className="space-y-6">
        {/* Key Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-600 dark:text-gray-400">API Response</span>
              <span className={`text-xs font-medium ${statusColors[getResponseTimeStatus(apiResponseTime)]}`}>
                {statusLabels[getResponseTimeStatus(apiResponseTime)]}
              </span>
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {apiResponseTime ? (
                apiResponseTime > 1000 ? `${(apiResponseTime / 1000).toFixed(1)}s` : `${Math.round(apiResponseTime)}ms`
              ) : 'N/A'}
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-600 dark:text-gray-400">DB Queries</span>
              <span className={`text-xs font-medium ${statusColors[getDbQueryStatus(dbQueryTime)]}`}>
                {statusLabels[getDbQueryStatus(dbQueryTime)]}
              </span>
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {dbQueryTime ? `${Math.round(dbQueryTime)}ms` : 'N/A'}
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-600 dark:text-gray-400">WebSocket</span>
              <span className={`text-xs font-medium ${websocketConnected ? statusColors.excellent : statusColors.poor}`}>
                {websocketConnected ? 'Connected' : 'Disconnected'}
              </span>
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {websocketConnected ? 'Online' : 'Offline'}
            </div>
          </div>
        </div>

        {/* Performance Charts */}
        <div className="space-y-6">
          <div>
            <h4 className="text-lg font-medium text-gray-900 dark:text-white mb-4">Response Time Trends</h4>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg">
                <SimpleLineChart
                  data={apiResponseHistory}
                  label="API Response Time"
                  color="#3b82f6"
                  unit="ms"
                  height={120}
                />
              </div>
              
              <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg">
                <SimpleLineChart
                  data={dbQueryHistory}
                  label="Database Query Time"
                  color="#10b981"
                  unit="ms"
                  height={120}
                />
              </div>
            </div>
          </div>

          <div>
            <h4 className="text-lg font-medium text-gray-900 dark:text-white mb-4">System Resources</h4>
            <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg">
              <SimpleLineChart
                data={cpuUsageHistory}
                label="CPU Usage"
                color="#f59e0b"
                unit="%"
                height={120}
              />
            </div>
          </div>
        </div>

        {/* Real-time Status */}
        <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
          <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-3">Real-time Status</h4>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">Last updated:</span>
              <span className="text-gray-900 dark:text-white">
                {performance?.timestamp ? new Date(performance.timestamp).toLocaleTimeString() : 'Never'}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">Data points collected:</span>
              <span className="text-gray-900 dark:text-white">{apiResponseHistory.length}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">Monitoring interval:</span>
              <span className="text-gray-900 dark:text-white">1 minute</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};