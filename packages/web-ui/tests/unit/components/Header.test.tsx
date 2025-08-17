import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@test-utils';
import userEvent from '@testing-library/user-event';
import Header from '../../../src/client/components/Header.js';

// Mock react-router-dom
const mockUseLocation = vi.fn();
vi.mock('react-router-dom', () => ({
  useLocation: () => mockUseLocation(),
}));

describe('Header', () => {
  const mockOnMenuClick = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseLocation.mockReturnValue({ pathname: '/' });
  });

  it('renders header with dashboard title by default', () => {
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    expect(screen.getByRole('heading', { name: 'Dashboard' })).toBeInTheDocument();
  });

  it('displays correct page titles for different routes', () => {
    const routes = [
      { pathname: '/', expectedTitle: 'Dashboard' },
      { pathname: '/search', expectedTitle: 'Search' },
      { pathname: '/worktrees', expectedTitle: 'Worktrees' },
      { pathname: '/agents', expectedTitle: 'Agents' },
      { pathname: '/settings', expectedTitle: 'Settings' },
      { pathname: '/unknown', expectedTitle: 'CrewChief' },
    ];

    routes.forEach(({ pathname, expectedTitle }) => {
      mockUseLocation.mockReturnValue({ pathname });
      
      const { rerender } = render(<Header onMenuClick={mockOnMenuClick} />);
      
      expect(screen.getByRole('heading', { name: expectedTitle })).toBeInTheDocument();
      
      rerender(<div />); // Clear the component
    });
  });

  it('calls onMenuClick when menu button is clicked', async () => {
    const user = userEvent.setup();
    
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    const menuButton = screen.getByRole('button', { name: /open sidebar/i });
    await user.click(menuButton);
    
    expect(mockOnMenuClick).toHaveBeenCalledTimes(1);
  });

  it('has accessible menu button', () => {
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    const menuButton = screen.getByRole('button', { name: /open sidebar/i });
    
    expect(menuButton).toBeInTheDocument();
    expect(menuButton).toHaveAttribute('type', 'button');
    expect(menuButton.querySelector('svg')).toBeInTheDocument();
  });

  it('has accessible theme toggle button', () => {
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    const themeButton = screen.getByRole('button', { name: /toggle theme/i });
    
    expect(themeButton).toBeInTheDocument();
    expect(themeButton).toHaveAttribute('type', 'button');
    expect(themeButton).toHaveAttribute('title', 'Toggle theme');
    expect(themeButton.querySelector('svg')).toBeInTheDocument();
  });

  it('displays online status indicator', () => {
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    expect(screen.getByText('Online')).toBeInTheDocument();
    
    // Check for the green status dot (by class or color)
    const statusContainer = screen.getByText('Online').closest('div');
    expect(statusContainer?.querySelector('.bg-green-400')).toBeInTheDocument();
  });

  it('has proper CSS classes for styling', () => {
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    const header = screen.getByRole('banner');
    expect(header).toHaveClass('relative', 'z-10', 'flex-shrink-0', 'flex', 'h-16');
  });

  it('has responsive menu button that is only visible on mobile', () => {
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    const menuButton = screen.getByRole('button', { name: /open sidebar/i });
    expect(menuButton).toHaveClass('md:hidden');
  });

  it('supports keyboard navigation', async () => {
    const user = userEvent.setup();
    
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    const menuButton = screen.getByRole('button', { name: /open sidebar/i });
    const themeButton = screen.getByRole('button', { name: /toggle theme/i });
    
    // Tab to menu button
    await user.tab();
    expect(menuButton).toHaveFocus();
    
    // Tab to theme button
    await user.tab();
    expect(themeButton).toHaveFocus();
    
    // Enter key should activate the focused button
    await user.keyboard('{Enter}');
    // Note: Theme button doesn't have an onClick handler yet, so we can't test its activation
  });

  it('has proper focus states for accessibility', () => {
    render(<Header onMenuClick={mockOnMenuClick} />);
    
    const menuButton = screen.getByRole('button', { name: /open sidebar/i });
    const themeButton = screen.getByRole('button', { name: /toggle theme/i });
    
    expect(menuButton).toHaveClass('focus:outline-none', 'focus:ring-2');
    expect(themeButton).toHaveClass('focus:outline-none', 'focus:ring-2');
  });

  describe('Dark mode support', () => {
    it('has dark mode classes', () => {
      render(<Header onMenuClick={mockOnMenuClick} />);
      
      const header = screen.getByRole('banner');
      expect(header).toHaveClass('dark:bg-gray-800');
      
      const title = screen.getByRole('heading');
      expect(title).toHaveClass('dark:text-white');
    });
  });
});