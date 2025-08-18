import React, { ReactNode, useEffect, useState } from 'react';
import { ApolloProvider, useApolloClient } from '@apollo/client';
import { apolloClient, updateAuthToken, resetApolloClient } from '../../lib/apollo-client.js';

interface ApolloProviderWrapperProps {
  children: ReactNode;
}

// Connection status indicator
function ConnectionStatus() {
  const [isConnected, setIsConnected] = useState(true);
  const [isReconnecting, setIsReconnecting] = useState(false);
  const client = useApolloClient();

  useEffect(() => {
    // Monitor network status
    const handleOnline = () => {
      setIsConnected(true);
      setIsReconnecting(false);
    };

    const handleOffline = () => {
      setIsConnected(false);
    };

    // Monitor Apollo Client connection
    const handleNetworkError = () => {
      setIsConnected(false);
      setIsReconnecting(true);
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [client]);

  if (!isConnected) {
    return (
      <div className="fixed top-0 left-0 right-0 bg-red-500 text-white text-center py-2 text-sm z-50">
        {isReconnecting ? 'Reconnecting...' : 'No internet connection'}
      </div>
    );
  }

  return null;
}

// Auth token manager
function AuthTokenManager({ children }: { children: ReactNode }) {
  useEffect(() => {
    // Listen for auth token changes
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === 'auth_token') {
        updateAuthToken(e.newValue);
      }
    };

    // Listen for custom auth events
    const handleAuthChange = (e: CustomEvent) => {
      updateAuthToken(e.detail.token);
    };

    window.addEventListener('storage', handleStorageChange);
    window.addEventListener('auth-token-changed' as any, handleAuthChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      window.removeEventListener('auth-token-changed' as any, handleAuthChange);
    };
  }, []);

  return <>{children}</>;
}

// Main Apollo provider wrapper
export function ApolloProviderWrapper({ children }: ApolloProviderWrapperProps) {
  return (
    <ApolloProvider client={apolloClient}>
      <ConnectionStatus />
      <AuthTokenManager>
        {children}
      </AuthTokenManager>
    </ApolloProvider>
  );
}

// Helper hook to get Apollo client instance
export function useApolloClientInstance() {
  return apolloClient;
}

// Helper functions for auth management
export function setAuthToken(token: string) {
  updateAuthToken(token);
  
  // Dispatch custom event for other components
  window.dispatchEvent(
    new CustomEvent('auth-token-changed', {
      detail: { token },
    })
  );
}

export function clearAuthToken() {
  updateAuthToken(null);
  resetApolloClient();
  
  // Dispatch custom event for other components
  window.dispatchEvent(
    new CustomEvent('auth-token-changed', {
      detail: { token: null },
    })
  );
}

// Error handler for subscription errors
export function handleSubscriptionError(error: any) {
  console.error('Subscription error:', error);
  
  // Check if it's an authentication error
  if (error.message?.includes('Authentication') || error.message?.includes('Unauthorized')) {
    // Clear token and redirect to login
    clearAuthToken();
    window.location.href = '/login';
    return;
  }
  
  // Check if it's a network error
  if (error.networkError) {
    console.warn('Network error in subscription, will retry automatically');
    return;
  }
  
  // Log other errors for debugging
  console.error('Unhandled subscription error:', error);
}