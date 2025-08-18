/**
 * Error handling types for the CrewChief Web UI
 */

export interface BaseError {
  message: string;
  code?: string;
  timestamp: Date;
}

export interface APIError extends BaseError {
  status?: number;
  statusText?: string;
  endpoint?: string;
  method?: string;
}

export interface ValidationError extends BaseError {
  field?: string;
  value?: unknown;
}

export interface NetworkError extends BaseError {
  isOffline?: boolean;
  timeout?: boolean;
}

export interface ReactError extends BaseError {
  componentStack?: string;
  errorBoundary?: string;
}

export type AppError = APIError | ValidationError | NetworkError | ReactError;

export interface ErrorInfo {
  componentStack?: string;
  errorBoundary?: string;
}

export interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export interface ErrorDisplayProps {
  error: AppError;
  onRetry?: () => void;
  onDismiss?: () => void;
  variant?: 'inline' | 'modal' | 'toast';
  className?: string;
}

export interface ErrorLogEntry {
  id: string;
  error: AppError;
  userAgent?: string;
  url?: string;
  userId?: string;
  sessionId?: string;
  environment: 'development' | 'production';
  severity: 'low' | 'medium' | 'high' | 'critical';
}

export enum ErrorSeverity {
  Low = 'low',
  Medium = 'medium',
  High = 'high',
  Critical = 'critical',
}

export enum ErrorCategory {
  Network = 'network',
  API = 'api',
  Validation = 'validation',
  Runtime = 'runtime',
  Permission = 'permission',
  NotFound = 'not-found',
}