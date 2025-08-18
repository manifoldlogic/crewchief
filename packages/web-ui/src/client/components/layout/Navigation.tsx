import React, { useState, useEffect } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Button } from '../ui/button';

interface NavigationItem {
  name: string;
  href: string;
  icon: React.ReactNode;
  children?: NavigationItem[];
  badge?: string | number;
}

interface NavigationProps {
  collapsed?: boolean;
  mobile?: boolean;
  onItemClick?: () => void;
}

const navigationItems: NavigationItem[] = [
  {
    name: 'Dashboard',
    href: '/',
    icon: <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 5a2 2 0 012-2h4a2 2 0 012 2v3H8V5z" />
    </svg>,
  },
  {
    name: 'Search',
    href: '/search',
    icon: <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>,
  },
  {
    name: 'Worktrees',
    href: '/worktrees',
    icon: <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
    </svg>,
  },
  {
    name: 'Agents',
    href: '/agents',
    icon: <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>,
    children: [
      {
        name: 'Active Agents',
        href: '/agents/active',
        icon: <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>,
      },
      {
        name: 'Agent Logs',
        href: '/agents/logs',
        icon: <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>,
      },
      {
        name: 'Performance',
        href: '/agents/performance',
        icon: <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
        </svg>,
      },
    ],
  },
  {
    name: 'Settings',
    href: '/settings',
    icon: <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    </svg>,
  },
  {
    name: 'Layout Demo',
    href: '/layout-demo',
    icon: <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
    </svg>,
  },
];

export const Navigation: React.FC<NavigationProps> = ({ 
  collapsed = false, 
  mobile = false, 
  onItemClick 
}) => {
  const location = useLocation();
  const [expandedItems, setExpandedItems] = useState<string[]>([]);
  const [focusedIndex, setFocusedIndex] = useState(-1);

  // Auto-expand parent items for current path
  useEffect(() => {
    const currentPath = location.pathname;
    const itemsToExpand: string[] = [];
    
    navigationItems.forEach(item => {
      if (item.children) {
        const hasActiveChild = item.children.some(child => 
          currentPath === child.href || currentPath.startsWith(child.href + '/')
        );
        if (hasActiveChild) {
          itemsToExpand.push(item.name);
        }
      }
    });
    
    setExpandedItems(itemsToExpand);
  }, [location.pathname]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const allItems = getAllNavigationItems();
      
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          setFocusedIndex(prev => 
            prev < allItems.length - 1 ? prev + 1 : 0
          );
          break;
        case 'ArrowUp':
          e.preventDefault();
          setFocusedIndex(prev => 
            prev > 0 ? prev - 1 : allItems.length - 1
          );
          break;
        case 'Enter':
        case ' ':
          e.preventDefault();
          if (focusedIndex >= 0 && allItems[focusedIndex]) {
            const item = allItems[focusedIndex];
            if (item.children) {
              toggleExpanded(item.name);
            } else {
              window.location.href = item.href;
              onItemClick?.();
            }
          }
          break;
        case 'Escape':
          setFocusedIndex(-1);
          break;
      }
    };

    if (!mobile) {
      document.addEventListener('keydown', handleKeyDown);
      return () => document.removeEventListener('keydown', handleKeyDown);
    }
  }, [focusedIndex, mobile, onItemClick]);

  const getAllNavigationItems = (): NavigationItem[] => {
    const items: NavigationItem[] = [];
    navigationItems.forEach(item => {
      items.push(item);
      if (item.children && expandedItems.includes(item.name)) {
        items.push(...item.children);
      }
    });
    return items;
  };

  const toggleExpanded = (itemName: string) => {
    setExpandedItems(prev => 
      prev.includes(itemName) 
        ? prev.filter(name => name !== itemName)
        : [...prev, itemName]
    );
  };

  const isActive = (href: string) => {
    return location.pathname === href || location.pathname.startsWith(href + '/');
  };

  const renderNavigationItem = (item: NavigationItem, level: number = 0, index: number = 0) => {
    const active = isActive(item.href);
    const hasChildren = item.children && item.children.length > 0;
    const isExpanded = hasChildren && expandedItems.includes(item.name);
    const isFocused = focusedIndex === index;
    
    const baseClasses = `
      group flex items-center w-full text-left transition-all duration-200
      ${level === 0 ? 'px-3 py-2' : 'px-3 py-1.5 ml-4'}
      ${collapsed && level === 0 ? 'justify-center px-2' : ''}
      ${active 
        ? 'bg-primary-100 dark:bg-primary-900 text-primary-900 dark:text-primary-100' 
        : 'text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 hover:text-gray-900 dark:hover:text-white'
      }
      ${isFocused ? 'ring-2 ring-primary-500 ring-inset' : ''}
      text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500
    `;

    const iconClasses = `
      ${active 
        ? 'text-primary-500 dark:text-primary-300' 
        : 'text-gray-400 dark:text-gray-500 group-hover:text-gray-500 dark:group-hover:text-gray-300'
      }
      ${collapsed && level === 0 ? '' : 'mr-3'}
      flex-shrink-0 transition-colors duration-200
    `;

    if (hasChildren && !collapsed) {
      return (
        <div key={item.name}>
          <Button
            variant="ghost"
            className={baseClasses}
            onClick={() => toggleExpanded(item.name)}
            aria-expanded={isExpanded}
            aria-label={`${isExpanded ? 'Collapse' : 'Expand'} ${item.name} menu`}
          >
            <span className={iconClasses}>
              {item.icon}
            </span>
            <span className="flex-1 truncate">{item.name}</span>
            {item.badge && (
              <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200">
                {item.badge}
              </span>
            )}
            <svg 
              className={`ml-2 w-4 h-4 transition-transform duration-200 ${
                isExpanded ? 'rotate-180' : ''
              }`}
              fill="none" 
              viewBox="0 0 24 24" 
              stroke="currentColor"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </Button>
          
          {isExpanded && (
            <div className="mt-1 space-y-1" role="group" aria-label={`${item.name} submenu`}>
              {item.children?.map((child, childIndex) => 
                renderNavigationItem(child, level + 1, index + childIndex + 1)
              )}
            </div>
          )}
        </div>
      );
    }

    return (
      <Link
        key={item.href}
        to={item.href}
        onClick={onItemClick}
        className={baseClasses}
        aria-current={active ? 'page' : undefined}
        title={collapsed ? item.name : undefined}
      >
        <span className={iconClasses}>
          {item.icon}
        </span>
        {(!collapsed || mobile) && (
          <>
            <span className="flex-1 truncate">{item.name}</span>
            {item.badge && (
              <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200">
                {item.badge}
              </span>
            )}
          </>
        )}
      </Link>
    );
  };

  return (
    <nav 
      className="mt-5 flex-grow flex flex-col overflow-y-auto"
      aria-label="Main navigation"
      role="navigation"
    >
      <div className="flex-grow">
        <div className={`space-y-1 ${collapsed ? 'px-1' : 'px-2'}`}>
          {navigationItems.map((item, index) => renderNavigationItem(item, 0, index))}
        </div>
      </div>

      {/* Footer */}
      {(!collapsed || mobile) && (
        <div className="flex-shrink-0 p-4 border-t border-gray-200 dark:border-gray-700">
          <div className="text-xs text-gray-500 dark:text-gray-400">
            <div>Version 0.1.0</div>
            <div className="mt-1">CrewChief Web UI</div>
          </div>
        </div>
      )}
      
      {/* Keyboard shortcuts help */}
      {!mobile && !collapsed && (
        <div className="flex-shrink-0 p-4 border-t border-gray-200 dark:border-gray-700">
          <div className="text-xs text-gray-400 dark:text-gray-500">
            <div className="mb-1">Keyboard shortcuts:</div>
            <div>⌘/Ctrl + B: Toggle sidebar</div>
            <div>↑↓: Navigate items</div>
            <div>Enter: Select item</div>
          </div>
        </div>
      )}
    </nav>
  );
};