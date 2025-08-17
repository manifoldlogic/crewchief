import React, { ReactElement } from 'react';
import { render, RenderOptions } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { vi } from 'vitest';

/**
 * Custom render function that includes common providers
 */
interface CustomRenderOptions extends Omit<RenderOptions, 'wrapper'> {
  initialEntries?: string[];
  withRouter?: boolean;
}

const AllTheProviders: React.FC<{ children: React.ReactNode; initialEntries?: string[] }> = ({ 
  children, 
  initialEntries = ['/'] 
}) => {
  return (
    <BrowserRouter>
      {children}
    </BrowserRouter>
  );
};

export function customRender(
  ui: ReactElement,
  options: CustomRenderOptions = {}
) {
  const { 
    initialEntries = ['/'], 
    withRouter = true, 
    ...renderOptions 
  } = options;

  const Wrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    if (withRouter) {
      return <AllTheProviders initialEntries={initialEntries}>{children}</AllTheProviders>;
    }
    return <>{children}</>;
  };

  return render(ui, { wrapper: Wrapper, ...renderOptions });
}

/**
 * Mock fetch with common response patterns
 */
export const mockFetch = {
  success: (data: any, status = 200) => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status,
      json: async () => data,
      text: async () => JSON.stringify(data),
    });
  },
  
  error: (message = 'Network error', status = 500) => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status,
      json: async () => ({ error: message }),
      text: async () => JSON.stringify({ error: message }),
    });
  },
  
  networkError: () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('Network error'));
  },
  
  reset: () => {
    vi.mocked(global.fetch).mockReset();
  },
};

/**
 * Common test utilities for component testing
 */
export const testUtils = {
  /**
   * Wait for an element to be removed from the document
   */
  waitForElementToBeRemoved: async (element: HTMLElement) => {
    return new Promise<void>((resolve) => {
      const observer = new MutationObserver(() => {
        if (!document.contains(element)) {
          observer.disconnect();
          resolve();
        }
      });
      
      observer.observe(document.body, {
        childList: true,
        subtree: true,
      });
    });
  },

  /**
   * Create a mock event
   */
  createMockEvent: (type: string, eventInit?: Partial<Event>) => {
    return new Event(type, eventInit);
  },

  /**
   * Mock console methods
   */
  mockConsole: () => {
    const originalConsole = { ...console };
    console.log = vi.fn();
    console.warn = vi.fn();
    console.error = vi.fn();
    console.info = vi.fn();
    
    return {
      restore: () => {
        Object.assign(console, originalConsole);
      },
    };
  },
};

/**
 * Re-export everything from React Testing Library
 */
export * from '@testing-library/react';
export * from '@testing-library/user-event';

// Override the default render with our custom render
export { customRender as render };