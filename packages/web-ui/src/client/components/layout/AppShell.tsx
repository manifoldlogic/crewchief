import React, { useState, useEffect, useCallback } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Button } from '../ui/button';
import { Navigation } from './Navigation';
import { Breadcrumbs } from './Breadcrumbs';

interface AppShellProps {
  children: React.ReactNode;
  showFooter?: boolean;
}

interface LayoutState {
  sidebarOpen: boolean;
  sidebarCollapsed: boolean;
  isMobile: boolean;
}

const STORAGE_KEY = 'crewchief-layout-state';

export const AppShell: React.FC<AppShellProps> = ({ children, showFooter = false }) => {
  const location = useLocation();
  const [isDark, setIsDark] = useState(false);
  const [layoutState, setLayoutState] = useState<LayoutState>({
    sidebarOpen: false,
    sidebarCollapsed: false,
    isMobile: false,
  });

  // Load layout state from localStorage
  useEffect(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      try {
        const parsedState = JSON.parse(saved);
        setLayoutState(prev => ({
          ...prev,
          sidebarCollapsed: parsedState.sidebarCollapsed || false,
        }));
      } catch (error) {
        console.warn('Failed to parse saved layout state:', error);
      }
    }
  }, []);

  // Save layout state to localStorage
  const saveLayoutState = useCallback((newState: Partial<LayoutState>) => {
    setLayoutState(prev => {
      const updatedState = { ...prev, ...newState };
      localStorage.setItem(STORAGE_KEY, JSON.stringify({
        sidebarCollapsed: updatedState.sidebarCollapsed,
      }));
      return updatedState;
    });
  }, []);

  // Check if mobile based on viewport
  useEffect(() => {
    const checkMobile = () => {
      const isMobile = window.innerWidth < 768;
      setLayoutState(prev => ({ ...prev, isMobile }));
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  // Check for dark mode preference
  useEffect(() => {
    const isDarkMode = document.documentElement.classList.contains('dark');
    setIsDark(isDarkMode);
  }, []);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyboard = (e: KeyboardEvent) => {
      // Toggle sidebar with Ctrl/Cmd + B
      if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault();
        if (layoutState.isMobile) {
          saveLayoutState({ sidebarOpen: !layoutState.sidebarOpen });
        } else {
          saveLayoutState({ sidebarCollapsed: !layoutState.sidebarCollapsed });
        }
      }
      
      // Close mobile sidebar with Escape
      if (e.key === 'Escape' && layoutState.isMobile && layoutState.sidebarOpen) {
        saveLayoutState({ sidebarOpen: false });
      }
    };

    document.addEventListener('keydown', handleKeyboard);
    return () => document.removeEventListener('keydown', handleKeyboard);
  }, [layoutState.isMobile, layoutState.sidebarOpen, layoutState.sidebarCollapsed, saveLayoutState]);

  const toggleTheme = () => {
    document.documentElement.classList.toggle('dark');
    setIsDark(!isDark);
  };

  const sidebarWidth = layoutState.sidebarCollapsed ? 'w-16' : 'w-64';
  const mainMargin = layoutState.isMobile ? 'ml-0' : (layoutState.sidebarCollapsed ? 'ml-16' : 'ml-64');

  return (
    <div className="h-screen flex overflow-hidden bg-gray-50 dark:bg-gray-900">
      {/* Desktop Sidebar */}
      <div 
        className={`hidden md:flex md:flex-shrink-0 transition-all duration-300 ease-in-out ${sidebarWidth}`}
        aria-label="Sidebar"
      >
        <div className="flex flex-col">
          {/* Logo/Header */}
          <div className="flex items-center h-16 flex-shrink-0 px-4 bg-primary-600 border-b border-primary-700">
            <div className="flex items-center min-w-0">
              <div className="flex-shrink-0">
                <div className="w-8 h-8 bg-white rounded-lg flex items-center justify-center">
                  <span className="text-primary-600 font-bold text-lg">CC</span>
                </div>
              </div>
              {!layoutState.sidebarCollapsed && (
                <div className="ml-3 min-w-0">
                  <h1 className="text-white font-semibold text-lg truncate">CrewChief</h1>
                </div>
              )}
            </div>
            
            {/* Collapse/Expand Button */}
            <Button
              variant="ghost"
              size="sm"
              className={`text-white hover:bg-primary-700 ${layoutState.sidebarCollapsed ? 'ml-auto' : 'ml-auto'}`}
              onClick={() => saveLayoutState({ sidebarCollapsed: !layoutState.sidebarCollapsed })}
              aria-label={layoutState.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            >
              {layoutState.sidebarCollapsed ? (
                <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              ) : (
                <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
              )}
            </Button>
          </div>

          {/* Navigation */}
          <Navigation collapsed={layoutState.sidebarCollapsed} />
        </div>
      </div>

      {/* Mobile Sidebar Overlay */}
      {layoutState.isMobile && layoutState.sidebarOpen && (
        <div 
          className="fixed inset-0 flex z-40 md:hidden"
          onClick={() => saveLayoutState({ sidebarOpen: false })}
        >
          <div 
            className="fixed inset-0 bg-gray-600 bg-opacity-75 transition-opacity"
            aria-hidden="true"
          />
          <div 
            className="relative flex-1 flex flex-col max-w-xs w-full bg-white dark:bg-gray-800 transform transition-transform"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Mobile Header */}
            <div className="flex items-center h-16 flex-shrink-0 px-4 bg-primary-600">
              <div className="flex items-center">
                <div className="flex-shrink-0">
                  <div className="w-8 h-8 bg-white rounded-lg flex items-center justify-center">
                    <span className="text-primary-600 font-bold text-lg">CC</span>
                  </div>
                </div>
                <div className="ml-3">
                  <h1 className="text-white font-semibold text-lg">CrewChief</h1>
                </div>
              </div>
              
              <Button
                variant="ghost"
                size="sm"
                className="ml-auto text-white hover:bg-primary-700"
                onClick={() => saveLayoutState({ sidebarOpen: false })}
                aria-label="Close sidebar"
              >
                <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </Button>
            </div>

            {/* Mobile Navigation */}
            <Navigation 
              mobile 
              onItemClick={() => saveLayoutState({ sidebarOpen: false })} 
            />
          </div>
        </div>
      )}

      {/* Main Content */}
      <div className={`flex flex-col min-w-0 flex-1 overflow-hidden transition-all duration-300 ease-in-out ${mainMargin}`}>
        {/* Header */}
        <header className="relative z-10 flex-shrink-0 flex h-16 bg-white dark:bg-gray-800 shadow">
          {/* Mobile menu button */}
          {layoutState.isMobile && (
            <Button
              variant="ghost"
              size="sm"
              className="px-4 border-r border-gray-200 dark:border-gray-700 text-gray-500 dark:text-gray-400 md:hidden"
              onClick={() => saveLayoutState({ sidebarOpen: true })}
              aria-label="Open sidebar"
            >
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
              </svg>
            </Button>
          )}
          
          <div className="flex-1 px-4 flex justify-between items-center">
            <div className="flex-1 min-w-0">
              <Breadcrumbs />
            </div>
            
            <div className="ml-4 flex items-center space-x-4">
              {/* Theme toggle */}
              <Button
                variant="ghost"
                size="sm"
                onClick={toggleTheme}
                aria-label="Toggle theme"
                className="text-gray-400 hover:text-gray-500 dark:hover:text-gray-300"
              >
                {isDark ? (
                  <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                  </svg>
                ) : (
                  <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                  </svg>
                )}
              </Button>
              
              {/* Status indicator */}
              <div className="flex items-center">
                <div className="w-2 h-2 bg-green-400 rounded-full mr-2"></div>
                <span className="text-sm text-gray-600 dark:text-gray-300 hidden sm:inline">
                  Online
                </span>
              </div>
            </div>
          </div>
        </header>

        {/* Main content area */}
        <main className="flex-1 relative overflow-y-auto focus:outline-none">
          <div className="py-6">
            <div className="max-w-7xl mx-auto px-4 sm:px-6 md:px-8">
              {children}
            </div>
          </div>
        </main>

        {/* Footer */}
        {showFooter && (
          <footer className="flex-shrink-0 bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700">
            <div className="max-w-7xl mx-auto px-4 sm:px-6 md:px-8 py-4">
              <div className="flex items-center justify-between">
                <div className="text-sm text-gray-500 dark:text-gray-400">
                  © 2025 CrewChief. All rights reserved.
                </div>
                <div className="text-sm text-gray-500 dark:text-gray-400">
                  Version 0.1.0
                </div>
              </div>
            </div>
          </footer>
        )}
      </div>
    </div>
  );
};