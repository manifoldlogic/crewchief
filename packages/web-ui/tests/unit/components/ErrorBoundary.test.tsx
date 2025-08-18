/**
 * Tests for ErrorBoundary component
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '../../utils/react-testing-utils';
import ErrorBoundary, { withErrorBoundary } from '../../../src/client/components/ErrorBoundary';
import type { ReactError } from '../../../src/client/types/error';

// Mock the error logger
vi.mock('../../../src/client/utils/errorLogger', () => ({
  logReactError: vi.fn(),
}));

// Component that throws an error for testing
const ThrowingComponent: React.FC<{ shouldThrow?: boolean }> = ({ shouldThrow = true }) => {
  if (shouldThrow) {
    throw new Error('Test error from component');
  }
  return <div>Component rendered successfully</div>;
};

// Component that can be toggled to throw errors
const ToggleThrowingComponent: React.FC<{ throwError: boolean }> = ({ throwError }) => {
  if (throwError) {
    throw new Error('Toggled error');
  }
  return <div>Normal component</div>;
};

describe('ErrorBoundary', () => {
  let consoleError: any;

  beforeEach(() => {
    // Mock console.error to prevent error output during tests
    consoleError = console.error;
    console.error = vi.fn();
  });

  afterEach(() => {
    console.error = consoleError;
    vi.clearAllMocks();
  });

  describe('Normal Operation', () => {
    it('renders children when no error occurs', () => {
      render(
        <ErrorBoundary>
          <div>Normal content</div>
        </ErrorBoundary>
      );

      expect(screen.getByText('Normal content')).toBeInTheDocument();
    });

    it('does not show error UI when children render successfully', () => {
      render(
        <ErrorBoundary>
          <ThrowingComponent shouldThrow={false} />
        </ErrorBoundary>
      );

      expect(screen.getByText('Component rendered successfully')).toBeInTheDocument();
      expect(screen.queryByText('Oops! Something went wrong')).not.toBeInTheDocument();
    });
  });

  describe('Error Handling', () => {
    it('catches errors and displays error UI', () => {
      render(
        <ErrorBoundary>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      expect(screen.getByText('Oops! Something went wrong')).toBeInTheDocument();
      expect(screen.getByText('A problem occurred while rendering this page. Don\'t worry, this has been reported to our team.')).toBeInTheDocument();
    });

    it('shows the error message in error display', () => {
      render(
        <ErrorBoundary>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      // Click to expand details
      fireEvent.click(screen.getByRole('button', { name: /more details/i }));

      expect(screen.getByText('Test error from component')).toBeInTheDocument();
    });

    it('calls custom onError callback when error occurs', () => {
      const onError = vi.fn();

      render(
        <ErrorBoundary onError={onError}>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      expect(onError).toHaveBeenCalledTimes(1);
      expect(onError).toHaveBeenCalledWith(
        expect.any(Error),
        expect.objectContaining({
          componentStack: expect.any(String),
        })
      );
    });

    it('logs error using error logger', async () => {
      const { logReactError } = await import('../../../src/client/utils/errorLogger');

      render(
        <ErrorBoundary>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      expect(logReactError).toHaveBeenCalledTimes(1);
      expect(logReactError).toHaveBeenCalledWith(
        expect.any(Error),
        expect.objectContaining({
          componentStack: expect.any(String),
        }),
        expect.objectContaining({
          props: expect.any(Array),
          location: expect.any(String),
        })
      );
    });
  });

  describe('Reset Functionality', () => {
    it('resets error state when Try Again button is clicked', () => {
      const { rerender } = render(
        <ErrorBoundary>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      // Error should be displayed
      expect(screen.getByText('Oops! Something went wrong')).toBeInTheDocument();

      // Click Try Again
      fireEvent.click(screen.getByRole('button', { name: /try again/i }));

      // Re-render with non-throwing component
      rerender(
        <ErrorBoundary>
          <ThrowingComponent shouldThrow={false} />
        </ErrorBoundary>
      );

      expect(screen.getByText('Component rendered successfully')).toBeInTheDocument();
      expect(screen.queryByText('Oops! Something went wrong')).not.toBeInTheDocument();
    });

    it('resets on props change when resetOnPropsChange is enabled', () => {
      let throwError = true;

      const { rerender } = render(
        <ErrorBoundary resetOnPropsChange>
          <ToggleThrowingComponent throwError={throwError} />
        </ErrorBoundary>
      );

      // Error should be displayed
      expect(screen.getByText('Oops! Something went wrong')).toBeInTheDocument();

      // Change props to fix the error
      throwError = false;
      rerender(
        <ErrorBoundary resetOnPropsChange>
          <ToggleThrowingComponent throwError={throwError} />
        </ErrorBoundary>
      );

      expect(screen.getByText('Normal component')).toBeInTheDocument();
      expect(screen.queryByText('Oops! Something went wrong')).not.toBeInTheDocument();
    });

    it('reloads page when Reload Page button is clicked', () => {
      // Mock window.location.reload
      const originalReload = window.location.reload;
      window.location.reload = vi.fn();

      render(
        <ErrorBoundary>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      fireEvent.click(screen.getByRole('button', { name: /reload page/i }));

      expect(window.location.reload).toHaveBeenCalledTimes(1);

      // Restore original method
      window.location.reload = originalReload;
    });
  });

  describe('Custom Fallback', () => {
    it('uses custom fallback component when provided', () => {
      const CustomFallback: React.FC<{ error: ReactError; onReset: () => void }> = ({ error, onReset }) => (
        <div>
          <h1>Custom Error UI</h1>
          <p>Error: {error.message}</p>
          <button onClick={onReset}>Custom Reset</button>
        </div>
      );

      render(
        <ErrorBoundary fallback={CustomFallback}>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      expect(screen.getByText('Custom Error UI')).toBeInTheDocument();
      expect(screen.getByText('Error: Test error from component')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /custom reset/i })).toBeInTheDocument();
    });

    it('calls onReset from custom fallback', () => {
      const CustomFallback: React.FC<{ error: ReactError; onReset: () => void }> = ({ onReset }) => (
        <button onClick={onReset}>Custom Reset</button>
      );

      const { rerender } = render(
        <ErrorBoundary fallback={CustomFallback}>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      fireEvent.click(screen.getByRole('button', { name: /custom reset/i }));

      rerender(
        <ErrorBoundary fallback={CustomFallback}>
          <ThrowingComponent shouldThrow={false} />
        </ErrorBoundary>
      );

      expect(screen.getByText('Component rendered successfully')).toBeInTheDocument();
    });
  });

  describe('Error Context Information', () => {
    it('includes component stack in error info', () => {
      render(
        <ErrorBoundary>
          <div>
            <div>
              <ThrowingComponent />
            </div>
          </div>
        </ErrorBoundary>
      );

      // Click to expand details
      fireEvent.click(screen.getByRole('button', { name: /more details/i }));

      // Should show technical details with component stack
      expect(screen.getByText('Technical Details')).toBeInTheDocument();
    });

    it('marks isolated error boundaries in logs', async () => {
      const { logReactError } = await import('../../../src/client/utils/errorLogger');

      render(
        <ErrorBoundary isolate>
          <ThrowingComponent />
        </ErrorBoundary>
      );

      expect(logReactError).toHaveBeenCalledWith(
        expect.any(Error),
        expect.objectContaining({
          componentStack: expect.any(String),
        }),
        expect.objectContaining({
          props: '[isolated]',
        })
      );
    });
  });

  describe('withErrorBoundary HOC', () => {
    it('wraps component with error boundary', () => {
      const TestComponent: React.FC = () => <div>Test Component</div>;
      const WrappedComponent = withErrorBoundary(TestComponent);

      render(<WrappedComponent />);

      expect(screen.getByText('Test Component')).toBeInTheDocument();
    });

    it('catches errors in wrapped component', () => {
      const WrappedThrowingComponent = withErrorBoundary(ThrowingComponent);

      render(<WrappedThrowingComponent />);

      expect(screen.getByText('Oops! Something went wrong')).toBeInTheDocument();
    });

    it('passes props through to wrapped component', () => {
      const TestComponent: React.FC<{ message: string }> = ({ message }) => <div>{message}</div>;
      const WrappedComponent = withErrorBoundary(TestComponent);

      render(<WrappedComponent message="Hello World" />);

      expect(screen.getByText('Hello World')).toBeInTheDocument();
    });

    it('sets correct displayName', () => {
      const TestComponent: React.FC = () => <div>Test</div>;
      TestComponent.displayName = 'TestComponent';
      
      const WrappedComponent = withErrorBoundary(TestComponent);

      expect(WrappedComponent.displayName).toBe('withErrorBoundary(TestComponent)');
    });

    it('handles anonymous components', () => {
      const WrappedComponent = withErrorBoundary(() => <div>Anonymous</div>);

      expect(WrappedComponent.displayName).toMatch(/withErrorBoundary\(/);
    });
  });
});