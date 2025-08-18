import React, { useEffect, useRef, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X, CheckCircle, AlertCircle, AlertTriangle, Info, ExternalLink } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Button } from '../ui/button';

export interface ToastAction {
  label: string;
  onClick: () => void;
  variant?: 'default' | 'destructive' | 'outline' | 'secondary';
  icon?: React.ReactNode;
}

export interface ProgressToastProps {
  /** Unique identifier */
  id: string;
  /** Toast title */
  title: string;
  /** Toast description */
  description?: string;
  /** Toast type */
  type?: 'info' | 'success' | 'warning' | 'error' | 'loading';
  /** Progress value (0-100) for loading toasts */
  progress?: number;
  /** Auto dismiss duration in milliseconds (0 = no auto dismiss) */
  duration?: number;
  /** Whether the toast is persistent (user must dismiss) */
  persistent?: boolean;
  /** Actions to show */
  actions?: ToastAction[];
  /** Custom icon */
  icon?: React.ReactNode;
  /** Whether to show close button */
  closable?: boolean;
  /** Custom className */
  className?: string;
  /** Callback when toast is dismissed */
  onDismiss?: () => void;
  /** Callback when auto-dismiss timer updates */
  onTimerUpdate?: (remaining: number) => void;
  /** Additional metadata to display */
  metadata?: {
    timestamp?: Date;
    source?: string;
    link?: {
      url: string;
      label: string;
    };
  };
}

const typeConfigs = {
  info: {
    icon: Info,
    color: 'text-blue-600 dark:text-blue-400',
    bg: 'bg-blue-50 dark:bg-blue-900/20',
    border: 'border-blue-200 dark:border-blue-800',
    progressColor: 'bg-blue-500',
  },
  success: {
    icon: CheckCircle,
    color: 'text-green-600 dark:text-green-400',
    bg: 'bg-green-50 dark:bg-green-900/20',
    border: 'border-green-200 dark:border-green-800',
    progressColor: 'bg-green-500',
  },
  warning: {
    icon: AlertTriangle,
    color: 'text-yellow-600 dark:text-yellow-400',
    bg: 'bg-yellow-50 dark:bg-yellow-900/20',
    border: 'border-yellow-200 dark:border-yellow-800',
    progressColor: 'bg-yellow-500',
  },
  error: {
    icon: AlertCircle,
    color: 'text-red-600 dark:text-red-400',
    bg: 'bg-red-50 dark:bg-red-900/20',
    border: 'border-red-200 dark:border-red-800',
    progressColor: 'bg-red-500',
  },
  loading: {
    icon: null,
    color: 'text-blue-600 dark:text-blue-400',
    bg: 'bg-blue-50 dark:bg-blue-900/20',
    border: 'border-blue-200 dark:border-blue-800',
    progressColor: 'bg-blue-500',
  },
};

export const ProgressToast: React.FC<ProgressToastProps> = ({
  id,
  title,
  description,
  type = 'info',
  progress,
  duration = 5000,
  persistent = false,
  actions = [],
  icon: customIcon,
  closable = true,
  className,
  onDismiss,
  onTimerUpdate,
  metadata,
}) => {
  const [isVisible, setIsVisible] = useState(true);
  const [timeRemaining, setTimeRemaining] = useState(duration);
  const [isPaused, setIsPaused] = useState(false);
  const timerRef = useRef<NodeJS.Timeout>();
  const startTimeRef = useRef(Date.now());

  const config = typeConfigs[type];
  const IconComponent = config.icon;

  // Auto-dismiss timer
  useEffect(() => {
    if (persistent || duration === 0 || !isVisible) return;

    const tick = () => {
      const elapsed = Date.now() - startTimeRef.current;
      const remaining = Math.max(0, duration - elapsed);
      
      setTimeRemaining(remaining);
      onTimerUpdate?.(remaining);

      if (remaining === 0) {
        handleDismiss();
      }
    };

    if (!isPaused) {
      timerRef.current = setInterval(tick, 100);
    }

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
    };
  }, [duration, persistent, isVisible, isPaused, onTimerUpdate]);

  // Pause timer on hover
  const handleMouseEnter = () => {
    if (!persistent && duration > 0) {
      setIsPaused(true);
    }
  };

  const handleMouseLeave = () => {
    if (!persistent && duration > 0) {
      setIsPaused(false);
      startTimeRef.current = Date.now() - (duration - timeRemaining);
    }
  };

  const handleDismiss = () => {
    setIsVisible(false);
    setTimeout(() => onDismiss?.(), 300); // Allow exit animation
  };

  const handleActionClick = (action: ToastAction) => {
    action.onClick();
    if (!persistent) {
      handleDismiss();
    }
  };

  const formatTimeRemaining = (ms: number): string => {
    const seconds = Math.ceil(ms / 1000);
    return `${seconds}s`;
  };

  const progressPercentage = duration > 0 && !persistent 
    ? ((duration - timeRemaining) / duration) * 100 
    : progress || 0;

  return (
    <AnimatePresence>
      {isVisible && (
        <motion.div
          layout
          initial={{ opacity: 0, y: 50, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: -50, scale: 0.95 }}
          transition={{ duration: 0.3, ease: 'easeOut' }}
          className={cn(
            'relative w-full max-w-md rounded-lg border p-4 shadow-lg backdrop-blur-sm',
            config.bg,
            config.border,
            className,
          )}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
          role="alert"
          aria-live="polite"
          aria-describedby={`toast-${id}-description`}
        >
          {/* Progress bar for auto-dismiss or loading */}
          {((duration > 0 && !persistent) || type === 'loading') && (
            <div className="absolute top-0 left-0 right-0 h-1 bg-gray-200 dark:bg-gray-700 rounded-t-lg overflow-hidden">
              <motion.div
                className={cn('h-full', config.progressColor)}
                initial={{ width: '0%' }}
                animate={{ width: `${progressPercentage}%` }}
                transition={{ 
                  duration: type === 'loading' ? 0.3 : 0.1,
                  ease: 'easeOut'
                }}
              />
            </div>
          )}

          {/* Header */}
          <div className="flex items-start space-x-3">
            {/* Icon */}
            <div className={cn('flex-shrink-0 mt-0.5', config.color)}>
              {customIcon || (IconComponent && <IconComponent className="h-5 w-5" />) || (
                type === 'loading' && (
                  <motion.div
                    className="h-5 w-5 border-2 border-current border-t-transparent rounded-full"
                    animate={{ rotate: 360 }}
                    transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
                  />
                )
              )}
            </div>

            {/* Content */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                  {title}
                </h4>
                
                <div className="flex items-center space-x-1">
                  {/* Timer indicator */}
                  {duration > 0 && !persistent && !isPaused && (
                    <span className="text-xs text-gray-500 dark:text-gray-400 font-mono">
                      {formatTimeRemaining(timeRemaining)}
                    </span>
                  )}
                  
                  {/* Close button */}
                  {closable && (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={handleDismiss}
                      className="h-6 w-6 p-0 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                      aria-label="Dismiss notification"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  )}
                </div>
              </div>

              {/* Description */}
              {description && (
                <p id={`toast-${id}-description`} className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                  {description}
                </p>
              )}

              {/* Metadata */}
              {metadata && (
                <div className="mt-2 flex items-center space-x-4 text-xs text-gray-500 dark:text-gray-400">
                  {metadata.timestamp && (
                    <span>{metadata.timestamp.toLocaleTimeString()}</span>
                  )}
                  {metadata.source && (
                    <span>Source: {metadata.source}</span>
                  )}
                  {metadata.link && (
                    <a
                      href={metadata.link.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="inline-flex items-center space-x-1 text-blue-600 dark:text-blue-400 hover:underline"
                    >
                      <span>{metadata.link.label}</span>
                      <ExternalLink className="h-3 w-3" />
                    </a>
                  )}
                </div>
              )}

              {/* Actions */}
              {actions.length > 0 && (
                <div className="mt-3 flex items-center space-x-2">
                  {actions.map((action, index) => (
                    <Button
                      key={index}
                      size="sm"
                      variant={action.variant || 'outline'}
                      onClick={() => handleActionClick(action)}
                      className="h-7 text-xs"
                    >
                      {action.icon && (
                        <span className="mr-1">{action.icon}</span>
                      )}
                      {action.label}
                    </Button>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Pause indicator */}
          <AnimatePresence>
            {isPaused && duration > 0 && !persistent && (
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="absolute top-2 right-2 text-xs text-gray-500 dark:text-gray-400 bg-white dark:bg-gray-800 px-1 rounded"
              >
                Paused
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      )}
    </AnimatePresence>
  );
};

// Toast container component
export interface ToastContainerProps {
  /** Toast position */
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left' | 'top-center' | 'bottom-center';
  /** Maximum number of toasts to show */
  maxToasts?: number;
  /** Custom className */
  className?: string;
  /** Array of toasts to display */
  toasts: ProgressToastProps[];
  /** Callback when a toast is dismissed */
  onDismissToast?: (id: string) => void;
}

export const ToastContainer: React.FC<ToastContainerProps> = ({
  position = 'top-right',
  maxToasts = 5,
  className,
  toasts,
  onDismissToast,
}) => {
  const positionClasses = {
    'top-right': 'top-4 right-4',
    'top-left': 'top-4 left-4',
    'bottom-right': 'bottom-4 right-4',
    'bottom-left': 'bottom-4 left-4',
    'top-center': 'top-4 left-1/2 transform -translate-x-1/2',
    'bottom-center': 'bottom-4 left-1/2 transform -translate-x-1/2',
  };

  const visibleToasts = toasts.slice(-maxToasts);

  return (
    <div
      className={cn(
        'fixed z-50 flex flex-col space-y-2 pointer-events-none',
        positionClasses[position],
        className,
      )}
      aria-live="polite"
      aria-label="Notifications"
    >
      <AnimatePresence mode="popLayout">
        {visibleToasts.map((toast) => (
          <div key={toast.id} className="pointer-events-auto">
            <ProgressToast
              {...toast}
              onDismiss={() => onDismissToast?.(toast.id)}
            />
          </div>
        ))}
      </AnimatePresence>
    </div>
  );
};

// Hook for managing toasts
export interface UseProgressToastReturn {
  toasts: ProgressToastProps[];
  showToast: (toast: Omit<ProgressToastProps, 'id'>) => string;
  dismissToast: (id: string) => void;
  dismissAll: () => void;
  updateToast: (id: string, updates: Partial<ProgressToastProps>) => void;
}

export const useProgressToast = (): UseProgressToastReturn => {
  const [toasts, setToasts] = useState<ProgressToastProps[]>([]);

  const showToast = (toast: Omit<ProgressToastProps, 'id'>): string => {
    const id = `toast-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const newToast: ProgressToastProps = {
      ...toast,
      id,
    };

    setToasts(prev => [...prev, newToast]);
    return id;
  };

  const dismissToast = (id: string) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  };

  const dismissAll = () => {
    setToasts([]);
  };

  const updateToast = (id: string, updates: Partial<ProgressToastProps>) => {
    setToasts(prev => prev.map(toast => 
      toast.id === id ? { ...toast, ...updates } : toast
    ));
  };

  return {
    toasts,
    showToast,
    dismissToast,
    dismissAll,
    updateToast,
  };
};

export default ProgressToast;