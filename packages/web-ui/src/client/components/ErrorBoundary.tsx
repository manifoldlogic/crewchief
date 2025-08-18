/**
 * Error Boundary component for catching React errors
 */

import React from 'react';
import type { ErrorBoundaryState, ReactError } from '@types/error';
import { logReactError } from '@utils/errorLogger';
import ErrorDisplay from './ErrorDisplay';

interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ComponentType<{ error: ReactError; onReset: () => void }>;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
  resetOnPropsChange?: boolean;
  isolate?: boolean;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  private resetTimeoutId: number | null = null;

  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
    };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo): void {
    // Log the error
    logReactError(error, errorInfo, {
      props: this.props.isolate ? '[isolated]' : Object.keys(this.props),
      location: window.location.href,
    });

    // Call custom error handler if provided
    this.props.onError?.(error, errorInfo);

    // Update state with error info
    this.setState({
      errorInfo: {
        componentStack: errorInfo.componentStack,
        errorBoundary: 'ErrorBoundary',
      },
    });
  }

  componentDidUpdate(prevProps: ErrorBoundaryProps): void {
    const { resetOnPropsChange } = this.props;
    const { hasError } = this.state;

    // Reset error state when props change (if enabled)
    if (hasError && resetOnPropsChange && prevProps.children !== this.props.children) {
      this.resetErrorBoundary();
    }
  }

  componentWillUnmount(): void {
    if (this.resetTimeoutId) {
      window.clearTimeout(this.resetTimeoutId);
    }
  }

  private resetErrorBoundary = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  private handleRetry = (): void => {
    this.resetErrorBoundary();
  };

  private createReactError(): ReactError {
    const { error, errorInfo } = this.state;
    
    return {
      message: error?.message || 'An unexpected error occurred in the React component tree',
      code: 'REACT_ERROR',
      timestamp: new Date(),
      componentStack: errorInfo?.componentStack,
      errorBoundary: errorInfo?.errorBoundary,
    };
  }

  render(): React.ReactNode {
    const { hasError } = this.state;
    const { children, fallback: Fallback } = this.props;

    if (hasError) {
      const reactError = this.createReactError();

      // Use custom fallback if provided
      if (Fallback) {
        return <Fallback error={reactError} onReset={this.handleRetry} />;
      }

      // Default error UI
      return (
        <div className="min-h-screen flex items-center justify-center bg-gray-50 px-4">
          <div className="max-w-lg w-full">
            <div className="text-center mb-6">
              <div className="mx-auto h-16 w-16 flex items-center justify-center rounded-full bg-red-100 mb-4">
                <span className="text-2xl" role="img" aria-label="Error">
                  ⚛️
                </span>
              </div>
              <h1 className="text-2xl font-bold text-gray-900 mb-2">
                Oops! Something went wrong
              </h1>
              <p className="text-gray-600">
                A problem occurred while rendering this page. Don't worry, this has been reported to our team.
              </p>
            </div>

            <ErrorDisplay
              error={reactError}
              onRetry={this.handleRetry}
              variant="inline"
              className="mb-6"
            />

            <div className="text-center space-y-3">
              <button
                onClick={this.handleRetry}
                className="inline-flex items-center px-4 py-2 text-sm font-medium text-white bg-primary-600 border border-transparent rounded-md shadow-sm hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
              >
                Try Again
              </button>
              
              <div>
                <button
                  onClick={() => window.location.reload()}
                  className="inline-flex items-center px-4 py-2 text-sm font-medium text-primary-700 bg-primary-100 border border-primary-300 rounded-md hover:bg-primary-200 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
                >
                  Reload Page
                </button>
              </div>
              
              <div>
                <a
                  href="/"
                  className="inline-flex items-center px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
                >
                  Go to Dashboard
                </a>
              </div>
            </div>
          </div>
        </div>
      );
    }

    return children;
  }
}

/**
 * Higher-order component to wrap components with error boundary
 */
export function withErrorBoundary<P extends object>(
  Component: React.ComponentType<P>,
  errorBoundaryProps?: Omit<ErrorBoundaryProps, 'children'>
): React.ComponentType<P> {
  const WrappedComponent = (props: P) => (
    <ErrorBoundary {...errorBoundaryProps}>
      <Component {...props} />
    </ErrorBoundary>
  );

  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name})`;

  return WrappedComponent;
}

export default ErrorBoundary;