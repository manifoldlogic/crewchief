/**
 * 404 Not Found page component
 */

import React from 'react';
import { Link, useLocation } from 'react-router-dom';

interface NotFoundProps {
  title?: string;
  message?: string;
  showBackButton?: boolean;
  showHomeButton?: boolean;
}

export function NotFound({
  title = 'Page Not Found',
  message,
  showBackButton = true,
  showHomeButton = true,
}: NotFoundProps): React.ReactElement {
  const location = useLocation();

  const defaultMessage = `The page "${location.pathname}" could not be found. It may have been moved, deleted, or you may have entered the URL incorrectly.`;

  const handleGoBack = () => {
    // Use history API to go back, fallback to home page
    if (window.history.length > 1) {
      window.history.back();
    } else {
      window.location.href = '/';
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 px-4">
      <div className="max-w-lg w-full text-center">
        {/* 404 Illustration */}
        <div className="mb-8">
          <div className="mx-auto h-32 w-32 flex items-center justify-center rounded-full bg-gray-100 mb-6">
            <span className="text-6xl" role="img" aria-label="Not found">
              🔍
            </span>
          </div>
          
          {/* Large 404 text */}
          <div className="mb-4">
            <h1 className="text-8xl font-bold text-gray-900 mb-2">404</h1>
            <h2 className="text-2xl font-semibold text-gray-700">{title}</h2>
          </div>
        </div>

        {/* Error message */}
        <div className="mb-8">
          <p className="text-gray-600 leading-relaxed">
            {message || defaultMessage}
          </p>
        </div>

        {/* Helpful suggestions */}
        <div className="bg-white rounded-lg border border-gray-200 p-6 mb-8">
          <h3 className="text-lg font-medium text-gray-900 mb-4">
            What can you do?
          </h3>
          <ul className="text-left space-y-3 text-gray-600">
            <li className="flex items-start gap-2">
              <span className="text-green-500 mt-1">✓</span>
              <span>Check the URL for typos or spelling errors</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500 mt-1">✓</span>
              <span>Use the navigation menu to find what you're looking for</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500 mt-1">✓</span>
              <span>Return to the dashboard to explore available features</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500 mt-1">✓</span>
              <span>Use the search feature to find specific content</span>
            </li>
          </ul>
        </div>

        {/* Action buttons */}
        <div className="space-y-3">
          {showHomeButton && (
            <Link
              to="/"
              className="inline-flex items-center px-6 py-3 text-base font-medium text-white bg-primary-600 border border-transparent rounded-md shadow-sm hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 transition-colors"
            >
              <svg
                className="w-5 h-5 mr-2"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
                />
              </svg>
              Go to Dashboard
            </Link>
          )}

          {showBackButton && (
            <div>
              <button
                onClick={handleGoBack}
                className="inline-flex items-center px-6 py-3 text-base font-medium text-gray-700 bg-white border border-gray-300 rounded-md shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 transition-colors"
              >
                <svg
                  className="w-5 h-5 mr-2"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 19l-7-7m0 0l7-7m-7 7h18"
                  />
                </svg>
                Go Back
              </button>
            </div>
          )}

          {/* Additional helpful links */}
          <div className="pt-4 border-t border-gray-200">
            <p className="text-sm text-gray-500 mb-3">
              Quick navigation:
            </p>
            <div className="flex flex-wrap gap-2 justify-center">
              <Link
                to="/search"
                className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-primary-700 bg-primary-100 rounded-md hover:bg-primary-200 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
              >
                Search
              </Link>
              <Link
                to="/agents"
                className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-primary-700 bg-primary-100 rounded-md hover:bg-primary-200 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
              >
                Agents
              </Link>
              <Link
                to="/worktrees"
                className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-primary-700 bg-primary-100 rounded-md hover:bg-primary-200 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
              >
                Worktrees
              </Link>
              <Link
                to="/settings"
                className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-primary-700 bg-primary-100 rounded-md hover:bg-primary-200 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
              >
                Settings
              </Link>
            </div>
          </div>
        </div>

        {/* Debug information (development only) */}
        {import.meta.env.DEV && (
          <details className="mt-8 text-left">
            <summary className="text-sm font-medium text-gray-500 cursor-pointer hover:text-gray-700">
              Debug Information
            </summary>
            <div className="mt-2 p-3 bg-gray-100 rounded text-xs font-mono text-gray-700">
              <div><strong>Pathname:</strong> {location.pathname}</div>
              <div><strong>Search:</strong> {location.search || 'None'}</div>
              <div><strong>Hash:</strong> {location.hash || 'None'}</div>
              <div><strong>State:</strong> {location.state ? JSON.stringify(location.state) : 'None'}</div>
              <div><strong>Timestamp:</strong> {new Date().toISOString()}</div>
            </div>
          </details>
        )}
      </div>
    </div>
  );
}

export default NotFound;