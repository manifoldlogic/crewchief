import { makeExecutableSchema } from '@graphql-tools/schema';

// Import type definitions
import { baseTypeDefs } from './types/base.js';
import { worktreeTypeDefs } from './types/worktree.js';
import { agentTypeDefs } from './types/agent.js';
import { runTypeDefs } from './types/run.js';
import { maproomIndexTypeDefs } from './types/maproom-index.js';
import { configurationTypeDefs } from './types/configuration.js';
import { eventTypeDefs } from './types/event.js';
import { agentMessageTypeDefs } from './types/agent-message.js';

// Import resolvers
import { baseResolvers } from './resolvers/base.js';
import { worktreeResolvers } from './resolvers/worktree.js';
import { agentResolvers } from './resolvers/agent.js';
import { runResolvers } from './resolvers/run.js';
import { maproomIndexResolvers } from './resolvers/maproom-index.js';
import { configurationResolvers } from './resolvers/configuration.js';
import { eventResolvers } from './resolvers/event.js';
import { agentMessageResolvers } from './resolvers/agent-message.js';

// Import subscription resolvers
import { subscriptionResolvers } from './subscriptions/resolvers.js';

// Root type definitions (required for schema extension)
const rootTypeDefs = `
  type Query {
    # Health check
    health: String!
  }

  type Mutation {
    # Placeholder mutation
    _empty: String
  }

  type Subscription {
    # Placeholder subscription
    _empty: String
  }
`;

// Combine all type definitions
export const typeDefs = [
  rootTypeDefs,
  baseTypeDefs,
  worktreeTypeDefs,
  agentTypeDefs,
  runTypeDefs,
  maproomIndexTypeDefs,
  configurationTypeDefs,
  eventTypeDefs,
  agentMessageTypeDefs,
];

// Combine all resolvers
export const resolvers = {
  ...baseResolvers,
  ...worktreeResolvers,
  ...agentResolvers,
  ...runResolvers,
  ...maproomIndexResolvers,
  ...configurationResolvers,
  ...eventResolvers,
  ...agentMessageResolvers,
  
  // Root resolvers
  Query: {
    health: () => 'GraphQL server is running!',
    ...baseResolvers.Query,
    ...worktreeResolvers.Query,
    ...agentResolvers.Query,
    ...runResolvers.Query,
    ...maproomIndexResolvers.Query,
    ...configurationResolvers.Query,
    ...eventResolvers.Query,
    ...agentMessageResolvers.Query,
  },
  
  Mutation: {
    _empty: () => null,
    ...baseResolvers.Mutation,
    ...worktreeResolvers.Mutation,
    ...agentResolvers.Mutation,
    ...runResolvers.Mutation,
    ...maproomIndexResolvers.Mutation,
    ...configurationResolvers.Mutation,
    ...eventResolvers.Mutation,
    ...agentMessageResolvers.Mutation,
  },
  
  Subscription: {
    _empty: () => null,
    ...subscriptionResolvers,
    ...baseResolvers.Subscription,
    ...worktreeResolvers.Subscription,
    ...agentResolvers.Subscription,
    ...runResolvers.Subscription,
    ...maproomIndexResolvers.Subscription,
    ...configurationResolvers.Subscription,
    ...eventResolvers.Subscription,
    ...agentMessageResolvers.Subscription,
  },
};

// Create executable schema
export const schema = makeExecutableSchema({
  typeDefs,
  resolvers,
});

export default schema;