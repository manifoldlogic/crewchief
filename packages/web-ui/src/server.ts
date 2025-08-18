import express, { type Express } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import morgan from 'morgan';
import cookieParser from 'cookie-parser';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { createServer } from 'http';
import maproomRoutes from './routes/maproom.js';
import { initializeDatabase } from './db/connection.js';
// import { setupGraphQLEndpoint } from './server/graphql/apollo.js';
import { createSimpleApiRouter } from './server/api/simple-index.js';
import { createAuthRouter } from './server/auth/routes/auth.js';
import { secureHeaders, secureCookies } from './server/auth/middleware/csrf.js';
import { apiRateLimit } from './server/auth/middleware/rate-limit.js';
import type { DatabaseConnection } from './db/connection.js';
import { WebSocketServer } from './server/websocket/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const PORT = process.env.PORT ? parseInt(process.env.PORT, 10) : 3456;
const NODE_ENV = process.env.NODE_ENV || 'development';
const isDevelopment = NODE_ENV === 'development';

// Create Express app
const app: Express = express();

// Trust proxy for production deployment
if (process.env.NODE_ENV === 'production') {
  app.set('trust proxy', 1);
}

// Cookie parser (must be before auth middleware)
app.use(cookieParser());

// Security headers and CSRF protection
app.use(secureHeaders());
app.use(secureCookies());

// Security middleware
app.use(helmet({
  contentSecurityPolicy: isDevelopment ? false : {
    directives: {
      defaultSrc: ["'self'"],
      scriptSrc: ["'self'", "'unsafe-inline'", "'unsafe-eval'"],
      styleSrc: ["'self'", "'unsafe-inline'"],
      imgSrc: ["'self'", "data:", "https:"],
      fontSrc: ["'self'", "data:"],
      connectSrc: ["'self'", "ws:", "wss:"],
      frameAncestors: ["'none'"],
    },
  },
}));

// CORS configuration
app.use(cors({
  origin: isDevelopment 
    ? ['http://localhost:3000', 'http://localhost:3456', 'http://localhost:5173']
    : process.env.ALLOWED_ORIGINS?.split(',') || false,
  credentials: true,
  methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Authorization', 'x-api-key', 'x-csrf-token', 'x-double-submit-token'],
}));

// Compression middleware
app.use(compression());

// Logging middleware
app.use(morgan(isDevelopment ? 'dev' : 'combined'));

// Rate limiting for API endpoints
app.use('/api', apiRateLimit);

// Body parsing middleware
app.use(express.json({ limit: '10mb' }));
app.use(express.urlencoded({ extended: true, limit: '10mb' }));

// Legacy maproom routes (keeping for backward compatibility)
app.use('/api/maproom', maproomRoutes);

// Placeholder for API routes (will be mounted in startServer)
// Note: This is just a placeholder - actual routes are mounted later

// Static file serving
if (isDevelopment) {
  // In development, serve the public directory for any static assets
  const publicPath = join(__dirname, '../public');
  app.use(express.static(publicPath));
} else {
  // In production, serve the built frontend from dist/client
  const clientPath = join(__dirname, './client');
  app.use(express.static(clientPath));
}

// Catch-all handler will be mounted in startServer() after API routes

// Error handling middleware
app.use((err: Error, req: express.Request, res: express.Response, next: express.NextFunction) => {
  console.error('Error:', err);
  
  if (res.headersSent) {
    return next(err);
  }

  const status = (err as any).status || (err as any).statusCode || 500;
  const message = isDevelopment ? err.message : 'Internal Server Error';
  
  res.status(status).json({
    error: message,
    ...(isDevelopment && { stack: err.stack }),
  });
});

// 404 handler will be mounted in startServer() after API routes

// Function to start the server
export async function startServer() {
  let db: DatabaseConnection | undefined;
  
  try {
    // Initialize database connection
    db = await initializeDatabase();
    console.log('✅ Database initialized successfully');

    // Store database in app locals for middleware access
    app.locals.db = db.getPool();
    
    // Setup authentication routes
    try {
      const authRouter = createAuthRouter(db.getPool());
      app.use('/auth', authRouter);
      console.log('✅ Authentication routes initialized successfully');
    } catch (error) {
      console.error('❌ Failed to initialize authentication routes:', error);
      console.warn('⚠️  Server will start without authentication');
    }
    
    // Setup REST API endpoints now that database is available
    try {
      const apiRouter = createSimpleApiRouter(db.getPool());
      app.use('/api', apiRouter);
      console.log('✅ REST API endpoints initialized successfully');
    } catch (error) {
      console.error('❌ Failed to initialize REST API endpoints:', error);
      console.warn('⚠️  Server will start without REST API endpoints');
    }
    
  } catch (error) {
    console.error('❌ Failed to initialize database:', error);
    console.warn('⚠️  Server will start without database connection');
  }

  // Mount 404 handler before catch-all
  app.use((req, res, next) => {
    // Only handle if no other route matched
    if (req.path.startsWith('/api/')) {
      res.status(404).json({
        error: 'Not Found',
        path: req.path,
        method: req.method,
      });
    } else {
      next();
    }
  });

  // Mount catch-all handler after API routes
  app.get('*', (req, res) => {
    if (isDevelopment) {
      // In development, let Vite handle the frontend
      res.json({
        message: 'CrewChief Web UI - Development mode',
        note: 'Frontend is served by Vite on port 3000',
        api: '/api',
        health: '/api/health',
      });
    } else {
      // In production, serve the built React app
      const indexPath = join(__dirname, './client/index.html');
      res.sendFile(indexPath);
    }
  });

  // Create HTTP server
  const httpServer = createServer(app);

  // Initialize WebSocket server
  let wsServer: WebSocketServer | undefined;
  if (db) {
    try {
      wsServer = new WebSocketServer(httpServer, db.getPool(), {
        cors: {
          origin: isDevelopment 
            ? ['http://localhost:3000', 'http://localhost:3456', 'http://localhost:5173']
            : process.env.ALLOWED_ORIGINS?.split(',') || false,
          credentials: true,
        },
        maxConnections: 1000,
        heartbeatInterval: 15000,
        connectionTimeout: 30000,
        maxMessageSize: 1024 * 1024, // 1MB
        rateLimitWindow: 60000,
        rateLimitMaxRequests: 100,
      });
      
      // Store WebSocket server in app locals for API access
      app.locals.wsServer = wsServer;
      
      console.log('✅ WebSocket server initialized successfully');
    } catch (error) {
      console.error('❌ Failed to initialize WebSocket server:', error);
      console.warn('⚠️  Server will start without WebSocket support');
    }
  }

  // Temporarily disabled GraphQL to focus on REST API
  // if (db) {
  //   try {
  //     await setupGraphQLEndpoint(app, httpServer, db, '/graphql');
  //     console.log('✅ GraphQL endpoint initialized successfully');
  //   } catch (error) {
  //     console.error('❌ Failed to initialize GraphQL endpoint:', error);
  //     console.warn('⚠️  Server will start without GraphQL endpoint');
  //   }
  // }

  const server = httpServer.listen(PORT, () => {
    console.log(`🚀 CrewChief Web UI server running on port ${PORT}`);
    console.log(`📊 Health check: http://localhost:${PORT}/api/health`);
    console.log(`🔍 Maproom API: http://localhost:${PORT}/api/maproom`);
    
    if (db) {
      console.log(`🔐 Authentication: http://localhost:${PORT}/auth`);
      console.log(`🛠️  REST API: http://localhost:${PORT}/api`);
      console.log(`📖 API Docs: http://localhost:${PORT}/api/docs`);
      // console.log(`📈 GraphQL API: http://localhost:${PORT}/graphql`);
    }
    
    if (wsServer) {
      console.log(`🔌 WebSocket server: ws://localhost:${PORT}`);
      console.log(`   • Max connections: 1000`);
      console.log(`   • Heartbeat interval: 15s`);
      console.log(`   • Message size limit: 1MB`);
    }
    
    console.log(`🔧 Environment: ${NODE_ENV}`);
    
    if (isDevelopment) {
      console.log(`🌐 Full endpoints:`);
      console.log(`   • Auth: http://localhost:${PORT}/auth`);
      console.log(`     - Register: POST /auth/register`);
      console.log(`     - Login: POST /auth/login`);
      console.log(`     - OAuth: GET /auth/oauth/{github,google}`);
      console.log(`   • API: http://localhost:${PORT}/api`);
      console.log(`     - Worktrees: /api/worktrees`);
      console.log(`     - Agents: /api/agents`);
      console.log(`     - Runs: /api/runs`);
      console.log(`     - Config: /api/config`);
      if (wsServer) {
        console.log(`   • WebSocket: ws://localhost:${PORT}`);
        console.log(`     - Real-time updates for all entities`);
        console.log(`     - Room-based broadcasting`);
        console.log(`     - Authentication via JWT tokens`);
      }
    }
  });

  // Graceful shutdown
  const shutdown = async () => {
    console.log('Shutting down gracefully...');
    
    // Close WebSocket server first
    if (wsServer) {
      try {
        await wsServer.shutdown();
        console.log('✅ WebSocket server closed');
      } catch (error) {
        console.error('❌ Error closing WebSocket server:', error);
      }
    }
    
    // Close HTTP server
    server.close(() => {
      console.log('✅ HTTP server closed');
      
      // Close database connection
      if (db) {
        try {
          db.close();
          console.log('✅ Database connection closed');
        } catch (error) {
          console.error('❌ Error closing database:', error);
        }
      }
      
      process.exit(0);
    });
  };

  process.on('SIGTERM', shutdown);
  process.on('SIGINT', shutdown);

  return server;
}

// Start server only if this file is run directly
if (import.meta.url === `file://${process.argv[1]}`) {
  startServer().catch(console.error);
}

export default app;