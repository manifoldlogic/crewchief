import { Router } from 'express';
import { type Pool } from 'pg';
import swaggerJsdoc from 'swagger-jsdoc';
import swaggerUi from 'swagger-ui-express';
import { createWorktreeRoutes } from './routes/worktrees.js';
import { createAgentRoutes } from './routes/agents.js';
import { createConfigRoutes } from './routes/config.js';
import { createRunsRoutes } from './routes/runs.js';
import {
  addRequestId,
  responseTime,
  setApiVersionHeaders,
  setSecurityHeaders,
  sendApiInfo,
  sendHealthCheck,
} from './utils/responses.js';
import { standardRateLimit } from './middleware/rate-limit.js';
import { authenticateRequest, optionalAuthentication } from './middleware/auth.js';

// OpenAPI/Swagger configuration
const swaggerOptions = {
  definition: {
    openapi: '3.0.0',
    info: {
      title: 'CrewChief Web UI API',
      version: '1.0.0',
      description: 'REST API for managing CrewChief operations, agents, worktrees, and configurations',
      contact: {
        name: 'CrewChief Team',
        url: 'https://github.com/manifoldlogic/crewchief',
        email: 'support@crewchief.dev',
      },
      license: {
        name: 'MIT',
        url: 'https://opensource.org/licenses/MIT',
      },
    },
    servers: [
      {
        url: '/api',
        description: 'API Server',
      },
    ],
    components: {
      securitySchemes: {
        ApiKeyAuth: {
          type: 'apiKey',
          in: 'header',
          name: 'x-api-key',
          description: 'API key for authentication',
        },
        BearerAuth: {
          type: 'http',
          scheme: 'bearer',
          bearerFormat: 'JWT',
          description: 'JWT token for authentication',
        },
      },
      schemas: {
        Error: {
          type: 'object',
          required: ['success', 'error'],
          properties: {
            success: {
              type: 'boolean',
              example: false,
            },
            error: {
              type: 'object',
              required: ['code', 'message'],
              properties: {
                code: {
                  type: 'string',
                  example: 'VALIDATION_ERROR',
                },
                message: {
                  type: 'string',
                  example: 'Request validation failed',
                },
                details: {
                  type: 'object',
                  description: 'Additional error details',
                },
              },
            },
            requestId: {
              type: 'string',
              example: 'req_1234567890_abc123',
            },
          },
        },
        Success: {
          type: 'object',
          required: ['success', 'data'],
          properties: {
            success: {
              type: 'boolean',
              example: true,
            },
            data: {
              type: 'object',
              description: 'Response data',
            },
            message: {
              type: 'string',
              example: 'Operation completed successfully',
            },
          },
        },
        PaginatedResponse: {
          type: 'object',
          required: ['success', 'data'],
          properties: {
            success: {
              type: 'boolean',
              example: true,
            },
            data: {
              type: 'object',
              required: ['items', 'pagination'],
              properties: {
                items: {
                  type: 'array',
                  items: {
                    type: 'object',
                  },
                },
                pagination: {
                  type: 'object',
                  required: ['total', 'limit', 'offset', 'hasMore'],
                  properties: {
                    total: {
                      type: 'integer',
                      minimum: 0,
                    },
                    limit: {
                      type: 'integer',
                      minimum: 1,
                      maximum: 100,
                    },
                    offset: {
                      type: 'integer',
                      minimum: 0,
                    },
                    hasMore: {
                      type: 'boolean',
                    },
                    nextCursor: {
                      type: 'string',
                      nullable: true,
                    },
                    prevCursor: {
                      type: 'string',
                      nullable: true,
                    },
                  },
                },
              },
            },
            message: {
              type: 'string',
            },
          },
        },
      },
      parameters: {
        LimitParam: {
          name: 'limit',
          in: 'query',
          description: 'Number of items to return (1-100)',
          required: false,
          schema: {
            type: 'integer',
            minimum: 1,
            maximum: 100,
            default: 20,
          },
        },
        OffsetParam: {
          name: 'offset',
          in: 'query',
          description: 'Number of items to skip',
          required: false,
          schema: {
            type: 'integer',
            minimum: 0,
            default: 0,
          },
        },
        SortParam: {
          name: 'sort',
          in: 'query',
          description: 'Field to sort by',
          required: false,
          schema: {
            type: 'string',
          },
        },
        OrderParam: {
          name: 'order',
          in: 'query',
          description: 'Sort order',
          required: false,
          schema: {
            type: 'string',
            enum: ['asc', 'desc'],
            default: 'desc',
          },
        },
      },
      responses: {
        BadRequest: {
          description: 'Bad Request',
          content: {
            'application/json': {
              schema: {
                $ref: '#/components/schemas/Error',
              },
              example: {
                success: false,
                error: {
                  code: 'BAD_REQUEST',
                  message: 'Invalid request parameters',
                },
                requestId: 'req_1234567890_abc123',
              },
            },
          },
        },
        Unauthorized: {
          description: 'Unauthorized',
          content: {
            'application/json': {
              schema: {
                $ref: '#/components/schemas/Error',
              },
              example: {
                success: false,
                error: {
                  code: 'UNAUTHORIZED',
                  message: 'Authentication required',
                },
                requestId: 'req_1234567890_abc123',
              },
            },
          },
        },
        Forbidden: {
          description: 'Forbidden',
          content: {
            'application/json': {
              schema: {
                $ref: '#/components/schemas/Error',
              },
              example: {
                success: false,
                error: {
                  code: 'FORBIDDEN',
                  message: 'Insufficient permissions',
                },
                requestId: 'req_1234567890_abc123',
              },
            },
          },
        },
        NotFound: {
          description: 'Not Found',
          content: {
            'application/json': {
              schema: {
                $ref: '#/components/schemas/Error',
              },
              example: {
                success: false,
                error: {
                  code: 'NOT_FOUND',
                  message: 'Resource not found',
                },
                requestId: 'req_1234567890_abc123',
              },
            },
          },
        },
        RateLimitExceeded: {
          description: 'Rate Limit Exceeded',
          content: {
            'application/json': {
              schema: {
                $ref: '#/components/schemas/Error',
              },
              example: {
                success: false,
                error: {
                  code: 'RATE_LIMIT_EXCEEDED',
                  message: 'Too many requests. Please try again later.',
                },
                requestId: 'req_1234567890_abc123',
              },
            },
          },
        },
        InternalServerError: {
          description: 'Internal Server Error',
          content: {
            'application/json': {
              schema: {
                $ref: '#/components/schemas/Error',
              },
              example: {
                success: false,
                error: {
                  code: 'INTERNAL_ERROR',
                  message: 'Internal server error',
                },
                requestId: 'req_1234567890_abc123',
              },
            },
          },
        },
      },
    },
    security: [
      {
        ApiKeyAuth: [],
      },
      {
        BearerAuth: [],
      },
    ],
    tags: [
      {
        name: 'Worktrees',
        description: 'Git worktree management operations',
      },
      {
        name: 'Agents',
        description: 'AI agent run and message management',
      },
      {
        name: 'Runs',
        description: 'Agent run execution and lifecycle management',
      },
      {
        name: 'Configuration',
        description: 'User preferences and system configuration',
      },
      {
        name: 'System',
        description: 'System information and health checks',
      },
    ],
  },
  apis: [
    './src/server/api/routes/*.ts', // Path to the API route files
    './src/server/api/schemas/*.ts', // Path to the schema files
  ],
};

const swaggerSpec = swaggerJsdoc(swaggerOptions);

export function createApiRouter(pool: Pool): Router {
  const router = Router();

  // Global middleware for all API routes
  router.use(addRequestId());
  router.use(responseTime());
  router.use(setApiVersionHeaders);
  router.use(setSecurityHeaders);

  /**
   * @swagger
   * /:
   *   get:
   *     summary: Get API information
   *     description: Returns basic information about the API including available endpoints
   *     tags: [System]
   *     security: []
   *     responses:
   *       200:
   *         description: API information retrieved successfully
   *         content:
   *           application/json:
   *             schema:
   *               $ref: '#/components/schemas/Success'
   */
  router.get('/', optionalAuthentication(), (req, res) => {
    sendApiInfo(res);
  });

  /**
   * @swagger
   * /health:
   *   get:
   *     summary: Health check
   *     description: Returns the health status of the API and its dependencies
   *     tags: [System]
   *     security: []
   *     responses:
   *       200:
   *         description: System is healthy
   *         content:
   *           application/json:
   *             schema:
   *               $ref: '#/components/schemas/Success'
   */
  router.get('/health', optionalAuthentication(), async (req, res) => {
    const checks: Record<string, any> = {};

    // Database health check
    try {
      const dbResult = await pool.query('SELECT NOW() as timestamp');
      checks.database = {
        status: 'healthy',
        response_time_ms: Date.now(),
        timestamp: dbResult.rows[0].timestamp,
      };
    } catch (error) {
      checks.database = {
        status: 'unhealthy',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Memory usage check
    const memUsage = process.memoryUsage();
    checks.memory = {
      status: memUsage.heapUsed < 500 * 1024 * 1024 ? 'healthy' : 'warning', // 500MB threshold
      heap_used_mb: Math.round(memUsage.heapUsed / 1024 / 1024),
      heap_total_mb: Math.round(memUsage.heapTotal / 1024 / 1024),
      external_mb: Math.round(memUsage.external / 1024 / 1024),
    };

    sendHealthCheck(res, checks);
  });

  // API Documentation
  router.use('/docs', swaggerUi.serve);
  router.get('/docs', swaggerUi.setup(swaggerSpec, {
    customCss: '.swagger-ui .topbar { display: none }',
    customSiteTitle: 'CrewChief API Documentation',
    swaggerOptions: {
      persistAuthorization: true,
      displayRequestDuration: true,
    },
  }));

  // Serve OpenAPI spec as JSON
  router.get('/docs/openapi.json', (req, res) => {
    res.setHeader('Content-Type', 'application/json');
    res.json(swaggerSpec);
  });

  // Resource routes
  router.use('/worktrees', createWorktreeRoutes(pool));
  router.use('/agents', createAgentRoutes(pool));
  router.use('/runs', createRunsRoutes(pool));
  router.use('/config', createConfigRoutes(pool));

  // Catch-all for undefined API routes
  router.use('*', standardRateLimit, (req, res) => {
    res.status(404).json({
      success: false,
      error: {
        code: 'NOT_FOUND',
        message: `API endpoint not found: ${req.method} ${req.originalUrl}`,
        details: {
          available_endpoints: [
            'GET /api',
            'GET /api/health',
            'GET /api/docs',
            'GET /api/worktrees',
            'GET /api/agents',
            'GET /api/runs',
            'GET /api/config',
          ],
        },
      },
      requestId: req.headers['x-request-id'] as string,
    });
  });

  return router;
}

// Export the OpenAPI specification for external use
export { swaggerSpec };