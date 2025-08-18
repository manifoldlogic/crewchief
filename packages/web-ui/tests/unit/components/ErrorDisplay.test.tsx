/**
 * Tests for ErrorDisplay component
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '../../utils/react-testing-utils';
import ErrorDisplay from '../../../src/client/components/ErrorDisplay';
import type { AppError } from '../../../src/client/types/error';

describe('ErrorDisplay', () => {
  const mockOnRetry = vi.fn();
  const mockOnDismiss = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Error Display', () => {
    it('renders error message correctly', () => {
      const error: AppError = {
        message: 'Test error message',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      render(<ErrorDisplay error={error} />);

      expect(screen.getByText('Something went wrong')).toBeInTheDocument();
      expect(screen.getByText('Test error message')).toBeInTheDocument();
    });

    it('displays appropriate icon for different error types', () => {
      const apiError: AppError = {
        message: 'API Error',
        code: 'API_ERROR',
        timestamp: new Date(),
        status: 500,
      };

      render(<ErrorDisplay error={apiError} />);
      
      // Should display the critical error icon (🚨) for 500 errors
      expect(screen.getByLabelText('Error')).toHaveTextContent('🚨');
    });

    it('shows user-friendly message for API errors', () => {
      const error: AppError = {
        message: 'Internal server error details',
        code: 'API_ERROR',
        timestamp: new Date(),
        status: 500,
      };

      render(<ErrorDisplay error={error} />);

      expect(screen.getByText('A server error occurred. Our team has been notified.')).toBeInTheDocument();
    });

    it('shows user-friendly message for network errors', () => {
      const error: AppError = {
        message: 'Network error',
        code: 'NETWORK_ERROR',
        timestamp: new Date(),
        isOffline: true,
      };

      render(<ErrorDisplay error={error} />);

      expect(screen.getByText('You appear to be offline. Please check your internet connection.')).toBeInTheDocument();
    });
  });

  describe('Interactive Features', () => {
    it('calls onRetry when retry button is clicked', async () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      render(<ErrorDisplay error={error} onRetry={mockOnRetry} />);

      const retryButton = screen.getByRole('button', { name: /try again/i });
      fireEvent.click(retryButton);

      expect(mockOnRetry).toHaveBeenCalledTimes(1);
    });

    it('calls onDismiss when dismiss button is clicked (toast variant)', async () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      render(<ErrorDisplay error={error} variant="toast" onDismiss={mockOnDismiss} />);

      const dismissButton = screen.getByLabelText('Dismiss error');
      fireEvent.click(dismissButton);

      expect(mockOnDismiss).toHaveBeenCalledTimes(1);
    });

    it('toggles details expansion when more details button is clicked', async () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
        status: 404,
        endpoint: '/api/test',
        method: 'GET',
      };

      render(<ErrorDisplay error={error} />);

      const detailsButton = screen.getByRole('button', { name: /more details/i });
      fireEvent.click(detailsButton);

      await waitFor(() => {
        expect(screen.getByText('Technical Details')).toBeInTheDocument();
        expect(screen.getByText('Code:')).toBeInTheDocument();
        expect(screen.getByText('TEST_ERROR')).toBeInTheDocument();
        expect(screen.getByText('Status:')).toBeInTheDocument();
        expect(screen.getByText('404')).toBeInTheDocument();
        expect(screen.getByText('Endpoint:')).toBeInTheDocument();
        expect(screen.getByText('/api/test')).toBeInTheDocument();
      });

      // Should now show "Less Details"
      expect(screen.getByRole('button', { name: /less details/i })).toBeInTheDocument();
    });
  });

  describe('Variants', () => {
    it('applies correct CSS classes for inline variant', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      const { container } = render(<ErrorDisplay error={error} variant="inline" />);
      const errorElement = container.firstChild as HTMLElement;

      expect(errorElement).toHaveClass('bg-red-50', 'border-red-200');
    });

    it('applies correct CSS classes for toast variant', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      const { container } = render(<ErrorDisplay error={error} variant="toast" />);
      const errorElement = container.firstChild as HTMLElement;

      expect(errorElement).toHaveClass('fixed', 'top-4', 'right-4', 'z-50');
    });

    it('applies correct CSS classes for modal variant', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      const { container } = render(<ErrorDisplay error={error} variant="modal" />);
      const errorElement = container.firstChild as HTMLElement;

      expect(errorElement).toHaveClass('bg-white', 'border-red-200', 'max-w-md', 'mx-auto');
    });

    it('applies custom className', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      const { container } = render(<ErrorDisplay error={error} className="custom-class" />);
      const errorElement = container.firstChild as HTMLElement;

      expect(errorElement).toHaveClass('custom-class');
    });
  });

  describe('Accessibility', () => {
    it('has proper ARIA attributes', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      render(<ErrorDisplay error={error} />);

      const errorElement = screen.getByRole('alert');
      expect(errorElement).toHaveAttribute('aria-live', 'polite');
    });

    it('has proper button labels', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      render(<ErrorDisplay error={error} variant="toast" onDismiss={mockOnDismiss} />);

      expect(screen.getByLabelText('Dismiss error')).toBeInTheDocument();
    });
  });

  describe('Error Type Specific Behavior', () => {
    it('handles 404 errors correctly', () => {
      const error: AppError = {
        message: 'Not found',
        code: 'API_ERROR',
        timestamp: new Date(),
        status: 404,
      };

      render(<ErrorDisplay error={error} />);

      expect(screen.getByText('The requested resource could not be found.')).toBeInTheDocument();
      expect(screen.getByLabelText('Error')).toHaveTextContent('🔍');
    });

    it('handles timeout errors correctly', () => {
      const error: AppError = {
        message: 'Request timeout',
        code: 'NETWORK_ERROR',
        timestamp: new Date(),
        timeout: true,
      };

      render(<ErrorDisplay error={error} />);

      expect(screen.getByText('The request timed out. Please check your connection and try again.')).toBeInTheDocument();
      expect(screen.getByLabelText('Error')).toHaveTextContent('⏱️');
    });

    it('handles React component errors correctly', () => {
      const error: AppError = {
        message: 'Component error',
        code: 'REACT_ERROR',
        timestamp: new Date(),
        componentStack: 'in Component (at App.js:10)',
      };

      render(<ErrorDisplay error={error} />);

      expect(screen.getByText('A problem occurred while loading this page. Please refresh and try again.')).toBeInTheDocument();
      expect(screen.getByLabelText('Error')).toHaveTextContent('⚛️');
    });
  });

  describe('Error Details', () => {
    it('shows all available error details when expanded', async () => {
      const timestamp = new Date('2023-01-01T12:00:00Z');
      const error: AppError = {
        message: 'Test error',
        code: 'API_ERROR',
        timestamp,
        status: 500,
        endpoint: '/api/test',
        method: 'POST',
      };

      render(<ErrorDisplay error={error} />);

      fireEvent.click(screen.getByRole('button', { name: /more details/i }));

      await waitFor(() => {
        expect(screen.getByText('Code:')).toBeInTheDocument();
        expect(screen.getByText('API_ERROR')).toBeInTheDocument();
        expect(screen.getByText('Time:')).toBeInTheDocument();
        expect(screen.getByText(timestamp.toLocaleString())).toBeInTheDocument();
        expect(screen.getByText('Status:')).toBeInTheDocument();
        expect(screen.getByText('500')).toBeInTheDocument();
        expect(screen.getByText('Endpoint:')).toBeInTheDocument();
        expect(screen.getByText('/api/test')).toBeInTheDocument();
        expect(screen.getByText('Method:')).toBeInTheDocument();
        expect(screen.getByText('POST')).toBeInTheDocument();
      });
    });

    it('hides details section when collapsed', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      render(<ErrorDisplay error={error} />);

      expect(screen.queryByText('Technical Details')).not.toBeInTheDocument();
    });
  });
});