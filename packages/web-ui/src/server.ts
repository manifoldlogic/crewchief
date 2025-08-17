import express, { type Express } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import morgan from 'morgan';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import maproomRoutes from './routes/maproom.js';
import { initializeDatabase } from './db/connection.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const PORT = process.env.PORT ? parseInt(process.env.PORT, 10) : 3456;
const NODE_ENV = process.env.NODE_ENV || 'development';
const isDevelopment = NODE_ENV === 'development';

// Create Express app
const app: Express = express();

// Security middleware
app.use(helmet({
  contentSecurityPolicy: isDevelopment ? false : undefined,
}));

// CORS configuration
app.use(cors({
  origin: isDevelopment ? ['http://localhost:3000', 'http://localhost:3456'] : false,
  credentials: true,
}));

// Compression middleware
app.use(compression());

// Logging middleware
app.use(morgan(isDevelopment ? 'dev' : 'combined'));

// Body parsing middleware
app.use(express.json({ limit: '10mb' }));
app.use(express.urlencoded({ extended: true, limit: '10mb' }));

// Health check endpoint
app.get('/api/health', (req, res) => {
  res.json({
    status: 'ok',
    timestamp: new Date().toISOString(),
    uptime: process.uptime(),
    version: process.env.npm_package_version || '0.1.0',
    environment: NODE_ENV,
  });
});

// API routes
app.use('/api/maproom', maproomRoutes);

// API routes placeholder
app.get('/api', (req, res) => {
  res.json({
    message: 'CrewChief Web UI API',
    version: '0.1.0',
    endpoints: {
      health: '/api/health',
      maproom: '/api/maproom',
    },
  });
});

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

// Catch-all handler for frontend routing (SPA support)
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

// Handle 404
app.use((req, res) => {
  res.status(404).json({
    error: 'Not Found',
    path: req.path,
    method: req.method,
  });
});

// Function to start the server
export async function startServer() {
  try {
    // Initialize database connection
    await initializeDatabase();
    console.log('✅ Database initialized successfully');
  } catch (error) {
    console.error('❌ Failed to initialize database:', error);
    console.warn('⚠️  Server will start without database connection');
  }

  const server = app.listen(PORT, () => {
    console.log(`🚀 CrewChief Web UI server running on port ${PORT}`);
    console.log(`📊 Health check: http://localhost:${PORT}/api/health`);
    console.log(`🔍 Maproom API: http://localhost:${PORT}/api/maproom`);
    console.log(`🔧 Environment: ${NODE_ENV}`);
    
    if (isDevelopment) {
      console.log(`🌐 API base: http://localhost:${PORT}/api`);
    }
  });

  // Graceful shutdown
  process.on('SIGTERM', () => {
    console.log('SIGTERM received, shutting down gracefully');
    server.close(() => {
      console.log('Server closed');
      process.exit(0);
    });
  });

  process.on('SIGINT', () => {
    console.log('SIGINT received, shutting down gracefully');
    server.close(() => {
      console.log('Server closed');
      process.exit(0);
    });
  });

  return server;
}

// Start server only if this file is run directly
if (import.meta.url === `file://${process.argv[1]}`) {
  startServer().catch(console.error);
}

export default app;