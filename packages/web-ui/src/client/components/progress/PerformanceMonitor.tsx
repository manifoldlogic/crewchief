import React, { useState, useEffect, useRef } from 'react';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { 
  ProgressBar, 
  useProgressToast, 
  usePerformanceMonitor,
  ToastContainer,
  LogViewer,
  ActivityIndicator,
  type LogEntry,
} from './index';

/**
 * Performance Monitor Component
 * 
 * Tests and monitors the performance of progress components under stress.
 * Includes memory usage tracking, FPS monitoring, and leak detection.
 */
export const PerformanceMonitor: React.FC = () => {
  const [isStressTest, setIsStressTest] = useState(false);
  const [componentCount, setComponentCount] = useState(10);
  const [updateFrequency, setUpdateFrequency] = useState(50); // ms
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [performanceLog, setPerformanceLog] = useState<string[]>([]);
  
  const { toasts, showToast, dismissToast } = useProgressToast();
  const { 
    metrics, 
    isMonitoring, 
    startMonitoring, 
    stopMonitoring, 
    resetMetrics 
  } = usePerformanceMonitor();

  const stressTestDataRef = useRef<{
    progressBars: Array<{ id: string; progress: number }>;
    activities: Array<{ id: string; progress: number; type: string }>;
    logCount: number;
  }>({
    progressBars: [],
    activities: [],
    logCount: 0,
  });

  const startTimeRef = useRef<number>(0);
  const frameCountRef = useRef<number>(0);
  const lastFpsUpdateRef = useRef<number>(0);
  const [currentFps, setCurrentFps] = useState<number>(60);

  // FPS Counter
  useEffect(() => {
    if (!isStressTest) return;

    let animationId: number;
    
    const measureFps = () => {
      frameCountRef.current++;
      const now = performance.now();
      
      if (now - lastFpsUpdateRef.current >= 1000) {
        const fps = Math.round((frameCountRef.current * 1000) / (now - lastFpsUpdateRef.current));
        setCurrentFps(fps);
        frameCountRef.current = 0;
        lastFpsUpdateRef.current = now;
      }
      
      animationId = requestAnimationFrame(measureFps);
    };

    animationId = requestAnimationFrame(measureFps);
    return () => cancelAnimationFrame(animationId);
  }, [isStressTest]);

  // Stress test simulation
  useEffect(() => {
    if (!isStressTest) return;

    const interval = setInterval(() => {
      const data = stressTestDataRef.current;
      
      // Update progress bars
      data.progressBars = data.progressBars.map(bar => ({
        ...bar,
        progress: Math.min(bar.progress + Math.random() * 5, 100),
      }));

      // Add/remove progress bars dynamically
      if (data.progressBars.length < componentCount && Math.random() < 0.1) {
        data.progressBars.push({
          id: `progress-${Date.now()}-${Math.random().toString(36).substr(2, 4)}`,
          progress: 0,
        });
      }

      // Remove completed progress bars
      data.progressBars = data.progressBars.filter(bar => bar.progress < 100);

      // Update activities
      data.activities = data.activities.map(activity => ({
        ...activity,
        progress: Math.min(activity.progress + Math.random() * 3, 100),
      }));

      // Add new activities
      if (data.activities.length < Math.floor(componentCount / 2) && Math.random() < 0.15) {
        const types = ['processing', 'indexing', 'syncing', 'analyzing', 'building'];
        data.activities.push({
          id: `activity-${Date.now()}-${Math.random().toString(36).substr(2, 4)}`,
          progress: 0,
          type: types[Math.floor(Math.random() * types.length)],
        });
      }

      // Remove completed activities
      data.activities = data.activities.filter(activity => activity.progress < 100);

      // Generate logs
      const logBatch = Math.floor(Math.random() * 3) + 1;
      const newLogs: LogEntry[] = [];
      
      for (let i = 0; i < logBatch; i++) {
        data.logCount++;
        newLogs.push({
          id: `perf-log-${data.logCount}`,
          timestamp: new Date(),
          level: ['debug', 'info', 'warn', 'error'][Math.floor(Math.random() * 4)] as LogEntry['level'],
          source: 'performance-test',
          message: `Performance test log ${data.logCount}: ${generateTestMessage()}`,
          metadata: { 
            fps: currentFps,
            memoryUsage: metrics.memoryUsage,
            componentCount: data.progressBars.length + data.activities.length,
          },
        });
      }

      setLogs(prev => [...prev, ...newLogs].slice(-1000)); // Keep last 1000 logs

      // Force re-render to test React performance
      setStressTestDataRef({ ...data });
    }, updateFrequency);

    return () => clearInterval(interval);
  }, [isStressTest, componentCount, updateFrequency, currentFps, metrics.memoryUsage]);

  // Performance logging
  useEffect(() => {
    if (!isMonitoring) return;

    const interval = setInterval(() => {
      const memInfo = (performance as any).memory;
      const timestamp = new Date().toISOString();
      
      const logEntry = [
        timestamp,
        `FPS: ${currentFps}`,
        `Memory: ${Math.round(metrics.memoryUsage)}%`,
        memInfo ? `JS Heap: ${Math.round(memInfo.usedJSHeapSize / 1024 / 1024)}MB` : 'N/A',
        `Components: ${stressTestDataRef.current.progressBars.length + stressTestDataRef.current.activities.length}`,
        `Logs: ${logs.length}`,
      ].join(' | ');

      setPerformanceLog(prev => [...prev, logEntry].slice(-100)); // Keep last 100 entries

      // Alert on performance issues
      if (currentFps < 30) {
        showToast({
          title: 'Performance Warning',
          description: `FPS dropped to ${currentFps}. Consider reducing component count.`,
          type: 'warning',
          duration: 5000,
        });
      }

      if (metrics.memoryUsage > 80) {
        showToast({
          title: 'Memory Warning',
          description: `Memory usage at ${Math.round(metrics.memoryUsage)}%. Possible memory leak detected.`,
          type: 'error',
          duration: 8000,
          actions: [
            {
              label: 'Clear Components',
              onClick: () => {
                stopStressTest();
                resetMetrics();
              },
            },
          ],
        });
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [isMonitoring, currentFps, metrics.memoryUsage, metrics.renderPerformance.memoryLeaks, logs.length, showToast]);

  const [stressTestDataState, setStressTestDataRef] = useState(stressTestDataRef.current);

  const generateTestMessage = (): string => {
    const messages = [
      'Stress testing component rendering performance',
      'Virtual scrolling optimization active',
      'Memory allocation within normal parameters',
      'Real-time update processing completed',
      'Component lifecycle management efficient',
      'Animation frame rendering stable',
      'Event handling performance optimal',
      'React reconciliation minimized',
      'Browser paint cycles optimized',
      'JavaScript execution time acceptable',
    ];
    return messages[Math.floor(Math.random() * messages.length)];
  };

  const startStressTest = () => {
    setIsStressTest(true);
    startTimeRef.current = performance.now();
    lastFpsUpdateRef.current = startTimeRef.current;
    frameCountRef.current = 0;
    
    // Initialize test data
    stressTestDataRef.current = {
      progressBars: Array.from({ length: Math.min(componentCount, 5) }, (_, i) => ({
        id: `initial-progress-${i}`,
        progress: Math.random() * 50,
      })),
      activities: Array.from({ length: Math.min(Math.floor(componentCount / 2), 3) }, (_, i) => ({
        id: `initial-activity-${i}`,
        progress: Math.random() * 30,
        type: ['processing', 'indexing', 'syncing'][i % 3],
      })),
      logCount: 0,
    };
    
    setStressTestDataRef(stressTestDataRef.current);
    startMonitoring();
    
    showToast({
      title: 'Stress Test Started',
      description: `Testing with ${componentCount} components at ${1000/updateFrequency}Hz update rate`,
      type: 'info',
      duration: 3000,
    });
  };

  const stopStressTest = () => {
    setIsStressTest(false);
    stopMonitoring();
    
    const duration = performance.now() - startTimeRef.current;
    const avgFps = Math.round(frameCountRef.current / (duration / 1000));
    
    showToast({
      title: 'Stress Test Completed',
      description: `Duration: ${Math.round(duration/1000)}s, Average FPS: ${avgFps}`,
      type: 'success',
      duration: 5000,
      actions: [
        {
          label: 'View Report',
          onClick: () => {
            console.log('Performance Report:', {
              duration: Math.round(duration),
              averageFps: avgFps,
              finalMetrics: metrics,
              performanceLog,
            });
          },
        },
      ],
    });
  };

  const exportPerformanceData = () => {
    const report = {
      timestamp: new Date().toISOString(),
      testConfiguration: {
        componentCount,
        updateFrequency,
        duration: performance.now() - startTimeRef.current,
      },
      metrics,
      performanceLog,
      browser: {
        userAgent: navigator.userAgent,
        memory: (performance as any).memory || 'Not available',
        hardwareConcurrency: navigator.hardwareConcurrency,
      },
    };

    const dataStr = JSON.stringify(report, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `performance-report-${Date.now()}.json`;
    link.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Performance Monitor</h1>
        <div className="space-x-2">
          <Button 
            onClick={isStressTest ? stopStressTest : startStressTest}
            variant={isStressTest ? "destructive" : "default"}
          >
            {isStressTest ? 'Stop' : 'Start'} Stress Test
          </Button>
          <Button 
            onClick={exportPerformanceData} 
            variant="outline"
            disabled={!isMonitoring && performanceLog.length === 0}
          >
            Export Report
          </Button>
        </div>
      </div>

      {/* Configuration */}
      <Card>
        <CardHeader>
          <CardTitle>Test Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">
                Component Count: {componentCount}
              </label>
              <input
                type="range"
                min="5"
                max="100"
                value={componentCount}
                onChange={(e) => setComponentCount(Number(e.target.value))}
                disabled={isStressTest}
                className="w-full"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">
                Update Frequency: {Math.round(1000/updateFrequency)}Hz
              </label>
              <input
                type="range"
                min="16"
                max="500"
                value={updateFrequency}
                onChange={(e) => setUpdateFrequency(Number(e.target.value))}
                disabled={isStressTest}
                className="w-full"
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Real-time Metrics */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <Card>
          <CardHeader>
            <CardTitle>Performance Metrics</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="font-medium">FPS:</span>
                <div className={`text-lg font-mono ${currentFps < 30 ? 'text-red-500' : currentFps < 50 ? 'text-yellow-500' : 'text-green-500'}`}>
                  {currentFps}
                </div>
              </div>
              <div>
                <span className="font-medium">Memory:</span>
                <div className={`text-lg font-mono ${metrics.memoryUsage > 80 ? 'text-red-500' : metrics.memoryUsage > 60 ? 'text-yellow-500' : 'text-green-500'}`}>
                  {Math.round(metrics.memoryUsage)}%
                </div>
              </div>
              <div>
                <span className="font-medium">Components:</span>
                <div className="text-lg font-mono">
                  {stressTestDataState.progressBars.length + stressTestDataState.activities.length}
                </div>
              </div>
              <div>
                <span className="font-medium">Logs:</span>
                <div className="text-lg font-mono">
                  {logs.length}
                </div>
              </div>
            </div>

            {/* Memory leak warning */}
            {metrics.renderPerformance.memoryLeaks && (
              <div className="p-2 bg-red-50 border border-red-200 rounded text-red-700 text-sm">
                ⚠️ Potential memory leak detected
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>WebSocket Health</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span>Connected:</span>
                <span className={metrics.websocketHealth.connected ? 'text-green-600' : 'text-red-600'}>
                  {metrics.websocketHealth.connected ? 'Yes' : 'No'}
                </span>
              </div>
              <div className="flex justify-between">
                <span>Latency:</span>
                <span className="font-mono">{metrics.websocketHealth.latency}ms</span>
              </div>
              <div className="flex justify-between">
                <span>Msg/sec:</span>
                <span className="font-mono">{metrics.websocketHealth.messagesPerSecond}</span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Browser Info</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 text-xs">
              <div>
                <span className="font-medium">Cores:</span> {navigator.hardwareConcurrency || 'Unknown'}
              </div>
              <div>
                <span className="font-medium">JS Heap:</span>{' '}
                {(performance as any).memory ? 
                  `${Math.round((performance as any).memory.usedJSHeapSize / 1024 / 1024)}MB` : 
                  'N/A'
                }
              </div>
              <div>
                <span className="font-medium">Monitoring:</span>{' '}
                <span className={isMonitoring ? 'text-green-600' : 'text-gray-500'}>
                  {isMonitoring ? 'Active' : 'Inactive'}
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Dynamic Components Under Test */}
      {isStressTest && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card>
            <CardHeader>
              <CardTitle>Progress Bars ({stressTestDataState.progressBars.length})</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 max-h-64 overflow-y-auto">
              {stressTestDataState.progressBars.map((bar) => (
                <ProgressBar
                  key={bar.id}
                  value={bar.progress}
                  size="sm"
                  label={`Task ${bar.id.split('-').pop()}`}
                  animated
                  showPercentage
                />
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Activities ({stressTestDataState.activities.length})</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 max-h-64 overflow-y-auto">
              {stressTestDataState.activities.map((activity) => (
                <ActivityIndicator
                  key={activity.id}
                  type={activity.type as any}
                  status="active"
                  label={`${activity.type} ${activity.id.split('-').pop()}`}
                  progress={activity.progress}
                  variant="compact"
                />
              ))}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Performance Log */}
      <Card>
        <CardHeader>
          <CardTitle>Live Performance Log</CardTitle>
        </CardHeader>
        <CardContent>
          <LogViewer
            logs={logs}
            height={200}
            isStreaming={isStressTest}
            autoScroll
            showControls
            onClear={() => setLogs([])}
          />
        </CardContent>
      </Card>

      {/* Performance History */}
      <Card>
        <CardHeader>
          <CardTitle>Performance History</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="max-h-40 overflow-y-auto font-mono text-xs space-y-1">
            {performanceLog.map((entry, index) => (
              <div key={index} className="text-gray-600">
                {entry}
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <ToastContainer
        toasts={toasts}
        onDismissToast={dismissToast}
        position="top-right"
        maxToasts={3}
      />
    </div>
  );
};

export default PerformanceMonitor;