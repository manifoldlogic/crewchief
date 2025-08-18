import React, { useEffect, useState, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { cn } from '../../lib/utils';

export interface ProgressBarProps {
  /** Current progress value (0-100) */
  value: number;
  /** Maximum value (default: 100) */
  max?: number;
  /** Optional label text */
  label?: string;
  /** Show percentage text */
  showPercentage?: boolean;
  /** Show ETA calculation */
  showEta?: boolean;
  /** Progress bar size */
  size?: 'sm' | 'md' | 'lg';
  /** Visual variant */
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  /** Whether to animate progress changes */
  animated?: boolean;
  /** Animation duration in seconds */
  animationDuration?: number;
  /** Whether the operation is indeterminate */
  indeterminate?: boolean;
  /** Custom className */
  className?: string;
  /** Accessibility label */
  'aria-label'?: string;
  /** Show pulse animation when active */
  pulse?: boolean;
  /** Start time for ETA calculation */
  startTime?: Date;
  /** Current step/total steps for stepped progress */
  steps?: {
    current: number;
    total: number;
  };
}

interface EtaCalculation {
  remaining: string;
  completion: Date;
  speed: number; // progress per second
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  value,
  max = 100,
  label,
  showPercentage = true,
  showEta = false,
  size = 'md',
  variant = 'default',
  animated = true,
  animationDuration = 0.5,
  indeterminate = false,
  className,
  'aria-label': ariaLabel,
  pulse = false,
  startTime,
  steps,
}) => {
  const [eta, setEta] = useState<EtaCalculation | null>(null);
  const progressHistory = useRef<Array<{ value: number; timestamp: number }>>([]);
  const prevValueRef = useRef(value);

  // Calculate percentage
  const percentage = Math.min(Math.max((value / max) * 100, 0), 100);
  
  // Size configurations
  const sizeConfig = {
    sm: {
      height: 'h-1',
      text: 'text-xs',
      padding: 'px-2 py-1',
    },
    md: {
      height: 'h-2',
      text: 'text-sm',
      padding: 'px-3 py-1.5',
    },
    lg: {
      height: 'h-3',
      text: 'text-base',
      padding: 'px-4 py-2',
    },
  };

  // Variant configurations
  const variantConfig = {
    default: {
      bg: 'bg-primary',
      glow: 'shadow-primary/20',
      text: 'text-primary',
    },
    success: {
      bg: 'bg-green-500',
      glow: 'shadow-green-500/20',
      text: 'text-green-600',
    },
    warning: {
      bg: 'bg-yellow-500',
      glow: 'shadow-yellow-500/20',
      text: 'text-yellow-600',
    },
    error: {
      bg: 'bg-red-500',
      glow: 'shadow-red-500/20',
      text: 'text-red-600',
    },
    info: {
      bg: 'bg-blue-500',
      glow: 'shadow-blue-500/20',
      text: 'text-blue-600',
    },
  };

  const config = {
    ...sizeConfig[size],
    ...variantConfig[variant],
  };

  // ETA calculation
  useEffect(() => {
    if (!showEta || !startTime || indeterminate) {
      setEta(null);
      return;
    }

    const now = Date.now();
    const elapsed = (now - startTime.getTime()) / 1000; // seconds

    // Store progress history for smooth ETA calculation
    progressHistory.current.push({ value: percentage, timestamp: now });
    
    // Keep only recent history (last 30 seconds)
    const cutoff = now - 30000;
    progressHistory.current = progressHistory.current.filter(p => p.timestamp > cutoff);

    if (progressHistory.current.length < 2 || percentage === 0) {
      setEta(null);
      return;
    }

    // Calculate average speed from recent history
    const firstPoint = progressHistory.current[0];
    const lastPoint = progressHistory.current[progressHistory.current.length - 1];
    const timeDiff = (lastPoint.timestamp - firstPoint.timestamp) / 1000;
    const progressDiff = lastPoint.value - firstPoint.value;

    if (timeDiff === 0 || progressDiff <= 0) {
      setEta(null);
      return;
    }

    const speed = progressDiff / timeDiff; // percentage per second
    const remaining = (100 - percentage) / speed; // seconds remaining

    if (remaining <= 0 || !isFinite(remaining)) {
      setEta(null);
      return;
    }

    const completion = new Date(Date.now() + remaining * 1000);

    // Format remaining time
    const formatTime = (seconds: number): string => {
      if (seconds < 60) return `${Math.round(seconds)}s`;
      if (seconds < 3600) return `${Math.round(seconds / 60)}m ${Math.round(seconds % 60)}s`;
      const hours = Math.floor(seconds / 3600);
      const minutes = Math.round((seconds % 3600) / 60);
      return `${hours}h ${minutes}m`;
    };

    setEta({
      remaining: formatTime(remaining),
      completion,
      speed,
    });
  }, [percentage, showEta, startTime, indeterminate]);

  // Reset history when value changes significantly (new operation)
  useEffect(() => {
    if (Math.abs(value - prevValueRef.current) > 10) {
      progressHistory.current = [];
    }
    prevValueRef.current = value;
  }, [value]);

  const progressId = React.useId();

  return (
    <div className={cn('w-full', className)}>
      {/* Label and info section */}
      {(label || showPercentage || showEta || steps) && (
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center space-x-2">
            {label && (
              <span className={cn(config.text, config.text, 'font-medium')}>
                {label}
              </span>
            )}
            {steps && (
              <span className="text-xs text-muted-foreground">
                ({steps.current}/{steps.total})
              </span>
            )}
          </div>
          <div className="flex items-center space-x-3 text-xs text-muted-foreground">
            {showPercentage && !indeterminate && (
              <span className="font-mono">
                {Math.round(percentage)}%
              </span>
            )}
            {eta && (
              <span className="flex items-center space-x-1">
                <span>ETA:</span>
                <span className="font-mono">{eta.remaining}</span>
              </span>
            )}
          </div>
        </div>
      )}

      {/* Progress bar container */}
      <div
        className={cn(
          'relative w-full rounded-full bg-secondary overflow-hidden',
          config.height,
        )}
        role="progressbar"
        aria-valuenow={indeterminate ? undefined : value}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-label={ariaLabel || label || 'Progress'}
        aria-describedby={eta ? `${progressId}-eta` : undefined}
      >
        {/* Progress fill */}
        <AnimatePresence>
          {!indeterminate ? (
            <motion.div
              className={cn(
                'h-full rounded-full',
                config.bg,
                pulse && 'animate-pulse',
              )}
              initial={{ width: 0 }}
              animate={{ 
                width: `${percentage}%`,
                boxShadow: pulse ? `0 0 20px ${config.glow}` : undefined,
              }}
              transition={{
                duration: animated ? animationDuration : 0,
                ease: 'easeOut',
              }}
              style={{
                boxShadow: percentage > 0 && pulse ? `0 0 10px var(--shadow-color)` : undefined,
              }}
            />
          ) : (
            <motion.div
              className={cn(
                'h-full rounded-full opacity-75',
                config.bg,
              )}
              initial={{ x: '-100%' }}
              animate={{ x: '100%' }}
              transition={{
                duration: 1.5,
                ease: 'easeInOut',
                repeat: Infinity,
                repeatType: 'loop',
              }}
              style={{ width: '50%' }}
            />
          )}
        </AnimatePresence>

        {/* Shimmer effect for active progress */}
        {percentage > 0 && percentage < 100 && animated && (
          <motion.div
            className="absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent"
            initial={{ x: '-100%' }}
            animate={{ x: '200%' }}
            transition={{
              duration: 2,
              ease: 'easeInOut',
              repeat: Infinity,
              repeatDelay: 1,
            }}
          />
        )}
      </div>

      {/* ETA details */}
      {eta && (
        <div id={`${progressId}-eta`} className="mt-1 text-xs text-muted-foreground">
          <div className="flex justify-between">
            <span>Estimated completion:</span>
            <span className="font-mono">
              {eta.completion.toLocaleTimeString()}
            </span>
          </div>
        </div>
      )}
    </div>
  );
};

// Specialized progress bar for file operations
export interface FileProgressBarProps extends Omit<ProgressBarProps, 'label' | 'steps'> {
  /** Current file being processed */
  currentFile?: string;
  /** Files processed/total files */
  filesProgress?: {
    processed: number;
    total: number;
  };
  /** Processing speed (files per second) */
  speed?: number;
}

export const FileProgressBar: React.FC<FileProgressBarProps> = ({
  currentFile,
  filesProgress,
  speed,
  ...props
}) => {
  const formatSpeed = (filesPerSec: number): string => {
    if (filesPerSec < 1) return `${(filesPerSec * 60).toFixed(1)} files/min`;
    return `${filesPerSec.toFixed(1)} files/sec`;
  };

  return (
    <div className="space-y-2">
      <ProgressBar
        {...props}
        label="File Processing"
        steps={filesProgress ? {
          current: filesProgress.processed,
          total: filesProgress.total,
        } : undefined}
      />
      
      {(currentFile || speed) && (
        <div className="flex justify-between text-xs text-muted-foreground">
          {currentFile && (
            <span className="truncate max-w-[60%]" title={currentFile}>
              Processing: {currentFile}
            </span>
          )}
          {speed && (
            <span className="font-mono">
              {formatSpeed(speed)}
            </span>
          )}
        </div>
      )}
    </div>
  );
};

export default ProgressBar;