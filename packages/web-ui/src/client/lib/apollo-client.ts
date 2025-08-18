import { ApolloClient, InMemoryCache, createHttpLink, split } from '@apollo/client';
import { setContext } from '@apollo/client/link/context';
import { getMainDefinition } from '@apollo/client/utilities';
import { GraphQLWsLink } from '@apollo/client/link/subscriptions';
import { createClient } from 'graphql-ws';

// Get authentication token from localStorage or context
function getAuthToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
}

// HTTP link for queries and mutations
const httpLink = createHttpLink({
  uri: process.env.NODE_ENV === 'development' 
    ? 'http://localhost:3456/graphql' 
    : '/graphql',
});

// WebSocket link for subscriptions
const wsLink = new GraphQLWsLink(
  createClient({
    url: process.env.NODE_ENV === 'development'
      ? 'ws://localhost:3456/graphql'
      : `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/graphql`,
    
    connectionParams: () => {
      const token = getAuthToken();
      return {
        authorization: token ? `Bearer ${token}` : '',
        token,
      };
    },
    
    retryAttempts: 5,
    shouldRetry: (errOrCloseEvent) => {
      // Retry on network errors, but not on authentication errors
      if (errOrCloseEvent instanceof CloseEvent) {
        return errOrCloseEvent.code !== 4401; // Don't retry on auth errors
      }
      return true;
    },
    
    on: {
      connected: () => {
        console.log('🔌 Connected to GraphQL subscriptions');
      },
      
      closed: (event) => {
        console.log('🔌 Disconnected from GraphQL subscriptions:', event);
      },
      
      error: (error) => {
        console.error('GraphQL subscription error:', error);
      },
    },
  })
);

// Authentication link to add auth headers
const authLink = setContext((_, { headers }) => {
  const token = getAuthToken();
  
  return {
    headers: {
      ...headers,
      authorization: token ? `Bearer ${token}` : '',
    },
  };
});

// Split link to route queries/mutations to HTTP and subscriptions to WebSocket
const splitLink = split(
  ({ query }) => {
    const definition = getMainDefinition(query);
    return (
      definition.kind === 'OperationDefinition' &&
      definition.operation === 'subscription'
    );
  },
  wsLink,
  authLink.concat(httpLink)
);

// Apollo Client configuration
export const apolloClient = new ApolloClient({
  link: splitLink,
  cache: new InMemoryCache({
    typePolicies: {
      // Configure cache policies for real-time updates
      Worktree: {
        fields: {
          agents: {
            merge: false, // Replace rather than merge
          },
          runs: {
            merge: false,
          },
        },
      },
      Agent: {
        fields: {
          messages: {
            merge: false,
          },
          runs: {
            merge: false,
          },
        },
      },
      Run: {
        fields: {
          messages: {
            merge: false,
          },
        },
      },
      MaproomIndex: {
        fields: {
          searchHistory: {
            merge: false,
          },
        },
      },
      Query: {
        fields: {
          // Configure polling and caching for frequently updated data
          worktrees: {
            merge: false,
          },
          agents: {
            merge: false,
          },
          runs: {
            merge: false,
          },
        },
      },
    },
  }),
  
  // Default options
  defaultOptions: {
    watchQuery: {
      errorPolicy: 'all',
      notifyOnNetworkStatusChange: true,
    },
    query: {
      errorPolicy: 'all',
    },
    mutate: {
      errorPolicy: 'all',
    },
  },
  
  // Development settings
  connectToDevTools: process.env.NODE_ENV === 'development',
});

// Helper to update auth token
export function updateAuthToken(token: string | null) {
  if (typeof window === 'undefined') return;
  
  if (token) {
    localStorage.setItem('auth_token', token);
  } else {
    localStorage.removeItem('auth_token');
    sessionStorage.removeItem('auth_token');
  }
  
  // Restart WebSocket connection with new token
  if (wsLink) {
    wsLink.client.restart();
  }
}

// Helper to clear cache and restart connections
export function resetApolloClient() {
  apolloClient.resetStore();
  if (wsLink) {
    wsLink.client.restart();
  }
}