import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Button } from '../ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '../ui/dialog';
import { useToast } from '../ui/use-toast';
import { apiClient } from '../../utils/apiClient';

interface QuickActionsProps {
  onActionComplete?: (action: string, success: boolean) => void;
}

interface ActionButtonProps {
  title: string;
  description: string;
  icon: React.ReactNode;
  onClick: () => void | Promise<void>;
  loading?: boolean;
  variant?: 'default' | 'outline' | 'destructive';
  disabled?: boolean;
}

const ActionButton: React.FC<ActionButtonProps> = ({
  title,
  description,
  icon,
  onClick,
  loading = false,
  variant = 'default',
  disabled = false,
}) => {
  return (
    <Button
      variant={variant}
      onClick={onClick}
      disabled={disabled || loading}
      className="h-auto p-6 flex flex-col items-center justify-center space-y-2 relative group"
      title={description}
    >
      {loading ? (
        <div className="w-6 h-6 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600"></div>
      ) : (
        <div className="text-xl group-hover:scale-110 transition-transform duration-200">
          {icon}
        </div>
      )}
      <span className="text-sm font-medium text-center">{title}</span>
      
      {/* Tooltip */}
      <div className="absolute bottom-full left-1/2 transform -translate-x-1/2 mb-2 px-3 py-2 bg-gray-900 text-white text-xs rounded-lg opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none whitespace-nowrap z-10">
        {description}
        <div className="absolute top-full left-1/2 transform -translate-x-1/2 w-0 h-0 border-l-4 border-r-4 border-t-4 border-transparent border-t-gray-900"></div>
      </div>
    </Button>
  );
};

const ConfirmationDialog: React.FC<{
  title: string;
  description: string;
  onConfirm: () => void;
  onCancel: () => void;
  isOpen: boolean;
  loading?: boolean;
  destructive?: boolean;
}> = ({ title, description, onConfirm, onCancel, isOpen, loading = false, destructive = false }) => {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className={destructive ? 'text-red-600 dark:text-red-400' : ''}>
            {title}
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <p className="text-gray-600 dark:text-gray-300">{description}</p>
          <div className="flex justify-end space-x-2">
            <Button variant="outline" onClick={onCancel} disabled={loading}>
              Cancel
            </Button>
            <Button
              variant={destructive ? 'destructive' : 'default'}
              onClick={onConfirm}
              disabled={loading}
            >
              {loading ? (
                <div className="w-4 h-4 animate-spin rounded-full border-2 border-gray-300 border-t-white mr-2"></div>
              ) : null}
              Confirm
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export const QuickActions: React.FC<QuickActionsProps> = ({ onActionComplete }) => {
  const [loadingStates, setLoadingStates] = useState<Record<string, boolean>>({});
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    title: string;
    description: string;
    action: () => Promise<void>;
    destructive?: boolean;
  }>({
    isOpen: false,
    title: '',
    description: '',
    action: async () => {},
  });

  const { toast } = useToast();

  const setLoading = (action: string, loading: boolean) => {
    setLoadingStates(prev => ({ ...prev, [action]: loading }));
  };

  const showConfirmation = (
    title: string,
    description: string,
    action: () => Promise<void>,
    destructive = false
  ) => {
    setConfirmDialog({
      isOpen: true,
      title,
      description,
      action,
      destructive,
    });
  };

  const handleConfirm = async () => {
    setLoading('confirm', true);
    try {
      await confirmDialog.action();
    } finally {
      setLoading('confirm', false);
      setConfirmDialog(prev => ({ ...prev, isOpen: false }));
    }
  };

  const handleCancel = () => {
    setConfirmDialog(prev => ({ ...prev, isOpen: false }));
  };

  const createWorktree = async () => {
    try {
      // This would typically open a form dialog
      // For now, we'll create a default worktree
      const response = await apiClient.post('/worktrees', {
        worktree_name: `worktree-${Date.now()}`,
        repo_id: 1, // Default repo
        current_branch: 'main',
        state: 'active',
      });

      toast({
        title: 'Worktree Created',
        description: 'New worktree has been created successfully.',
      });

      onActionComplete?.('create-worktree', true);
    } catch (error) {
      toast({
        title: 'Error',
        description: 'Failed to create worktree. Please try again.',
        variant: 'destructive',
      });

      onActionComplete?.('create-worktree', false);
    }
  };

  const spawnAgent = async () => {
    try {
      // This would typically open an agent configuration dialog
      const response = await apiClient.post('/agents/runs', {
        agent_id: `agent-${Date.now()}`,
        agent_type: 'claude-3-5-sonnet',
        repo_id: 1,
        worktree_id: 1,
        commit_sha: 'HEAD',
        task_description: 'General development task',
        task_type: 'development',
        instructions: {},
        context_files: [],
        review_required: false,
        auto_merge_eligible: true,
        tags: ['dashboard-created'],
      });

      toast({
        title: 'Agent Spawned',
        description: 'New agent has been started successfully.',
      });

      onActionComplete?.('spawn-agent', true);
    } catch (error) {
      toast({
        title: 'Error',
        description: 'Failed to spawn agent. Please try again.',
        variant: 'destructive',
      });

      onActionComplete?.('spawn-agent', false);
    }
  };

  const runIndexing = async () => {
    try {
      // This would trigger maproom indexing
      const response = await apiClient.post('/maproom/scan', {
        force: true,
      });

      toast({
        title: 'Indexing Started',
        description: 'Code indexing has been initiated.',
      });

      onActionComplete?.('run-indexing', true);
    } catch (error) {
      toast({
        title: 'Error',
        description: 'Failed to start indexing. Please try again.',
        variant: 'destructive',
      });

      onActionComplete?.('run-indexing', false);
    }
  };

  const viewLogs = async () => {
    try {
      // This would open the logs page or drawer
      window.open('/logs', '_blank');

      toast({
        title: 'Logs Opened',
        description: 'System logs opened in new tab.',
      });

      onActionComplete?.('view-logs', true);
    } catch (error) {
      toast({
        title: 'Error',
        description: 'Failed to open logs. Please try again.',
        variant: 'destructive',
      });

      onActionComplete?.('view-logs', false);
    }
  };

  const refreshDashboard = async () => {
    try {
      // Trigger a dashboard refresh
      window.location.reload();
    } catch (error) {
      toast({
        title: 'Error',
        description: 'Failed to refresh dashboard.',
        variant: 'destructive',
      });

      onActionComplete?.('refresh-dashboard', false);
    }
  };

  const clearCache = async () => {
    try {
      const response = await apiClient.post('/system/clear-cache');

      toast({
        title: 'Cache Cleared',
        description: 'System cache has been cleared successfully.',
      });

      onActionComplete?.('clear-cache', true);
    } catch (error) {
      toast({
        title: 'Error',
        description: 'Failed to clear cache. Please try again.',
        variant: 'destructive',
      });

      onActionComplete?.('clear-cache', false);
    }
  };

  const actions = [
    {
      id: 'create-worktree',
      title: 'Create Worktree',
      description: 'Create a new git worktree for isolated development',
      icon: (
        <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
        </svg>
      ),
      onClick: () => {
        showConfirmation(
          'Create New Worktree',
          'This will create a new git worktree for isolated development. Continue?',
          async () => {
            setLoading('create-worktree', true);
            try {
              await createWorktree();
            } finally {
              setLoading('create-worktree', false);
            }
          }
        );
      },
      variant: 'default' as const,
    },
    {
      id: 'spawn-agent',
      title: 'Spawn Agent',
      description: 'Start a new AI agent for development tasks',
      icon: (
        <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
      ),
      onClick: () => {
        showConfirmation(
          'Spawn New Agent',
          'This will create and start a new AI agent. Continue?',
          async () => {
            setLoading('spawn-agent', true);
            try {
              await spawnAgent();
            } finally {
              setLoading('spawn-agent', false);
            }
          }
        );
      },
      variant: 'default' as const,
    },
    {
      id: 'run-indexing',
      title: 'Run Indexing',
      description: 'Start code indexing for improved search',
      icon: (
        <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
      ),
      onClick: () => {
        showConfirmation(
          'Start Code Indexing',
          'This will start the maproom indexing process. This may take several minutes. Continue?',
          async () => {
            setLoading('run-indexing', true);
            try {
              await runIndexing();
            } finally {
              setLoading('run-indexing', false);
            }
          }
        );
      },
      variant: 'outline' as const,
    },
    {
      id: 'view-logs',
      title: 'View Logs',
      description: 'Open system logs for debugging',
      icon: (
        <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
      ),
      onClick: async () => {
        await viewLogs();
      },
      variant: 'outline' as const,
    },
    {
      id: 'refresh-dashboard',
      title: 'Refresh',
      description: 'Reload the dashboard with latest data',
      icon: (
        <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
      ),
      onClick: async () => {
        await refreshDashboard();
      },
      variant: 'outline' as const,
    },
    {
      id: 'clear-cache',
      title: 'Clear Cache',
      description: 'Clear system cache and temporary files',
      icon: (
        <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
      ),
      onClick: () => {
        showConfirmation(
          'Clear System Cache',
          'This will clear all cached data and temporary files. This action cannot be undone. Continue?',
          async () => {
            setLoading('clear-cache', true);
            try {
              await clearCache();
            } finally {
              setLoading('clear-cache', false);
            }
          },
          true // destructive
        );
      },
      variant: 'outline' as const,
    },
  ];

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            {actions.map((action) => (
              <ActionButton
                key={action.id}
                title={action.title}
                description={action.description}
                icon={action.icon}
                onClick={action.onClick}
                loading={loadingStates[action.id]}
                variant={action.variant}
              />
            ))}
          </div>
        </CardContent>
      </Card>

      <ConfirmationDialog
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={handleConfirm}
        onCancel={handleCancel}
        isOpen={confirmDialog.isOpen}
        loading={loadingStates.confirm}
        destructive={confirmDialog.destructive}
      />
    </>
  );
};