import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Loader2, 
  CheckCircle, 
  AlertCircle, 
  Clock, 
  Zap, 
  Database, 
  Upload, 
  Download, 
  Search, 
  RefreshCw, 
  GitBranch,
  FileText,
  Monitor,
  Activity as ActivityIcon,
} from 'lucide-react';
import { cn } from '../../lib/utils';

export type ActivityType = 
  | 'loading'
  | 'processing'
  | 'searching'
  | 'indexing'
  | 'uploading'
  | 'downloading'
  | 'syncing'
  | 'analyzing'
  | 'building'
  | 'testing'
  | 'deploying'
  | 'merging'
  | 'connecting'
  | 'monitoring'
  | 'custom';

export type ActivityStatus = 'active' | 'completed' | 'error' | 'paused' | 'waiting';

export interface ActivityIndicatorProps {
  /** Type of activity */
  type: ActivityType;
  /** Current status */
  status: ActivityStatus;
  /** Activity label */
  label?: string;
  /** Additional description */
  description?: string;
  /** Size variant */
  size?: 'xs' | 'sm' | 'md' | 'lg';
  /** Visual variant */
  variant?: 'default' | 'compact' | 'detailed';
  /** Custom icon */
  icon?: React.ReactNode;
  /** Whether to show pulse animation */
  pulse?: boolean;
  /** Custom className */
  className?: string;
  /** Progress percentage (0-100) */
  progress?: number;
  /** Estimated time remaining */
  eta?: string;
  /** Start time for calculating duration */
  startTime?: Date;
  /** Show duration counter */
  showDuration?: boolean;
  /** Activity metadata */
  metadata?: {
    speed?: string;
    throughput?: string;
    itemsProcessed?: number;
    totalItems?: number;
  };
}

const activityConfig: Record<ActivityType, { icon: React.ComponentType<any>; label: string; color: string }> = {
  loading: { icon: Loader2, label: 'Loading', color: 'text-blue-500' },
  processing: { icon: Zap, label: 'Processing', color: 'text-purple-500' },
  searching: { icon: Search, label: 'Searching', color: 'text-green-500' },
  indexing: { icon: Database, label: 'Indexing', color: 'text-orange-500' },
  uploading: { icon: Upload, label: 'Uploading', color: 'text-blue-500' },
  downloading: { icon: Download, label: 'Downloading', color: 'text-blue-500' },
  syncing: { icon: RefreshCw, label: 'Syncing', color: 'text-indigo-500' },
  analyzing: { icon: ActivityIcon, label: 'Analyzing', color: 'text-pink-500' },
  building: { icon: FileText, label: 'Building', color: 'text-yellow-500' },
  testing: { icon: CheckCircle, label: 'Testing', color: 'text-green-500' },
  deploying: { icon: Upload, label: 'Deploying', color: 'text-red-500' },
  merging: { icon: GitBranch, label: 'Merging', color: 'text-purple-500' },
  connecting: { icon: Loader2, label: 'Connecting', color: 'text-blue-500' },
  monitoring: { icon: Monitor, label: 'Monitoring', color: 'text-gray-500' },
  custom: { icon: ActivityIcon, label: 'Activity', color: 'text-gray-500' },
};

const statusConfig: Record<ActivityStatus, { color: string; bgColor: string }> = {
  active: { color: 'text-blue-600 dark:text-blue-400', bgColor: 'bg-blue-50 dark:bg-blue-900/20' },
  completed: { color: 'text-green-600 dark:text-green-400', bgColor: 'bg-green-50 dark:bg-green-900/20' },
  error: { color: 'text-red-600 dark:text-red-400', bgColor: 'bg-red-50 dark:bg-red-900/20' },
  paused: { color: 'text-yellow-600 dark:text-yellow-400', bgColor: 'bg-yellow-50 dark:bg-yellow-900/20' },
  waiting: { color: 'text-gray-600 dark:text-gray-400', bgColor: 'bg-gray-50 dark:bg-gray-900/20' },
};

const sizeConfig = {
  xs: { 
    icon: 'h-3 w-3', 
    text: 'text-xs', 
    container: 'p-1',
    progress: 'h-1',
  },
  sm: { 
    icon: 'h-4 w-4', 
    text: 'text-sm', 
    container: 'p-2',
    progress: 'h-1.5',
  },
  md: { 
    icon: 'h-5 w-5', 
    text: 'text-base', 
    container: 'p-3',
    progress: 'h-2',
  },
  lg: { 
    icon: 'h-6 w-6', 
    text: 'text-lg', 
    container: 'p-4',
    progress: 'h-2.5',
  },
};

export const ActivityIndicator: React.FC<ActivityIndicatorProps> = ({
  type,
  status,
  label,
  description,
  size = 'md',
  variant = 'default',
  icon: customIcon,
  pulse: forcePulse = false,
  className,
  progress,
  eta,
  startTime,
  showDuration = false,
  metadata,
}) => {
  const [duration, setDuration] = useState<string>('');

  const config = activityConfig[type];
  const statusConf = statusConfig[status];
  const sizeConf = sizeConfig[size];
  const IconComponent = config.icon;

  // Calculate duration
  useEffect(() => {
    if (!showDuration || !startTime) return;

    const updateDuration = () => {
      const now = new Date();
      const elapsed = now.getTime() - startTime.getTime();
      const seconds = Math.floor(elapsed / 1000);
      const minutes = Math.floor(seconds / 60);
      const hours = Math.floor(minutes / 60);

      if (seconds < 60) {
        setDuration(`${seconds}s`);
      } else if (minutes < 60) {
        setDuration(`${minutes}m ${seconds % 60}s`);
      } else {
        setDuration(`${hours}h ${minutes % 60}m`);
      }
    };

    updateDuration();
    const interval = setInterval(updateDuration, 1000);
    return () => clearInterval(interval);
  }, [showDuration, startTime]);

  const shouldPulse = forcePulse || (status === 'active' && type !== 'monitoring');
  const shouldSpin = status === 'active' && ['loading', 'processing', 'syncing', 'connecting'].includes(type);

  // Compact variant
  if (variant === 'compact') {
    return (
      <div className={cn(
        'inline-flex items-center space-x-1.5 rounded-full px-2 py-1 text-xs font-medium',
        statusConf.bgColor,
        statusConf.color,
        className,
      )}>
        <div className={cn(sizeConf.icon, shouldPulse && 'animate-pulse')}>
          {customIcon || (
            <motion.div
              animate={shouldSpin ? { rotate: 360 } : {}}
              transition={shouldSpin ? { duration: 2, repeat: Infinity, ease: 'linear' } : {}}
            >
              <IconComponent className={sizeConf.icon} />
            </motion.div>
          )}
        </div>
        <span>{label || config.label}</span>
        {progress !== undefined && (
          <span className="font-mono">({Math.round(progress)}%)</span>
        )}
      </div>
    );
  }

  // Detailed variant
  if (variant === 'detailed') {
    return (
      <div className={cn(
        'rounded-lg border p-4 space-y-3',
        statusConf.bgColor,
        'border-gray-200 dark:border-gray-700',
        className,
      )}>
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className={cn(statusConf.color, shouldPulse && 'animate-pulse')}>
              {customIcon || (
                <motion.div
                  animate={shouldSpin ? { rotate: 360 } : {}}
                  transition={shouldSpin ? { duration: 2, repeat: Infinity, ease: 'linear' } : {}}
                >
                  <IconComponent className={sizeConf.icon} />
                </motion.div>
              )}
            </div>
            <div>
              <h4 className={cn('font-medium', statusConf.color, sizeConf.text)}>
                {label || config.label}
              </h4>
              {description && (
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {description}
                </p>
              )}
            </div>
          </div>
          
          {/* Status badge */}
          <span className={cn(
            'px-2 py-1 rounded-full text-xs font-medium capitalize',
            statusConf.color,
            statusConf.bgColor,
          )}>
            {status}
          </span>
        </div>

        {/* Progress bar */}
        {progress !== undefined && (
          <div className="space-y-1">
            <div className="flex justify-between text-sm text-gray-600 dark:text-gray-400">
              <span>Progress</span>
              <span className="font-mono">{Math.round(progress)}%</span>
            </div>
            <div className={cn('w-full bg-gray-200 dark:bg-gray-700 rounded-full', sizeConf.progress)}>
              <motion.div
                className={cn('h-full rounded-full', config.color.replace('text-', 'bg-'))}
                initial={{ width: 0 }}
                animate={{ width: `${progress}%` }}
                transition={{ duration: 0.3, ease: 'easeOut' }}
              />
            </div>
          </div>
        )}

        {/* Metadata */}
        {(metadata || eta || duration) && (
          <div className="grid grid-cols-2 gap-2 text-xs text-gray-600 dark:text-gray-400">
            {metadata?.itemsProcessed !== undefined && metadata?.totalItems !== undefined && (
              <div>
                <span className="font-medium">Items:</span>{' '}
                {metadata.itemsProcessed}/{metadata.totalItems}
              </div>
            )}
            {metadata?.speed && (
              <div>
                <span className="font-medium">Speed:</span> {metadata.speed}
              </div>
            )}
            {metadata?.throughput && (
              <div>
                <span className="font-medium">Throughput:</span> {metadata.throughput}
              </div>
            )}
            {eta && (
              <div>
                <span className="font-medium">ETA:</span> {eta}
              </div>
            )}
            {duration && (
              <div>
                <span className="font-medium">Duration:</span> {duration}
              </div>
            )}
          </div>
        )}
      </div>
    );
  }

  // Default variant
  return (
    <div className={cn(
      'inline-flex items-center space-x-2 rounded-lg border px-3 py-2',
      statusConf.bgColor,
      'border-gray-200 dark:border-gray-700',
      className,
    )}>
      <div className={cn(statusConf.color, shouldPulse && 'animate-pulse')}>
        {customIcon || (
          <motion.div
            animate={shouldSpin ? { rotate: 360 } : {}}
            transition={shouldSpin ? { duration: 2, repeat: Infinity, ease: 'linear' } : {}}
          >
            <IconComponent className={sizeConf.icon} />
          </motion.div>
        )}
      </div>
      
      <div className="flex-1 min-w-0">
        <div className={cn('font-medium', statusConf.color, sizeConf.text)}>
          {label || config.label}
        </div>
        {description && (
          <div className="text-xs text-gray-600 dark:text-gray-400 truncate">
            {description}
          </div>
        )}
      </div>

      {progress !== undefined && (
        <div className="text-xs font-mono text-gray-600 dark:text-gray-400">
          {Math.round(progress)}%
        </div>
      )}

      {eta && (
        <div className="text-xs text-gray-600 dark:text-gray-400">
          ETA: {eta}
        </div>
      )}
    </div>
  );
};

// Multi-activity indicator for showing multiple activities
export interface MultiActivityIndicatorProps {
  /** Array of activities */
  activities: (ActivityIndicatorProps & { id: string })[];
  /** Maximum number to show */
  maxShow?: number;
  /** Layout direction */
  direction?: 'vertical' | 'horizontal';
  /** Size for all indicators */
  size?: ActivityIndicatorProps['size'];
  /** Variant for all indicators */
  variant?: ActivityIndicatorProps['variant'];
  /** Custom className */
  className?: string;
  /** Show summary when collapsed */
  showSummary?: boolean;
}

export const MultiActivityIndicator: React.FC<MultiActivityIndicatorProps> = ({
  activities,
  maxShow = 3,
  direction = 'vertical',
  size = 'sm',
  variant = 'compact',
  className,
  showSummary = true,
}) => {
  const [showAll, setShowAll] = useState(false);

  const visibleActivities = showAll ? activities : activities.slice(0, maxShow);
  const hiddenCount = Math.max(0, activities.length - maxShow);

  const activeCounts = activities.reduce((acc, activity) => {
    acc[activity.status] = (acc[activity.status] || 0) + 1;
    return acc;
  }, {} as Record<ActivityStatus, number>);

  const containerClasses = direction === 'vertical' 
    ? 'flex flex-col space-y-2' 
    : 'flex flex-wrap gap-2';

  return (
    <div className={cn(containerClasses, className)}>
      <AnimatePresence mode="popLayout">
        {visibleActivities.map((activity) => (
          <motion.div
            key={activity.id}
            layout
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.9 }}
            transition={{ duration: 0.2 }}
          >
            <ActivityIndicator
              {...activity}
              size={size}
              variant={variant}
            />
          </motion.div>
        ))}
      </AnimatePresence>

      {/* Show more/less toggle */}
      {hiddenCount > 0 && (
        <button
          onClick={() => setShowAll(!showAll)}
          className="text-sm text-blue-600 dark:text-blue-400 hover:underline text-left"
        >
          {showAll ? 'Show less' : `Show ${hiddenCount} more activities`}
        </button>
      )}

      {/* Summary */}
      {showSummary && activities.length > 1 && (
        <div className="text-xs text-gray-600 dark:text-gray-400 border-t pt-2 mt-2">
          <div className="flex items-center space-x-4">
            {Object.entries(activeCounts).map(([status, count]) => (
              <span key={status} className="flex items-center space-x-1">
                <span className={cn('w-2 h-2 rounded-full', {
                  'bg-blue-500': status === 'active',
                  'bg-green-500': status === 'completed',
                  'bg-red-500': status === 'error',
                  'bg-yellow-500': status === 'paused',
                  'bg-gray-500': status === 'waiting',
                })} />
                <span>{count} {status}</span>
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

// Pulse indicator for simple status indication
export interface PulseIndicatorProps {
  /** Whether the indicator is active */
  active?: boolean;
  /** Size of the pulse */
  size?: 'xs' | 'sm' | 'md' | 'lg';
  /** Color variant */
  color?: 'blue' | 'green' | 'red' | 'yellow' | 'purple' | 'gray';
  /** Custom className */
  className?: string;
  /** Pulse speed multiplier */
  speed?: number;
}

export const PulseIndicator: React.FC<PulseIndicatorProps> = ({
  active = true,
  size = 'md',
  color = 'blue',
  className,
  speed = 1,
}) => {
  const sizeClasses = {
    xs: 'w-2 h-2',
    sm: 'w-3 h-3',
    md: 'w-4 h-4',
    lg: 'w-5 h-5',
  };

  const colorClasses = {
    blue: 'bg-blue-500',
    green: 'bg-green-500',
    red: 'bg-red-500',
    yellow: 'bg-yellow-500',
    purple: 'bg-purple-500',
    gray: 'bg-gray-500',
  };

  return (
    <div className={cn('relative', className)}>
      <div className={cn('rounded-full', sizeClasses[size], colorClasses[color])} />
      {active && (
        <motion.div
          className={cn('absolute inset-0 rounded-full opacity-75', colorClasses[color])}
          animate={{
            scale: [1, 2, 1],
            opacity: [0.75, 0, 0.75],
          }}
          transition={{
            duration: 2 / speed,
            repeat: Infinity,
            ease: 'easeOut',
          }}
        />
      )}
    </div>
  );
};

export default ActivityIndicator;