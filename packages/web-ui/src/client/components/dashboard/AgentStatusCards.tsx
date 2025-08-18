import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Button } from '../ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '../ui/dialog';
import { useToast } from '../ui/use-toast';
import { apiClient } from '../../utils/apiClient';
import type { AgentStatus } from '../../hooks/useWebSocket';

interface AgentStatusCardsProps {
  agents: AgentStatus[];
  loading?: boolean;
  error?: string | null;
  onAgentAction?: (agentId: string, action: string, success: boolean) => void;
}

interface AgentCardProps {
  agent: AgentStatus;
  onAction?: (action: string) => void;
  loading?: boolean;
}

const StatusBadge: React.FC<{ status: AgentStatus['status'] }> = ({ status }) => {
  const statusConfig = {
    idle: {
      color: 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200',
      icon: '⏸️',
    },
    running: {
      color: 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400',
      icon: '▶️',
    },
    error: {
      color: 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400',
      icon: '❌',
    },
    stopped: {
      color: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400',
      icon: '⏹️',
    },
  };

  const config = statusConfig[status];

  return (
    <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${config.color}`}>
      <span className="mr-1">{config.icon}</span>
      {status.charAt(0).toUpperCase() + status.slice(1)}
    </span>
  );
};

const ResourceUsageMeter: React.FC<{ 
  label: string; 
  value: number; 
  max?: number; 
  unit?: string;
  color?: 'blue' | 'green' | 'yellow' | 'red';
}> = ({ 
  label, 
  value, 
  max = 100, 
  unit = '%',
  color = 'blue'
}) => {
  const percentage = Math.min((value / max) * 100, 100);
  
  const colorClasses = {
    blue: 'bg-blue-500',
    green: 'bg-green-500',
    yellow: 'bg-yellow-500',
    red: 'bg-red-500',
  };

  const getColor = () => {
    if (percentage > 90) return 'red';
    if (percentage > 70) return 'yellow';
    if (percentage > 50) return 'green';
    return 'blue';
  };

  const actualColor = color === 'blue' ? getColor() : color;

  return (
    <div className="flex items-center space-x-2 text-sm">
      <span className="text-gray-600 dark:text-gray-400 w-12">{label}:</span>
      <div className="flex-1 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
        <div
          className={`h-full rounded-full transition-all duration-300 ${colorClasses[actualColor]}`}
          style={{ width: `${percentage}%` }}
        />
      </div>
      <span className="text-gray-600 dark:text-gray-400 w-12 text-right">
        {value.toFixed(1)}{unit}
      </span>
    </div>
  );
};

const AgentCard: React.FC<AgentCardProps> = ({ agent, onAction, loading = false }) => {
  const [actionLoading, setActionLoading] = useState<Record<string, boolean>>({});
  const { toast } = useToast();

  const formatLastActive = (timestamp: string) => {
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

  const handleAction = async (action: string) => {
    setActionLoading(prev => ({ ...prev, [action]: true }));
    
    try {
      let response;
      
      switch (action) {
        case 'start':
          response = await apiClient.post(`/agents/${agent.id}/start`);
          break;
        case 'stop':
          response = await apiClient.post(`/agents/${agent.id}/stop`);
          break;
        case 'restart':
          response = await apiClient.post(`/agents/${agent.id}/restart`);
          break;
        case 'pause':
          response = await apiClient.post(`/agents/${agent.id}/pause`);
          break;
        default:
          throw new Error(`Unknown action: ${action}`);
      }

      toast({
        title: 'Action Successful',
        description: `Agent ${action} completed successfully.`,
      });

      onAction?.(action);
    } catch (error) {
      toast({
        title: 'Action Failed',
        description: `Failed to ${action} agent. Please try again.`,
        variant: 'destructive',
      });
    } finally {
      setActionLoading(prev => ({ ...prev, [action]: false }));
    }
  };

  const getAvailableActions = () => {
    switch (agent.status) {
      case 'idle':
        return ['start', 'restart'];
      case 'running':
        return ['stop', 'pause', 'restart'];
      case 'stopped':
        return ['start', 'restart'];
      case 'error':
        return ['restart', 'stop'];
      default:
        return [];
    }
  };

  const actionLabels = {
    start: 'Start',
    stop: 'Stop',
    restart: 'Restart',
    pause: 'Pause',
  };

  const actionIcons = {
    start: '▶️',
    stop: '⏹️',
    restart: '🔄',
    pause: '⏸️',
  };

  return (
    <Card className="transition-all duration-200 hover:shadow-md">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <div className="w-8 h-8 bg-blue-100 dark:bg-blue-900/20 rounded-full flex items-center justify-center">
              <span className="text-blue-600 dark:text-blue-400 text-sm font-bold">
                {agent.name.charAt(0).toUpperCase()}
              </span>
            </div>
            <div>
              <CardTitle className="text-sm font-medium">{agent.name}</CardTitle>
              <p className="text-xs text-gray-500 dark:text-gray-400">{agent.type}</p>
            </div>
          </div>
          <StatusBadge status={agent.status} />
        </div>
      </CardHeader>
      
      <CardContent className="space-y-4">
        {/* Resource Usage */}
        <div className="space-y-2">
          <ResourceUsageMeter
            label="CPU"
            value={agent.cpuUsage}
            unit="%"
          />
          <ResourceUsageMeter
            label="RAM"
            value={agent.memoryUsage}
            unit="%"
          />
        </div>

        {/* Current Task */}
        {agent.currentTask && (
          <div>
            <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">Current Task:</p>
            <p className="text-sm text-gray-700 dark:text-gray-300 truncate" title={agent.currentTask}>
              {agent.currentTask}
            </p>
          </div>
        )}

        {/* Worktree */}
        {agent.worktreeId && (
          <div>
            <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">Worktree:</p>
            <p className="text-sm text-gray-700 dark:text-gray-300">
              {agent.worktreeId}
            </p>
          </div>
        )}

        {/* Last Active */}
        <div>
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">Last Active:</p>
          <p className="text-sm text-gray-700 dark:text-gray-300">
            {formatLastActive(agent.lastActive)}
          </p>
        </div>

        {/* Actions */}
        <div className="flex flex-wrap gap-2 pt-2">
          {getAvailableActions().map(action => (
            <Button
              key={action}
              size="sm"
              variant="outline"
              onClick={() => handleAction(action)}
              disabled={actionLoading[action] || loading}
              className="text-xs"
            >
              {actionLoading[action] ? (
                <div className="w-3 h-3 animate-spin rounded-full border border-gray-300 border-t-gray-600 mr-1"></div>
              ) : (
                <span className="mr-1">{actionIcons[action as keyof typeof actionIcons]}</span>
              )}
              {actionLabels[action as keyof typeof actionLabels]}
            </Button>
          ))}
        </div>

        {/* Details Dialog */}
        <Dialog>
          <DialogTrigger asChild>
            <Button variant="ghost" size="sm" className="w-full text-xs">
              View Details
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle className="flex items-center space-x-2">
                <div className="w-8 h-8 bg-blue-100 dark:bg-blue-900/20 rounded-full flex items-center justify-center">
                  <span className="text-blue-600 dark:text-blue-400 text-sm font-bold">
                    {agent.name.charAt(0).toUpperCase()}
                  </span>
                </div>
                <span>{agent.name}</span>
                <StatusBadge status={agent.status} />
              </DialogTitle>
            </DialogHeader>
            
            <div className="space-y-6">
              {/* Basic Info */}
              <div>
                <h4 className="font-medium text-gray-900 dark:text-white mb-3">Basic Information</h4>
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="font-medium">ID:</span> {agent.id}
                  </div>
                  <div>
                    <span className="font-medium">Type:</span> {agent.type}
                  </div>
                  <div>
                    <span className="font-medium">Status:</span> {agent.status}
                  </div>
                  <div>
                    <span className="font-medium">Last Active:</span> {formatLastActive(agent.lastActive)}
                  </div>
                </div>
              </div>

              {/* Resource Usage */}
              <div>
                <h4 className="font-medium text-gray-900 dark:text-white mb-3">Resource Usage</h4>
                <div className="space-y-3">
                  <ResourceUsageMeter
                    label="CPU Usage"
                    value={agent.cpuUsage}
                    unit="%"
                  />
                  <ResourceUsageMeter
                    label="Memory Usage"
                    value={agent.memoryUsage}
                    unit="%"
                  />
                </div>
              </div>

              {/* Current Task */}
              {agent.currentTask && (
                <div>
                  <h4 className="font-medium text-gray-900 dark:text-white mb-3">Current Task</h4>
                  <p className="text-gray-600 dark:text-gray-300 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                    {agent.currentTask}
                  </p>
                </div>
              )}

              {/* Worktree Info */}
              {agent.worktreeId && (
                <div>
                  <h4 className="font-medium text-gray-900 dark:text-white mb-3">Worktree</h4>
                  <p className="text-gray-600 dark:text-gray-300">
                    Working in worktree: <code className="bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded">{agent.worktreeId}</code>
                  </p>
                </div>
              )}

              {/* Actions */}
              <div>
                <h4 className="font-medium text-gray-900 dark:text-white mb-3">Available Actions</h4>
                <div className="flex flex-wrap gap-2">
                  {getAvailableActions().map(action => (
                    <Button
                      key={action}
                      variant="outline"
                      onClick={() => handleAction(action)}
                      disabled={actionLoading[action] || loading}
                    >
                      {actionLoading[action] ? (
                        <div className="w-4 h-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 mr-2"></div>
                      ) : (
                        <span className="mr-2">{actionIcons[action as keyof typeof actionIcons]}</span>
                      )}
                      {actionLabels[action as keyof typeof actionLabels]}
                    </Button>
                  ))}
                </div>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
};

export const AgentStatusCards: React.FC<AgentStatusCardsProps> = ({
  agents,
  loading = false,
  error,
  onAgentAction,
}) => {
  if (loading) {
    return (
      <div>
        <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4">Agent Status</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, index) => (
            <Card key={index} className="animate-pulse">
              <CardHeader>
                <div className="flex items-center space-x-2">
                  <div className="w-8 h-8 bg-gray-200 dark:bg-gray-700 rounded-full"></div>
                  <div className="space-y-1">
                    <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-20"></div>
                    <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-16"></div>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div className="h-2 bg-gray-200 dark:bg-gray-700 rounded"></div>
                  <div className="h-2 bg-gray-200 dark:bg-gray-700 rounded"></div>
                  <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded"></div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div>
        <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4">Agent Status</h3>
        <Card className="border-red-200 dark:border-red-800">
          <CardContent className="p-6">
            <div className="text-center text-red-600 dark:text-red-400">
              <div className="text-2xl mb-2">⚠️</div>
              <div>Error loading agent status: {error}</div>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-medium text-gray-900 dark:text-white">
          Agent Status
        </h3>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          {agents.length} {agents.length === 1 ? 'agent' : 'agents'}
        </span>
      </div>
      
      {agents.length === 0 ? (
        <Card>
          <CardContent className="p-8">
            <div className="text-center text-gray-500 dark:text-gray-400">
              <div className="text-4xl mb-4">🤖</div>
              <div className="text-lg font-medium mb-2">No Active Agents</div>
              <div className="text-sm">Start an agent to see its status here</div>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {agents.map((agent) => (
            <AgentCard
              key={agent.id}
              agent={agent}
              onAction={(action) => onAgentAction?.(agent.id, action, true)}
              loading={loading}
            />
          ))}
        </div>
      )}
    </div>
  );
};