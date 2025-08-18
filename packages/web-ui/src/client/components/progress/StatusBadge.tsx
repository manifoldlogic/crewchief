import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { cn } from '../../lib/utils';
import type { AgentStatusChange } from '../../contexts/websocket/types';

export type StatusType = 'online' | 'busy' | 'error' | 'offline' | 'idle' | 'running' | 'stopped' | 'connecting' | 'disconnected';

export interface StatusBadgeProps {
  /** Status type */
  status: StatusType;
  /** Optional custom label */
  label?: string;
  /** Badge size */
  size?: 'xs' | 'sm' | 'md' | 'lg';
  /** Show pulse animation */
  pulse?: boolean;
  /** Show status dot */
  showDot?: boolean;
  /** Custom className */
  className?: string;
  /** Accessibility label */
  'aria-label'?: string;
  /** Last update timestamp for showing freshness */
  lastUpdate?: Date;
  /** Show relative time since last update */
  showLastUpdate?: boolean;
  /** Click handler */
  onClick?: () => void;
}

interface StatusConfig {
  color: string;
  bgColor: string;
  borderColor: string;
  icon: string;
  label: string;
  dotColor: string;
  pulse: boolean;
}

export const StatusBadge: React.FC<StatusBadgeProps> = ({
  status,
  label,
  size = 'sm',
  pulse: forcePulse = false,
  showDot = true,
  className,
  'aria-label': ariaLabel,
  lastUpdate,
  showLastUpdate = false,
  onClick,
}) => {
  const [timeAgo, setTimeAgo] = useState<string>('');

  // Status configurations
  const statusConfigs: Record<StatusType, StatusConfig> = {
    online: {
      color: 'text-green-700 dark:text-green-300',
      bgColor: 'bg-green-50 dark:bg-green-900/20',
      borderColor: 'border-green-200 dark:border-green-800',
      dotColor: 'bg-green-500',
      icon: '●',
      label: 'Online',
      pulse: true,
    },
    busy: {
      color: 'text-orange-700 dark:text-orange-300',
      bgColor: 'bg-orange-50 dark:bg-orange-900/20',
      borderColor: 'border-orange-200 dark:border-orange-800',
      dotColor: 'bg-orange-500',
      icon: '⚡',
      label: 'Busy',
      pulse: true,
    },
    running: {
      color: 'text-blue-700 dark:text-blue-300',
      bgColor: 'bg-blue-50 dark:bg-blue-900/20',
      borderColor: 'border-blue-200 dark:border-blue-800',
      dotColor: 'bg-blue-500',
      icon: '▶️',
      label: 'Running',
      pulse: true,
    },
    error: {
      color: 'text-red-700 dark:text-red-300',
      bgColor: 'bg-red-50 dark:bg-red-900/20',
      borderColor: 'border-red-200 dark:border-red-800',
      dotColor: 'bg-red-500',
      icon: '❌',
      label: 'Error',
      pulse: false,
    },
    offline: {
      color: 'text-gray-700 dark:text-gray-400',
      bgColor: 'bg-gray-50 dark:bg-gray-900/20',
      borderColor: 'border-gray-200 dark:border-gray-700',
      dotColor: 'bg-gray-400',
      icon: '●',
      label: 'Offline',
      pulse: false,
    },
    idle: {
      color: 'text-gray-700 dark:text-gray-400',
      bgColor: 'bg-gray-50 dark:bg-gray-900/20',
      borderColor: 'border-gray-200 dark:border-gray-700',
      dotColor: 'bg-gray-400',
      icon: '⏸️',
      label: 'Idle',
      pulse: false,
    },
    stopped: {
      color: 'text-yellow-700 dark:text-yellow-300',
      bgColor: 'bg-yellow-50 dark:bg-yellow-900/20',
      borderColor: 'border-yellow-200 dark:border-yellow-800',
      dotColor: 'bg-yellow-500',
      icon: '⏹️',
      label: 'Stopped',
      pulse: false,
    },
    connecting: {
      color: 'text-blue-700 dark:text-blue-300',
      bgColor: 'bg-blue-50 dark:bg-blue-900/20',
      borderColor: 'border-blue-200 dark:border-blue-800',
      dotColor: 'bg-blue-500',
      icon: '🔄',
      label: 'Connecting',
      pulse: true,
    },
    disconnected: {
      color: 'text-red-700 dark:text-red-300',
      bgColor: 'bg-red-50 dark:bg-red-900/20',
      borderColor: 'border-red-200 dark:border-red-800',
      dotColor: 'bg-red-500',
      icon: '🔌',
      label: 'Disconnected',
      pulse: false,
    },
  };

  // Size configurations
  const sizeConfigs = {
    xs: {
      container: 'px-1.5 py-0.5 text-xs',
      dot: 'w-1.5 h-1.5',
      icon: 'text-xs',
    },
    sm: {
      container: 'px-2 py-1 text-xs',
      dot: 'w-2 h-2',
      icon: 'text-sm',
    },
    md: {
      container: 'px-2.5 py-1 text-sm',
      dot: 'w-2.5 h-2.5',
      icon: 'text-base',
    },
    lg: {
      container: 'px-3 py-1.5 text-base',
      dot: 'w-3 h-3',
      icon: 'text-lg',
    },
  };

  const config = statusConfigs[status];
  const sizeConfig = sizeConfigs[size];

  // Update time ago
  useEffect(() => {
    if (!lastUpdate || !showLastUpdate) return;

    const updateTimeAgo = () => {
      const now = new Date();
      const diff = now.getTime() - lastUpdate.getTime();
      const seconds = Math.floor(diff / 1000);
      const minutes = Math.floor(seconds / 60);
      const hours = Math.floor(minutes / 60);
      const days = Math.floor(hours / 24);

      if (seconds < 30) {
        setTimeAgo('now');
      } else if (seconds < 60) {
        setTimeAgo(`${seconds}s`);
      } else if (minutes < 60) {
        setTimeAgo(`${minutes}m`);
      } else if (hours < 24) {
        setTimeAgo(`${hours}h`);
      } else {
        setTimeAgo(`${days}d`);
      }
    };

    updateTimeAgo();
    const interval = setInterval(updateTimeAgo, 1000);
    return () => clearInterval(interval);
  }, [lastUpdate, showLastUpdate]);

  const shouldPulse = forcePulse || config.pulse;
  const displayLabel = label || config.label;

  return (
    <motion.div
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full border font-medium transition-all duration-200',
        config.color,
        config.bgColor,
        config.borderColor,
        sizeConfig.container,
        onClick && 'cursor-pointer hover:shadow-sm',
        className,
      )}
      onClick={onClick}
      role={onClick ? 'button' : undefined}
      aria-label={ariaLabel || `Status: ${displayLabel}`}
      initial={{ scale: 0.95, opacity: 0 }}
      animate={{ scale: 1, opacity: 1 }}
      transition={{ duration: 0.2 }}
      whileHover={onClick ? { scale: 1.05 } : undefined}
      whileTap={onClick ? { scale: 0.95 } : undefined}
    >
      {/* Status dot */}
      {showDot && (
        <div className="relative">
          <div
            className={cn(
              'rounded-full',
              config.dotColor,
              sizeConfig.dot,
            )}
          />
          {shouldPulse && (
            <motion.div
              className={cn(
                'absolute inset-0 rounded-full opacity-75',
                config.dotColor,
              )}
              animate={{
                scale: [1, 1.5, 1],
                opacity: [0.75, 0, 0.75],
              }}
              transition={{
                duration: 2,
                repeat: Infinity,
                ease: 'easeInOut',
              }}
            />
          )}
        </div>
      )}

      {/* Status icon */}
      <span className={cn(sizeConfig.icon)}>
        {config.icon}
      </span>

      {/* Status label */}
      <span>{displayLabel}</span>

      {/* Time ago indicator */}
      {showLastUpdate && timeAgo && (
        <span className="opacity-60 text-xs">
          · {timeAgo}
        </span>
      )}
    </motion.div>
  );
};

// Agent-specific status badge with WebSocket integration
export interface AgentStatusBadgeProps extends Omit<StatusBadgeProps, 'status'> {
  /** Agent data */
  agent: AgentStatusChange;
  /** Update frequency threshold in ms */
  updateThreshold?: number;
}

export const AgentStatusBadge: React.FC<AgentStatusBadgeProps> = ({
  agent,
  updateThreshold = 100,
  ...props
}) => {
  const [isRecentUpdate, setIsRecentUpdate] = useState(false);

  // Map agent status to badge status
  const mapAgentStatus = (agentStatus: AgentStatusChange['status']): StatusType => {
    switch (agentStatus) {
      case 'running':
        return 'running';
      case 'idle':
        return 'idle';
      case 'error':
        return 'error';
      case 'stopped':
        return 'offline';
      default:
        return 'offline';
    }
  };

  // Show recent update indicator
  useEffect(() => {
    setIsRecentUpdate(true);
    const timer = setTimeout(() => setIsRecentUpdate(false), updateThreshold);
    return () => clearTimeout(timer);
  }, [agent.lastActive, updateThreshold]);

  const status = mapAgentStatus(agent.status);
  const lastUpdate = new Date(agent.lastActive);

  return (
    <div className="relative">
      <StatusBadge
        status={status}
        label={`${agent.name} (${agent.type})`}
        lastUpdate={lastUpdate}
        showLastUpdate
        pulse={isRecentUpdate}
        {...props}
      />
      
      {/* Recent update flash */}
      <AnimatePresence>
        {isRecentUpdate && (
          <motion.div
            className="absolute inset-0 rounded-full bg-white/20 pointer-events-none"
            initial={{ opacity: 0 }}
            animate={{ opacity: [0, 0.5, 0] }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.3 }}
          />
        )}
      </AnimatePresence>
    </div>
  );
};

// Multi-status indicator for showing multiple statuses at once
export interface MultiStatusProps {
  /** Array of statuses to display */
  statuses: Array<{
    status: StatusType;
    label?: string;
    count?: number;
  }>;
  /** Layout orientation */
  orientation?: 'horizontal' | 'vertical';
  /** Maximum number of statuses to show */
  maxShow?: number;
  /** Size for all badges */
  size?: StatusBadgeProps['size'];
  /** Custom className */
  className?: string;
}

export const MultiStatus: React.FC<MultiStatusProps> = ({
  statuses,
  orientation = 'horizontal',
  maxShow = 5,
  size = 'sm',
  className,
}) => {
  const visibleStatuses = statuses.slice(0, maxShow);
  const remainingCount = Math.max(0, statuses.length - maxShow);

  const containerClasses = orientation === 'horizontal' 
    ? 'flex flex-wrap gap-1' 
    : 'flex flex-col gap-1';

  return (
    <div className={cn(containerClasses, className)}>
      {visibleStatuses.map((statusItem, index) => (
        <StatusBadge
          key={index}
          status={statusItem.status}
          label={statusItem.count ? `${statusItem.label} (${statusItem.count})` : statusItem.label}
          size={size}
        />
      ))}
      
      {remainingCount > 0 && (
        <StatusBadge
          status="offline"
          label={`+${remainingCount} more`}
          size={size}
          showDot={false}
        />
      )}
    </div>
  );
};

export default StatusBadge;