/**
 * Error logging utility with environment-specific handling
 */

import type { AppError, ErrorLogEntry, ErrorSeverity } from '@types/error';

class ErrorLogger {
  private isDevelopment = import.meta.env.DEV;
  private logQueue: ErrorLogEntry[] = [];

  /**
   * Logs an error with appropriate handling based on environment
   */
  log(error: AppError, severity: ErrorSeverity = 'medium', context?: Record<string, unknown>): void {
    const logEntry = this.createLogEntry(error, severity, context);

    if (this.isDevelopment) {
      this.logToDevelopmentConsole(logEntry);
    } else {
      this.logToProduction(logEntry);
    }

    // Add to queue for potential batch processing
    this.logQueue.push(logEntry);
    
    // Prevent memory leaks by limiting queue size
    if (this.logQueue.length > 100) {
      this.logQueue.shift();
    }
  }

  /**
   * Logs a React error boundary error
   */
  logReactError(error: Error, errorInfo: { componentStack?: string }, context?: Record<string, unknown>): void {
    const appError: AppError = {
      message: error.message,
      code: 'REACT_ERROR',
      timestamp: new Date(),
      componentStack: errorInfo.componentStack,
      errorBoundary: 'ErrorBoundary',
    };

    this.log(appError, 'high', {
      ...context,
      stack: error.stack,
      name: error.name,
    });
  }

  /**
   * Logs an API error
   */
  logAPIError(error: Error, endpoint?: string, method?: string, status?: number): void {
    const appError: AppError = {
      message: error.message,
      code: 'API_ERROR',
      timestamp: new Date(),
      status,
      endpoint,
      method,
    };

    this.log(appError, this.getAPISeverity(status), {
      stack: error.stack,
      name: error.name,
    });
  }

  /**
   * Logs a network error
   */
  logNetworkError(error: Error, isOffline = false, timeout = false): void {
    const appError: AppError = {
      message: error.message,
      code: 'NETWORK_ERROR',
      timestamp: new Date(),
      isOffline,
      timeout,
    };

    this.log(appError, 'high', {
      stack: error.stack,
      name: error.name,
    });
  }

  /**
   * Creates a structured log entry
   */
  private createLogEntry(
    error: AppError,
    severity: ErrorSeverity,
    context?: Record<string, unknown>
  ): ErrorLogEntry {
    return {
      id: this.generateId(),
      error: {
        ...error,
        // Sanitize error message for production
        message: this.isDevelopment ? error.message : this.sanitizeErrorMessage(error.message),
      },
      userAgent: navigator.userAgent,
      url: window.location.href,
      sessionId: this.getSessionId(),
      environment: this.isDevelopment ? 'development' : 'production',
      severity,
      ...context,
    };
  }

  /**
   * Logs to development console with rich formatting
   */
  private logToDevelopmentConsole(logEntry: ErrorLogEntry): void {
    const { error, severity } = logEntry;
    
    const style = this.getConsoleStyle(severity);
    
    console.group(`%c[${severity.toUpperCase()}] ${error.code || 'ERROR'}`, style);
    console.error('Message:', error.message);
    console.error('Error:', error);
    console.error('Full Log Entry:', logEntry);
    console.groupEnd();
  }

  /**
   * Logs to production (in a real app, this would send to a logging service)
   */
  private logToProduction(logEntry: ErrorLogEntry): void {
    // In development, just log to console
    // In production, this would send to a logging service like Sentry, LogRocket, etc.
    console.error('Production Error:', {
      id: logEntry.id,
      message: logEntry.error.message,
      code: logEntry.error.code,
      severity: logEntry.severity,
      timestamp: logEntry.error.timestamp,
      url: logEntry.url,
    });
  }

  /**
   * Gets console styling based on severity
   */
  private getConsoleStyle(severity: ErrorSeverity): string {
    const styles = {
      low: 'background: #3b82f6; color: white; padding: 2px 4px; border-radius: 2px;',
      medium: 'background: #f59e0b; color: white; padding: 2px 4px; border-radius: 2px;',
      high: 'background: #ef4444; color: white; padding: 2px 4px; border-radius: 2px;',
      critical: 'background: #7c2d12; color: white; padding: 2px 4px; border-radius: 2px; font-weight: bold;',
    };
    
    return styles[severity];
  }

  /**
   * Determines API error severity based on status code
   */
  private getAPISeverity(status?: number): ErrorSeverity {
    if (!status) return 'medium';
    
    if (status >= 500) return 'critical';
    if (status >= 400) return 'high';
    if (status >= 300) return 'medium';
    return 'low';
  }

  /**
   * Sanitizes error messages for production to avoid exposing sensitive information
   */
  private sanitizeErrorMessage(message: string): string {
    // Remove potential sensitive information
    return message
      .replace(/password=\w+/gi, 'password=***')
      .replace(/token=[\w-]+/gi, 'token=***')
      .replace(/key=[\w-]+/gi, 'key=***')
      .replace(/secret=[\w-]+/gi, 'secret=***');
  }

  /**
   * Generates a unique ID for log entries
   */
  private generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Gets or creates a session ID
   */
  private getSessionId(): string {
    let sessionId = sessionStorage.getItem('crewchief_session_id');
    if (!sessionId) {
      sessionId = this.generateId();
      sessionStorage.setItem('crewchief_session_id', sessionId);
    }
    return sessionId;
  }

  /**
   * Gets recent error logs (useful for debugging)
   */
  getRecentLogs(limit = 10): ErrorLogEntry[] {
    return this.logQueue.slice(-limit);
  }

  /**
   * Clears the error log queue
   */
  clearLogs(): void {
    this.logQueue = [];
  }
}

// Export singleton instance
export const errorLogger = new ErrorLogger();

// Export utilities
export const logError = (error: AppError, severity?: ErrorSeverity, context?: Record<string, unknown>) => {
  errorLogger.log(error, severity, context);
};

export const logReactError = (error: Error, errorInfo: { componentStack?: string }, context?: Record<string, unknown>) => {
  errorLogger.logReactError(error, errorInfo, context);
};

export const logAPIError = (error: Error, endpoint?: string, method?: string, status?: number) => {
  errorLogger.logAPIError(error, endpoint, method, status);
};

export const logNetworkError = (error: Error, isOffline?: boolean, timeout?: boolean) => {
  errorLogger.logNetworkError(error, isOffline, timeout);
};