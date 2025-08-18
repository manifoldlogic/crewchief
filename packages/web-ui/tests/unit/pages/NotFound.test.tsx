/**
 * Tests for NotFound page component
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '../../utils/react-testing-utils';
import { BrowserRouter } from 'react-router-dom';
import NotFound from '../../../src/client/pages/NotFound';

// Mock react-router-dom's useLocation hook
const mockLocation = {
  pathname: '/non-existent-page',
  search: '',
  hash: '',
  state: null,
  key: 'test',
};

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom') as any;
  return {
    ...actual,
    useLocation: () => mockLocation,
  };
});

describe('NotFound', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Default Rendering', () => {
    it('renders the 404 page with default content', () => {
      render(<NotFound />);

      expect(screen.getByText('404')).toBeInTheDocument();
      expect(screen.getByText('Page Not Found')).toBeInTheDocument();
      expect(screen.getByText(/The page "\/non-existent-page" could not be found/)).toBeInTheDocument();
    });

    it('displays the search icon', () => {
      render(<NotFound />);

      expect(screen.getByLabelText('Not found')).toHaveTextContent('🔍');
    });

    it('shows helpful suggestions', () => {
      render(<NotFound />);

      expect(screen.getByText('What can you do?')).toBeInTheDocument();
      expect(screen.getByText('Check the URL for typos or spelling errors')).toBeInTheDocument();
      expect(screen.getByText('Use the navigation menu to find what you\'re looking for')).toBeInTheDocument();
      expect(screen.getByText('Return to the dashboard to explore available features')).toBeInTheDocument();
      expect(screen.getByText('Use the search feature to find specific content')).toBeInTheDocument();
    });
  });

  describe('Customization Props', () => {
    it('renders custom title', () => {
      render(<NotFound title="Custom Not Found" />);

      expect(screen.getByText('Custom Not Found')).toBeInTheDocument();
    });

    it('renders custom message', () => {
      const customMessage = 'This is a custom error message';
      render(<NotFound message={customMessage} />);

      expect(screen.getByText(customMessage)).toBeInTheDocument();
    });

    it('hides back button when showBackButton is false', () => {
      render(<NotFound showBackButton={false} />);

      expect(screen.queryByRole('button', { name: /go back/i })).not.toBeInTheDocument();
    });

    it('hides home button when showHomeButton is false', () => {
      render(<NotFound showHomeButton={false} />);

      expect(screen.queryByRole('link', { name: /go to dashboard/i })).not.toBeInTheDocument();
    });
  });

  describe('Navigation', () => {
    it('renders dashboard link', () => {
      render(<NotFound />);

      const dashboardLink = screen.getByRole('link', { name: /go to dashboard/i });
      expect(dashboardLink).toBeInTheDocument();
      expect(dashboardLink).toHaveAttribute('href', '/');
    });

    it('renders quick navigation links', () => {
      render(<NotFound />);

      expect(screen.getByRole('link', { name: 'Search' })).toHaveAttribute('href', '/search');
      expect(screen.getByRole('link', { name: 'Agents' })).toHaveAttribute('href', '/agents');
      expect(screen.getByRole('link', { name: 'Worktrees' })).toHaveAttribute('href', '/worktrees');
      expect(screen.getByRole('link', { name: 'Settings' })).toHaveAttribute('href', '/settings');
    });

    it('handles go back button click', () => {
      // Mock window.history
      const mockHistory = {
        length: 2,
        back: vi.fn(),
      };
      Object.defineProperty(window, 'history', {
        value: mockHistory,
        writable: true,
      });

      render(<NotFound />);

      const backButton = screen.getByRole('button', { name: /go back/i });
      fireEvent.click(backButton);

      expect(mockHistory.back).toHaveBeenCalledTimes(1);
    });

    it('redirects to home when history is empty', () => {
      // Mock window.history with length 1 (no previous page)
      const mockHistory = {
        length: 1,
        back: vi.fn(),
      };
      
      // Mock window.location
      const mockLocation = {
        href: '',
      };

      Object.defineProperty(window, 'history', {
        value: mockHistory,
        writable: true,
      });
      Object.defineProperty(window, 'location', {
        value: mockLocation,
        writable: true,
      });

      render(<NotFound />);

      const backButton = screen.getByRole('button', { name: /go back/i });
      fireEvent.click(backButton);

      expect(mockLocation.href).toBe('/');
    });
  });

  describe('Debug Information', () => {
    it('shows debug information in development mode', () => {
      // Mock import.meta.env.DEV
      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: true };

      render(<NotFound />);

      const debugDetails = screen.getByText('Debug Information');
      expect(debugDetails).toBeInTheDocument();

      // Click to expand debug details
      fireEvent.click(debugDetails);

      expect(screen.getByText('Pathname:')).toBeInTheDocument();
      expect(screen.getByText('/non-existent-page')).toBeInTheDocument();
      expect(screen.getByText('Search:')).toBeInTheDocument();
      expect(screen.getByText('Hash:')).toBeInTheDocument();
      expect(screen.getByText('State:')).toBeInTheDocument();
      expect(screen.getByText('Timestamp:')).toBeInTheDocument();

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });

    it('hides debug information in production mode', () => {
      // Mock import.meta.env.DEV
      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: false };

      render(<NotFound />);

      expect(screen.queryByText('Debug Information')).not.toBeInTheDocument();

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });

    it('displays location state in debug info when present', () => {
      // Mock location with state
      const mockLocationWithState = {
        ...mockLocation,
        state: { from: '/previous-page', reason: 'redirect' },
      };

      // Re-mock useLocation with state
      vi.doMock('react-router-dom', async () => {
        const actual = await vi.importActual('react-router-dom') as any;
        return {
          ...actual,
          useLocation: () => mockLocationWithState,
        };
      });

      // Mock development environment
      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: true };

      render(<NotFound />);

      const debugDetails = screen.getByText('Debug Information');
      fireEvent.click(debugDetails);

      expect(screen.getByText('State:')).toBeInTheDocument();
      expect(screen.getByText(JSON.stringify(mockLocationWithState.state))).toBeInTheDocument();

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });
  });

  describe('Location Information', () => {
    it('displays the current pathname in error message', () => {
      render(<NotFound />);

      expect(screen.getByText(/The page "\/non-existent-page" could not be found/)).toBeInTheDocument();
    });

    it('handles search parameters in location', () => {
      // Mock location with search params
      const mockLocationWithSearch = {
        ...mockLocation,
        search: '?param=value',
      };

      vi.doMock('react-router-dom', async () => {
        const actual = await vi.importActual('react-router-dom') as any;
        return {
          ...actual,
          useLocation: () => mockLocationWithSearch,
        };
      });

      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: true };

      render(<NotFound />);

      const debugDetails = screen.getByText('Debug Information');
      fireEvent.click(debugDetails);

      expect(screen.getByText('?param=value')).toBeInTheDocument();

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });

    it('handles hash in location', () => {
      // Mock location with hash
      const mockLocationWithHash = {
        ...mockLocation,
        hash: '#section',
      };

      vi.doMock('react-router-dom', async () => {
        const actual = await vi.importActual('react-router-dom') as any;
        return {
          ...actual,
          useLocation: () => mockLocationWithHash,
        };
      });

      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: true };

      render(<NotFound />);

      const debugDetails = screen.getByText('Debug Information');
      fireEvent.click(debugDetails);

      expect(screen.getByText('#section')).toBeInTheDocument();

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });
  });

  describe('Accessibility', () => {
    it('has proper heading hierarchy', () => {
      render(<NotFound />);

      const mainHeading = screen.getByRole('heading', { level: 1 });
      expect(mainHeading).toHaveTextContent('404');

      const subHeading = screen.getByRole('heading', { level: 2 });
      expect(subHeading).toHaveTextContent('Page Not Found');

      const sectionHeading = screen.getByRole('heading', { level: 3 });
      expect(sectionHeading).toHaveTextContent('What can you do?');
    });

    it('has descriptive link text', () => {
      render(<NotFound />);

      const dashboardLink = screen.getByRole('link', { name: /go to dashboard/i });
      expect(dashboardLink).toBeInTheDocument();

      const searchLink = screen.getByRole('link', { name: 'Search' });
      expect(searchLink).toBeInTheDocument();
    });
  });
});