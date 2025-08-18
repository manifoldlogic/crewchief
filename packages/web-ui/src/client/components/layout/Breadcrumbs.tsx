import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Button } from '../ui/button';

interface BreadcrumbItem {
  label: string;
  href: string;
  icon?: React.ReactNode;
}

interface BreadcrumbsProps {
  className?: string;
  maxItems?: number;
}

// Route configuration for generating breadcrumbs
const routeConfig: Record<string, { label: string; icon?: React.ReactNode }> = {
  '/': { 
    label: 'Dashboard', 
    icon: <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
    </svg>
  },
  '/search': { label: 'Search' },
  '/worktrees': { label: 'Worktrees' },
  '/agents': { label: 'Agents' },
  '/agents/active': { label: 'Active Agents' },
  '/agents/logs': { label: 'Agent Logs' },
  '/agents/performance': { label: 'Performance' },
  '/settings': { label: 'Settings' },
  '/settings/general': { label: 'General' },
  '/settings/security': { label: 'Security' },
  '/settings/integrations': { label: 'Integrations' },
  '/layout-demo': { label: 'Layout Demo' },
};

export const Breadcrumbs: React.FC<BreadcrumbsProps> = ({ 
  className = '',
  maxItems = 4 
}) => {
  const location = useLocation();

  const generateBreadcrumbs = (): BreadcrumbItem[] => {
    const pathSegments = location.pathname.split('/').filter(Boolean);
    const breadcrumbs: BreadcrumbItem[] = [];

    // Always include home unless we're already on home
    if (location.pathname !== '/') {
      breadcrumbs.push({
        label: 'Dashboard',
        href: '/',
        icon: <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
        </svg>,
      });
    }

    // Build breadcrumbs from path segments
    let currentPath = '';
    pathSegments.forEach((segment, index) => {
      currentPath += `/${segment}`;
      
      const config = routeConfig[currentPath];
      if (config) {
        // Don't duplicate home
        if (currentPath !== '/' || breadcrumbs.length === 0) {
          breadcrumbs.push({
            label: config.label,
            href: currentPath,
            icon: config.icon,
          });
        }
      } else {
        // Fallback for dynamic routes - capitalize and clean segment
        const label = segment
          .split('-')
          .map(word => word.charAt(0).toUpperCase() + word.slice(1))
          .join(' ');
        
        breadcrumbs.push({
          label,
          href: currentPath,
        });
      }
    });

    return breadcrumbs;
  };

  const breadcrumbs = generateBreadcrumbs();

  // Handle truncation if there are too many items
  const displayBreadcrumbs = breadcrumbs.length > maxItems
    ? [
        breadcrumbs[0], // Always show first (home)
        { label: '...', href: '', icon: null }, // Ellipsis
        ...breadcrumbs.slice(-2), // Show last 2 items
      ]
    : breadcrumbs;

  if (breadcrumbs.length <= 1) {
    // If only one item or just home, show page title instead
    const currentConfig = routeConfig[location.pathname];
    return (
      <div className={`flex items-center ${className}`}>
        <h1 className="text-xl font-semibold text-gray-900 dark:text-white flex items-center">
          {currentConfig?.icon && (
            <span className="mr-2 text-gray-500 dark:text-gray-400">
              {currentConfig.icon}
            </span>
          )}
          {currentConfig?.label || 'CrewChief'}
        </h1>
      </div>
    );
  }

  return (
    <nav 
      className={`flex items-center space-x-1 ${className}`}
      aria-label="Breadcrumb"
    >
      <ol className="flex items-center space-x-1" role="list">
        {displayBreadcrumbs.map((item, index) => {
          const isLast = index === displayBreadcrumbs.length - 1;
          const isEllipsis = item.label === '...';

          return (
            <li key={item.href || index} className="flex items-center">
              {index > 0 && (
                <svg 
                  className="flex-shrink-0 h-4 w-4 text-gray-400 dark:text-gray-600 mx-1" 
                  aria-hidden="true"
                  fill="none" 
                  viewBox="0 0 24 24" 
                  stroke="currentColor"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              )}
              
              {isEllipsis ? (
                <span 
                  className="text-gray-400 dark:text-gray-600 px-2"
                  aria-label="More items"
                >
                  ...
                </span>
              ) : isLast ? (
                <span 
                  className="flex items-center text-gray-900 dark:text-white font-medium"
                  aria-current="page"
                >
                  {item.icon && (
                    <span className="mr-2 text-gray-500 dark:text-gray-400">
                      {item.icon}
                    </span>
                  )}
                  <span className="truncate max-w-xs">{item.label}</span>
                </span>
              ) : (
                <Button
                  variant="ghost"
                  size="sm"
                  asChild
                  className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 h-auto p-1 font-medium"
                >
                  <Link 
                    to={item.href}
                    className="flex items-center"
                    aria-label={`Go to ${item.label}`}
                  >
                    {item.icon && (
                      <span className="mr-2">
                        {item.icon}
                      </span>
                    )}
                    <span className="truncate max-w-xs">{item.label}</span>
                  </Link>
                </Button>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
};