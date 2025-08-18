/**
 * User-friendly error display component
 */

import React from 'react';
import type { ErrorDisplayProps, AppError } from '@types/error';

interface ErrorDisplayState {
  isExpanded: boolean;
}

export class ErrorDisplay extends React.Component<ErrorDisplayProps, ErrorDisplayState> {
  constructor(props: ErrorDisplayProps) {
    super(props);
    this.state = {
      isExpanded: false,
    };
  }

  private toggleExpanded = () => {
    this.setState(prevState => ({
      isExpanded: !prevState.isExpanded,
    }));
  };

  private handleRetry = () => {
    this.props.onRetry?.();
  };

  private handleDismiss = () => {
    this.props.onDismiss?.();
  };

  private getErrorIcon(error: AppError): string {
    if ('status' in error && error.status) {
      if (error.status >= 500) return '🚨';
      if (error.status === 404) return '🔍';
      if (error.status >= 400) return '⚠️';
    }
    
    if ('isOffline' in error && error.isOffline) return '🌐';
    if ('timeout' in error && error.timeout) return '⏱️';
    if ('componentStack' in error) return '⚛️';
    
    return '❌';
  }

  private getUserFriendlyMessage(error: AppError): string {
    // Provide user-friendly messages while hiding sensitive details
    if ('status' in error && error.status) {
      switch (error.status) {
        case 400:
          return 'The request was invalid. Please check your input and try again.';
        case 401:
          return 'You need to sign in to access this resource.';
        case 403:
          return 'You don\'t have permission to access this resource.';
        case 404:
          return 'The requested resource could not be found.';
        case 408:
          return 'The request timed out. Please try again.';
        case 429:
          return 'Too many requests. Please wait a moment and try again.';
        case 500:
          return 'A server error occurred. Our team has been notified.';
        case 502:
        case 503:
        case 504:
          return 'The service is temporarily unavailable. Please try again later.';
        default:
          return 'An unexpected error occurred. Please try again.';
      }
    }

    if ('isOffline' in error && error.isOffline) {
      return 'You appear to be offline. Please check your internet connection.';
    }

    if ('timeout' in error && error.timeout) {
      return 'The request timed out. Please check your connection and try again.';
    }

    if ('componentStack' in error) {
      return 'A problem occurred while loading this page. Please refresh and try again.';
    }

    // Fallback to original message (sanitized)
    return error.message || 'An unexpected error occurred.';
  }

  private getVariantClasses(): string {
    const { variant = 'inline' } = this.props;
    
    const baseClasses = 'rounded-lg border p-4 shadow-sm';
    
    switch (variant) {
      case 'modal':
        return `${baseClasses} bg-white border-red-200 max-w-md mx-auto`;
      case 'toast':
        return `${baseClasses} bg-red-50 border-red-200 fixed top-4 right-4 z-50 max-w-sm`;
      case 'inline':
      default:
        return `${baseClasses} bg-red-50 border-red-200`;
    }
  }

  render(): React.ReactNode {
    const { error, className = '', variant = 'inline' } = this.props;
    const { isExpanded } = this.state;

    const containerClasses = `${this.getVariantClasses()} ${className}`;
    const icon = this.getErrorIcon(error);
    const message = this.getUserFriendlyMessage(error);

    return (
      <div className={containerClasses} role="alert" aria-live="polite">
        <div className="flex items-start gap-3">
          <div className="flex-shrink-0">
            <span className="text-xl" role="img" aria-label="Error">
              {icon}
            </span>
          </div>
          
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1">
                <h3 className="text-sm font-medium text-red-800">
                  Something went wrong
                </h3>
                <p className="mt-1 text-sm text-red-700">
                  {message}
                </p>
              </div>
              
              {variant === 'toast' && this.props.onDismiss && (
                <button
                  onClick={this.handleDismiss}
                  className="flex-shrink-0 text-red-400 hover:text-red-600 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
                  aria-label="Dismiss error"
                >
                  <span className="sr-only">Dismiss</span>
                  <svg className="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                    <path
                      fillRule="evenodd"
                      d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                      clipRule="evenodd"
                    />
                  </svg>
                </button>
              )}
            </div>

            {/* Action buttons */}
            <div className="mt-3 flex flex-wrap gap-2">
              {this.props.onRetry && (
                <button
                  onClick={this.handleRetry}
                  className="inline-flex items-center px-3 py-1.5 text-xs font-medium text-red-700 bg-red-100 border border-red-300 rounded-md hover:bg-red-200 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
                >
                  Try Again
                </button>
              )}
              
              <button
                onClick={this.toggleExpanded}
                className="inline-flex items-center px-3 py-1.5 text-xs font-medium text-red-700 bg-transparent border border-red-300 rounded-md hover:bg-red-50 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
              >
                {isExpanded ? 'Less Details' : 'More Details'}
              </button>
            </div>

            {/* Expandable error details */}
            {isExpanded && (
              <div className="mt-3 p-3 bg-red-100 rounded-md">
                <details className="text-xs">
                  <summary className="font-medium text-red-800 cursor-pointer">
                    Technical Details
                  </summary>
                  <div className="mt-2 space-y-1">
                    {error.code && (
                      <div>
                        <span className="font-medium">Code:</span> {error.code}
                      </div>
                    )}
                    {error.timestamp && (
                      <div>
                        <span className="font-medium">Time:</span> {error.timestamp.toLocaleString()}
                      </div>
                    )}
                    {'status' in error && error.status && (
                      <div>
                        <span className="font-medium">Status:</span> {error.status}
                      </div>
                    )}
                    {'endpoint' in error && error.endpoint && (
                      <div>
                        <span className="font-medium">Endpoint:</span> {error.endpoint}
                      </div>
                    )}
                    {'method' in error && error.method && (
                      <div>
                        <span className="font-medium">Method:</span> {error.method}
                      </div>
                    )}
                  </div>
                </details>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }
}

export default ErrorDisplay;