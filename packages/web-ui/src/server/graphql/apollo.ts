import { ApolloServer } from '@apollo/server';
import { expressMiddleware } from '@apollo/server/express4';
import { ApolloServerPluginDrainHttpServer } from '@apollo/server/plugin/drainHttpServer';
import { ApolloServerPluginLandingPageLocalDefault } from '@apollo/server/plugin/landingPage/default';
import { makeExecutableSchema } from '@graphql-tools/schema';
import type { Express } from 'express';
import type { Server } from 'http';
import cors from 'cors';
import bodyParser from 'body-parser';
import type { DatabaseConnection } from '../../db/connection.js';

// Import schema and resolvers
import { typeDefs, resolvers } from './schema.js';
import { initializeDatabaseService } from './services/database.js';

// Security plugins
import { createDepthLimitPlugin } from './middleware/depth-limit.js';
import { createRateLimitPlugin } from './middleware/rate-limit.js';
import { createAuthPlugin } from './middleware/auth.js';

// Context type
export interface GraphQLContext {
  req: any;
  res: any;
  user?: {
    id: string;
    sessionId: string;
    permissions: string[];
  };
  dataSources: {
    db: DatabaseConnection;
  };
}

// Create and configure Apollo Server
export async function createApolloServer(
  httpServer: Server,
  db: DatabaseConnection
): Promise<ApolloServer<GraphQLContext>> {
  // Initialize database service
  initializeDatabaseService(db);

  // Create executable schema
  const schema = makeExecutableSchema({
    typeDefs,
    resolvers,
  });

  // Create Apollo Server
  const server = new ApolloServer<GraphQLContext>({
    schema,
    plugins: [
      // HTTP server drainage plugin
      ApolloServerPluginDrainHttpServer({ httpServer }),
      
      // Development landing page (GraphQL Playground)
      process.env.NODE_ENV === 'development'
        ? ApolloServerPluginLandingPageLocalDefault({ embed: true })
        : ApolloServerPluginLandingPageLocalDefault({ embed: false }),
      
      // Security plugins
      createDepthLimitPlugin(10), // Max depth of 10
      createRateLimitPlugin({
        windowMs: 15 * 60 * 1000, // 15 minutes
        max: 1000, // limit each IP to 1000 requests per windowMs
        skipSuccessfulRequests: false,
      }),
      createAuthPlugin(),
    ],
    
    // Error formatting
    formatError: (formattedError, error) => {
      // Log error for debugging
      console.error('GraphQL Error:', error);
      
      // Don't expose internal errors in production
      if (process.env.NODE_ENV === 'production') {
        if (formattedError.message.includes('Database') || 
            formattedError.message.includes('Internal')) {
          return new Error('Internal server error');
        }
      }
      
      return formattedError;
    },
    
    // Introspection and playground in development only
    introspection: process.env.NODE_ENV === 'development',
  });

  return server;
}

// Context function for Apollo Server middleware
export function createContextFunction(db: DatabaseConnection) {
  return async ({ req, res }: { req: any; res: any }): Promise<GraphQLContext> => {
    // Extract user information from request (from auth middleware)
    const user = req.user || null;
    
    return {
      req,
      res,
      user,
      dataSources: {
        db,
      },
    };
  };
}

// Setup GraphQL endpoint with Express
export async function setupGraphQLEndpoint(
  app: Express,
  httpServer: Server,
  db: DatabaseConnection,
  path = '/graphql'
) {
  try {
    // Create Apollo Server
    const server = await createApolloServer(httpServer, db);
    
    // Start the server
    await server.start();
    
    // Apply middleware
    app.use(
      path,
      cors<cors.CorsRequest>(),
      bodyParser.json(),
      expressMiddleware(server, {
        context: createContextFunction(db),
      })
    );
    
    console.log(`🚀 GraphQL server ready at http://localhost:${process.env.PORT || 3456}${path}`);
    console.log(`📊 GraphQL Playground available in development mode`);
    
    return server;
  } catch (error) {
    console.error('Failed to setup GraphQL endpoint:', error);
    throw error;
  }
}