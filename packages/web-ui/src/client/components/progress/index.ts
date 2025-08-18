/**
 * Progress Components
 * 
 * Live progress indicators for the CrewChief Web UI.
 * Includes progress bars, status badges, log viewers, toast notifications, and activity indicators.
 */

// Progress Bar Components
export { 
  ProgressBar, 
  FileProgressBar,
  type ProgressBarProps,
  type FileProgressBarProps,
} from './ProgressBar';

// Status Badge Components
export { 
  StatusBadge, 
  AgentStatusBadge, 
  MultiStatus,
  type StatusBadgeProps,
  type AgentStatusBadgeProps,
  type MultiStatusProps,
  type StatusType,
} from './StatusBadge';

// Log Viewer Components
export { 
  LogViewer,
  type LogViewerProps,
  type LogEntry,
} from './LogViewer';

// Toast Notification Components
export { 
  ProgressToast, 
  ToastContainer, 
  useProgressToast,
  type ProgressToastProps,
  type ToastContainerProps,
  type ToastAction,
  type UseProgressToastReturn,
} from './ProgressToast';

// Activity Indicator Components
export { 
  ActivityIndicator, 
  MultiActivityIndicator, 
  PulseIndicator,
  type ActivityIndicatorProps,
  type MultiActivityIndicatorProps,
  type PulseIndicatorProps,
  type ActivityType,
  type ActivityStatus,
} from './ActivityIndicator';

// Hooks
export { 
  useProgress, 
  usePerformanceMonitor, 
  useOperationHistory,
  type UseProgressReturn,
  type UsePerformanceMonitorReturn,
  type UseOperationHistoryReturn,
  type ProgressItem,
  type PerformanceMetrics,
  type OperationHistoryItem,
} from '../hooks/useProgress';

// Demo and Test Components
export { ProgressDemo } from './ProgressDemo';
export { default as AccessibilityTest } from './AccessibilityTest';
export { default as PerformanceMonitor } from './PerformanceMonitor';