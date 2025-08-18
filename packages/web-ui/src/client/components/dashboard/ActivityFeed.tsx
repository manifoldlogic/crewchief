import React, { useState, useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Button } from '../ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '../ui/dialog';
import type { ActivityEvent } from '../../hooks/useWebSocket';

interface ActivityFeedProps {
  activities: ActivityEvent[];
  loading?: boolean;
  error?: string | null;
  onClear?: () => void;
}

const ActivityTypeIcon: React.FC<{ type: ActivityEvent['type'] }> = ({ type }) => {
  const icons = {
    worktree_update: (
      <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2z" />
      </svg>
    ),
    agent_status_change: (
      <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
    ),
    run_progress: (
      <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
      </svg>
    ),
    maproom_indexing_status: (
      <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
    ),
    config_change: (
      <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ),
    system_update: (
      <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    ),
  };

  return icons[type] || icons.system_update;
};

const SeverityBadge: React.FC<{ severity: ActivityEvent['severity'] }> = ({ severity }) => {
  const colors = {
    info: 'bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400',
    success: 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400',
    warning: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400',
    error: 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400',
  };

  return (
    <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${colors[severity]}`}>
      {severity}
    </span>
  );
};

const ActivityItem: React.FC<{ activity: ActivityEvent; index: number }> = ({ activity, index }) => {
  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMinutes = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMinutes / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMinutes < 1) return 'Just now';
    if (diffMinutes < 60) return `${diffMinutes}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  const severityIconColors = {
    info: 'text-blue-500 bg-blue-100 dark:bg-blue-900/20',
    success: 'text-green-500 bg-green-100 dark:bg-green-900/20',
    warning: 'text-yellow-500 bg-yellow-100 dark:bg-yellow-900/20',
    error: 'text-red-500 bg-red-100 dark:bg-red-900/20',
  };

  return (
    <Dialog>
      <DialogTrigger asChild>
        <div className="flex items-start space-x-3 p-3 hover:bg-gray-50 dark:hover:bg-gray-800/50 rounded-lg cursor-pointer transition-colors">
          <div className={`flex-shrink-0 p-2 rounded-full ${severityIconColors[activity.severity]}`}>
            <ActivityTypeIcon type={activity.type} />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                {activity.title}
              </p>
              <div className="flex items-center space-x-2">
                <SeverityBadge severity={activity.severity} />
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  {formatTimestamp(activity.timestamp)}
                </span>
              </div>
            </div>
            <p className="text-sm text-gray-600 dark:text-gray-300 truncate">
              {activity.description}
            </p>
          </div>
        </div>
      </DialogTrigger>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center space-x-2">
            <div className={`p-2 rounded-full ${severityIconColors[activity.severity]}`}>
              <ActivityTypeIcon type={activity.type} />
            </div>
            <span>{activity.title}</span>
            <SeverityBadge severity={activity.severity} />
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div>
            <h4 className="font-medium text-gray-900 dark:text-white mb-2">Description</h4>
            <p className="text-gray-600 dark:text-gray-300">{activity.description}</p>
          </div>
          <div>
            <h4 className="font-medium text-gray-900 dark:text-white mb-2">Details</h4>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="font-medium">Type:</span> {activity.type.replace(/_/g, ' ')}
              </div>
              <div>
                <span className="font-medium">Timestamp:</span> {new Date(activity.timestamp).toLocaleString()}
              </div>
              <div>
                <span className="font-medium">ID:</span> {activity.id}
              </div>
              <div>
                <span className="font-medium">Severity:</span> {activity.severity}
              </div>
            </div>
          </div>
          {activity.metadata && Object.keys(activity.metadata).length > 0 && (
            <div>
              <h4 className="font-medium text-gray-900 dark:text-white mb-2">Metadata</h4>
              <pre className="bg-gray-100 dark:bg-gray-800 p-3 rounded-lg text-xs overflow-auto max-h-40">
                {JSON.stringify(activity.metadata, null, 2)}
              </pre>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
};

export const ActivityFeed: React.FC<ActivityFeedProps> = ({
  activities,
  loading = false,
  error,
  onClear,
}) => {
  const [filterType, setFilterType] = useState<string>('all');
  const [filterSeverity, setFilterSeverity] = useState<string>('all');

  const filteredActivities = useMemo(() => {
    return activities.filter(activity => {
      if (filterType !== 'all' && activity.type !== filterType) return false;
      if (filterSeverity !== 'all' && activity.severity !== filterSeverity) return false;
      return true;
    });
  }, [activities, filterType, filterSeverity]);

  const activityTypes = useMemo(() => {
    const types = new Set(activities.map(a => a.type));
    return Array.from(types);
  }, [activities]);

  const severityTypes = ['info', 'success', 'warning', 'error'] as const;

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Activity Feed</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {Array.from({ length: 5 }).map((_, index) => (
              <div key={index} className="flex items-start space-x-3 animate-pulse">
                <div className="w-8 h-8 bg-gray-200 dark:bg-gray-700 rounded-full"></div>
                <div className="flex-1 space-y-2">
                  <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
                  <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
                </div>
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
          <CardTitle className="text-red-600 dark:text-red-400">Activity Feed</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-red-600 dark:text-red-400 py-8">
            <div className="text-2xl mb-2">⚠️</div>
            <div>Error loading activity feed: {error}</div>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Activity Feed</CardTitle>
          <div className="flex items-center space-x-2">
            <span className="text-sm text-gray-500 dark:text-gray-400">
              {filteredActivities.length} of {activities.length} events
            </span>
            {onClear && activities.length > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={onClear}
                className="text-xs"
              >
                Clear All
              </Button>
            )}
          </div>
        </div>
        
        {/* Filters */}
        <div className="flex flex-wrap gap-2 mt-4">
          <select
            value={filterType}
            onChange={(e) => setFilterType(e.target.value)}
            className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300"
          >
            <option value="all">All Types</option>
            {activityTypes.map(type => (
              <option key={type} value={type}>
                {type.replace(/_/g, ' ')}
              </option>
            ))}
          </select>
          
          <select
            value={filterSeverity}
            onChange={(e) => setFilterSeverity(e.target.value)}
            className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300"
          >
            <option value="all">All Severities</option>
            {severityTypes.map(severity => (
              <option key={severity} value={severity}>
                {severity}
              </option>
            ))}
          </select>
        </div>
      </CardHeader>
      
      <CardContent className="max-h-96 overflow-y-auto">
        {filteredActivities.length === 0 ? (
          <div className="text-center text-gray-500 dark:text-gray-400 py-8">
            <div className="text-4xl mb-2">📭</div>
            <div>
              {activities.length === 0 ? 'No activity events yet' : 'No events match the current filters'}
            </div>
          </div>
        ) : (
          <div className="space-y-1">
            {filteredActivities.map((activity, index) => (
              <ActivityItem key={activity.id} activity={activity} index={index} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
};