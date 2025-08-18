import React, { useState, useEffect, useRef } from 'react';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { 
  ProgressBar, 
  FileProgressBar, 
  StatusBadge, 
  LogViewer, 
  ProgressToast, 
  ToastContainer, 
  ActivityIndicator, 
  MultiActivityIndicator,
  useProgressToast,
  type LogEntry,
  type ActivityType,
  type ActivityStatus,
} from './index';

/**
 * Demo component for testing progress indicators with high-frequency updates
 * This component simulates various real-world scenarios to test performance
 */
export const ProgressDemo: React.FC = () => {
  const [progress, setProgress] = useState(0);
  const [fileProgress, setFileProgress] = useState({ processed: 0, total: 100 });
  const [isRunning, setIsRunning] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [activities, setActivities] = useState<Array<{
    id: string;
    type: ActivityType;
    status: ActivityStatus;
    label: string;
    progress?: number;
  }>>([]);

  const { toasts, showToast, dismissToast } = useProgressToast();
  const logCountRef = useRef(0);
  const startTimeRef = useRef<Date>();

  // Simulate high-frequency progress updates
  useEffect(() => {
    if (!isRunning) return;

    const interval = setInterval(() => {
      setProgress(prev => {
        const newProgress = prev + Math.random() * 5;
        return Math.min(newProgress, 100);
      });

      setFileProgress(prev => {
        const newProcessed = Math.min(prev.processed + Math.floor(Math.random() * 3), prev.total);
        return { ...prev, processed: newProcessed };
      });
    }, 50); // 20 updates per second

    return () => clearInterval(interval);
  }, [isRunning]);

  // Simulate high-frequency log generation
  useEffect(() => {
    if (!isRunning) return;

    const interval = setInterval(() => {
      const newLogs: LogEntry[] = [];
      const batchSize = Math.floor(Math.random() * 5) + 1; // 1-5 logs per batch

      for (let i = 0; i < batchSize; i++) {
        logCountRef.current++;
        newLogs.push({
          id: `log-${logCountRef.current}`,
          timestamp: new Date(),
          level: ['debug', 'info', 'warn', 'error'][Math.floor(Math.random() * 4)] as LogEntry['level'],
          source: ['agent', 'maproom', 'websocket', 'ui'][Math.floor(Math.random() * 4)],
          message: `High-frequency log message ${logCountRef.current}: ${generateRandomMessage()}`,
          metadata: { batch: Math.floor(logCountRef.current / batchSize) },
        });
      }

      setLogs(prev => [...prev, ...newLogs].slice(-1000)); // Keep last 1000 logs
    }, 100); // 10 batches per second = up to 50 logs per second

    return () => clearInterval(interval);
  }, [isRunning]);

  // Simulate activity updates
  useEffect(() => {
    if (!isRunning) return;

    const interval = setInterval(() => {
      setActivities(prev => {
        const updated = [...prev];
        
        // Update existing activities
        updated.forEach(activity => {
          if (activity.status === 'active' && activity.progress !== undefined) {
            activity.progress = Math.min(activity.progress + Math.random() * 10, 100);
            
            if (activity.progress >= 100) {
              activity.status = 'completed';
            }
          }
        });

        // Add new activities occasionally
        if (Math.random() < 0.1 && updated.length < 10) {
          const activityTypes: ActivityType[] = ['processing', 'indexing', 'syncing', 'analyzing'];
          const newActivity = {
            id: `activity-${Date.now()}-${Math.random().toString(36).substr(2, 4)}`,
            type: activityTypes[Math.floor(Math.random() * activityTypes.length)],
            status: 'active' as ActivityStatus,
            label: `Task ${updated.length + 1}`,
            progress: Math.random() * 50,
          };
          updated.push(newActivity);
        }

        // Remove completed activities after a delay
        return updated.filter(activity => 
          activity.status !== 'completed' || Math.random() > 0.05
        );
      });
    }, 200); // 5 updates per second

    return () => clearInterval(interval);
  }, [isRunning]);

  const generateRandomMessage = (): string => {
    const messages = [
      'Processing batch data with optimized algorithms',
      'Indexing file system changes detected',
      'WebSocket connection established successfully',
      'Memory usage within acceptable limits',
      'Database query executed in optimal time',
      'Cache invalidation triggered for performance',
      'Background task completed successfully',
      'Real-time synchronization in progress',
      'Performance metrics collected and analyzed',
      'User interface rendering at 60fps',
    ];
    return messages[Math.floor(Math.random() * messages.length)];
  };

  const startDemo = () => {
    setIsRunning(true);
    setProgress(0);
    setFileProgress({ processed: 0, total: 100 });
    setLogs([]);
    setActivities([]);
    startTimeRef.current = new Date();
    
    showToast({
      title: 'Demo Started',
      description: 'High-frequency updates simulation started',
      type: 'info',
      duration: 3000,
    });
  };

  const stopDemo = () => {
    setIsRunning(false);
    
    showToast({
      title: 'Demo Stopped',
      description: 'Performance test completed',
      type: 'success',
      duration: 3000,
      actions: [
        {
          label: 'View Results',
          onClick: () => {
            console.log('Demo results:', {
              duration: startTimeRef.current ? Date.now() - startTimeRef.current.getTime() : 0,
              logsGenerated: logCountRef.current,
              finalProgress: progress,
            });
          },
        },
      ],
    });
  };

  const triggerError = () => {
    showToast({
      title: 'Simulated Error',
      description: 'This is a test error notification with actions',
      type: 'error',
      persistent: true,
      actions: [
        {
          label: 'Retry',
          onClick: () => console.log('Retry clicked'),
          variant: 'default',
        },
        {
          label: 'Details',
          onClick: () => console.log('Details clicked'),
          variant: 'outline',
        },
      ],
    });
  };

  const showLoadingToast = () => {
    const id = showToast({
      title: 'Processing...',
      description: 'This operation will take a moment',
      type: 'loading',
      persistent: true,
      progress: 0,
    });

    // Simulate progress updates
    let currentProgress = 0;
    const interval = setInterval(() => {
      currentProgress += 10;
      // In a real implementation, you'd use updateToast here
      if (currentProgress >= 100) {
        clearInterval(interval);
        dismissToast(id);
        showToast({
          title: 'Complete!',
          description: 'Operation finished successfully',
          type: 'success',
          duration: 3000,
        });
      }
    }, 500);
  };

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Progress Indicators Demo</h1>
        <div className="space-x-2">
          <Button onClick={startDemo} disabled={isRunning}>
            Start High-Frequency Test
          </Button>
          <Button onClick={stopDemo} disabled={!isRunning} variant="outline">
            Stop Test
          </Button>
          <Button onClick={triggerError} variant="destructive">
            Trigger Error
          </Button>
          <Button onClick={showLoadingToast} variant="secondary">
            Show Loading
          </Button>
        </div>
      </div>

      {/* Performance indicator */}
      {isRunning && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center space-x-2">
              <div className="w-3 h-3 bg-green-500 rounded-full animate-pulse" />
              <span>High-Frequency Updates Active</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-4 text-sm">
              <div>
                <span className="font-medium">Progress Updates:</span> 20/sec
              </div>
              <div>
                <span className="font-medium">Log Generation:</span> up to 50/sec
              </div>
              <div>
                <span className="font-medium">Activity Updates:</span> 5/sec
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Progress Bars */}
        <Card>
          <CardHeader>
            <CardTitle>Progress Bars</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <ProgressBar
              value={progress}
              label="Main Progress"
              showEta
              startTime={startTimeRef.current}
              animated
              pulse={isRunning}
            />
            
            <FileProgressBar
              value={(fileProgress.processed / fileProgress.total) * 100}
              filesProgress={fileProgress}
              currentFile={`file-${fileProgress.processed}.ts`}
              speed={isRunning ? Math.random() * 10 + 5 : 0}
            />

            <ProgressBar
              value={isRunning ? undefined : 100}
              indeterminate={isRunning}
              label="Indeterminate Progress"
              size="sm"
            />
          </CardContent>
        </Card>

        {/* Status Badges */}
        <Card>
          <CardHeader>
            <CardTitle>Status Badges</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex flex-wrap gap-2">
              <StatusBadge status="online" pulse={isRunning} />
              <StatusBadge status="busy" showLastUpdate lastUpdate={new Date()} />
              <StatusBadge status="running" />
              <StatusBadge status="error" />
              <StatusBadge status="offline" />
            </div>
            
            <div className="space-y-2">
              <h4 className="font-medium">Agent Status (Simulated)</h4>
              <div className="flex flex-wrap gap-2">
                <StatusBadge 
                  status={isRunning ? "running" : "idle"} 
                  label="Agent 1 (Claude)"
                  size="md"
                />
                <StatusBadge 
                  status={isRunning ? "busy" : "offline"} 
                  label="Agent 2 (GPT-4)"
                  size="md"
                />
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Activity Indicators */}
        <Card>
          <CardHeader>
            <CardTitle>Activity Indicators</CardTitle>
          </CardHeader>
          <CardContent>
            <MultiActivityIndicator
              activities={activities}
              maxShow={5}
              direction="vertical"
              variant="default"
              showSummary
            />
          </CardContent>
        </Card>

        {/* Log Viewer */}
        <Card>
          <CardHeader>
            <CardTitle>Log Viewer ({logs.length} logs)</CardTitle>
          </CardHeader>
          <CardContent>
            <LogViewer
              logs={logs}
              height={300}
              isStreaming={isRunning}
              autoScroll
              showSearch
              showControls
              onClear={() => setLogs([])}
              onExport={(exportedLogs) => {
                console.log('Exported logs:', exportedLogs.length);
                const dataStr = JSON.stringify(exportedLogs, null, 2);
                const dataBlob = new Blob([dataStr], { type: 'application/json' });
                const url = URL.createObjectURL(dataBlob);
                const link = document.createElement('a');
                link.href = url;
                link.download = 'exported-logs.json';
                link.click();
                URL.revokeObjectURL(url);
              }}
            />
          </CardContent>
        </Card>
      </div>

      {/* Memory and Performance Info */}
      {isRunning && (
        <Card>
          <CardHeader>
            <CardTitle>Performance Monitoring</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 text-sm">
              <div>
                <span className="font-medium">Memory Usage:</span>
                <div className="text-xs text-gray-600">
                  {(performance as any).memory ? 
                    `${Math.round((performance as any).memory.usedJSHeapSize / 1024 / 1024)}MB` : 
                    'N/A'
                  }
                </div>
              </div>
              <div>
                <span className="font-medium">Components:</span>
                <div className="text-xs text-gray-600">
                  {activities.length} activities, {logs.length} logs
                </div>
              </div>
              <div>
                <span className="font-medium">Update Rate:</span>
                <div className="text-xs text-gray-600">
                  60+ FPS target
                </div>
              </div>
              <div>
                <span className="font-medium">Virtual Scrolling:</span>
                <div className="text-xs text-gray-600">
                  {logs.length > 100 ? 'Active' : 'Inactive'}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Toast Container */}
      <ToastContainer
        toasts={toasts}
        onDismissToast={dismissToast}
        position="top-right"
        maxToasts={5}
      />
    </div>
  );
};

export default ProgressDemo;