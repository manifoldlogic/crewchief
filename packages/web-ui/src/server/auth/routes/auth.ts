import { Router } from 'express';
import passport from 'passport';
import cookieParser from 'cookie-parser';
import { Pool } from 'pg';
import { AuthController } from '../controllers/auth.js';
import { SessionService } from '../services/session.js';
import {
  loginValidation,
  registerValidation,
  refreshValidation,
  changePasswordValidation,
} from '../utils/validation.js';
import {
  loginRateLimit,
  registerRateLimit,
  refreshRateLimit,
  createAccountLockout,
  createIPSecurityCheck,
  createCSRFProtection,
} from '../middleware/rate-limit.js';
import { authenticateJWT, requirePermissions } from '../../api/middleware/auth.js';

export function createAuthRouter(db: Pool): Router {
  const router = Router();
  const authController = new AuthController(db);
  const sessionService = new SessionService();

  // Initialize auth controller
  authController.initialize().catch(console.error);

  // Middleware
  router.use(cookieParser());
  router.use(createIPSecurityCheck());
  router.use(createCSRFProtection());

  // Session middleware
  router.use(sessionService.getSessionMiddleware());

  // Initialize Passport
  router.use(passport.initialize());
  router.use(passport.session());

  // Passport serialization (for OAuth)
  passport.serializeUser((user: any, done) => {
    done(null, user.id);
  });

  passport.deserializeUser(async (id: number, done) => {
    try {
      const userModel = new (await import('../models/user.js')).UserModel(db);
      const user = await userModel.getUserById(id);
      done(null, user);
    } catch (error) {
      done(error, null);
    }
  });

  // Health check endpoint
  router.get('/health', (req, res) => {
    res.json({
      success: true,
      message: 'Authentication service is healthy',
      timestamp: new Date().toISOString(),
    });
  });

  // Authentication endpoints
  router.post('/register', 
    registerRateLimit,
    registerValidation,
    authController.register
  );

  router.post('/login',
    loginRateLimit,
    createAccountLockout(db),
    loginValidation,
    authController.login
  );

  router.post('/refresh',
    refreshRateLimit,
    refreshValidation,
    authController.refresh
  );

  router.post('/logout',
    authenticateJWT(),
    authController.logout
  );

  router.post('/logout-all',
    authenticateJWT(),
    authController.logoutAll
  );

  // User profile endpoints
  router.get('/me',
    authenticateJWT(),
    authController.me
  );

  router.put('/change-password',
    authenticateJWT(),
    changePasswordValidation,
    authController.changePassword
  );

  // OAuth routes
  const setupOAuthRoutes = async () => {
    try {
      const { OAuthService } = await import('../services/oauth.js');
      const oauthService = new OAuthService(db);
      
      // GitHub OAuth
      router.get('/oauth/github',
        passport.authenticate('github', { scope: ['user:email'] })
      );

      router.get('/oauth/github/callback',
        passport.authenticate('github', { failureRedirect: '/login?error=oauth_failed' }),
        async (req, res) => {
          try {
            const user = req.user as any;
            
            if (!user) {
              return res.redirect('/login?error=oauth_failed');
            }

            // Get user roles and permissions
            const userModel = new (await import('../models/user.js')).UserModel(db);
            const { roles, permissions } = await userModel.getUserRolesAndPermissions(user.id);
            const roleNames = roles.map(role => role.name);

            // Create JWT tokens
            const jwtService = new (await import('../services/jwt.js')).JWTService(db);
            const deviceInfo = {
              userAgent: req.headers['user-agent'],
              ip: req.ip,
            };

            const tokens = await jwtService.createTokenPair(
              user,
              permissions,
              roleNames,
              deviceInfo,
              req.ip
            );

            // Set secure cookies
            const isProduction = process.env.NODE_ENV === 'production';
            res.cookie('accessToken', tokens.accessToken, {
              httpOnly: true,
              secure: isProduction,
              sameSite: isProduction ? 'strict' : 'lax',
              maxAge: 24 * 60 * 60 * 1000,
            });

            res.cookie('refreshToken', tokens.refreshToken, {
              httpOnly: true,
              secure: isProduction,
              sameSite: isProduction ? 'strict' : 'lax',
              maxAge: 7 * 24 * 60 * 60 * 1000,
            });

            // Redirect to dashboard or intended page
            const redirectUrl = req.session.redirectTo || '/dashboard';
            delete req.session.redirectTo;
            
            res.redirect(redirectUrl);

          } catch (error) {
            console.error('GitHub OAuth callback error:', error);
            res.redirect('/login?error=oauth_callback_failed');
          }
        }
      );

      // Google OAuth
      router.get('/oauth/google',
        passport.authenticate('google', { scope: ['profile', 'email'] })
      );

      router.get('/oauth/google/callback',
        passport.authenticate('google', { failureRedirect: '/login?error=oauth_failed' }),
        async (req, res) => {
          try {
            const user = req.user as any;
            
            if (!user) {
              return res.redirect('/login?error=oauth_failed');
            }

            // Same logic as GitHub callback
            const userModel = new (await import('../models/user.js')).UserModel(db);
            const { roles, permissions } = await userModel.getUserRolesAndPermissions(user.id);
            const roleNames = roles.map(role => role.name);

            const jwtService = new (await import('../services/jwt.js')).JWTService(db);
            const deviceInfo = {
              userAgent: req.headers['user-agent'],
              ip: req.ip,
            };

            const tokens = await jwtService.createTokenPair(
              user,
              permissions,
              roleNames,
              deviceInfo,
              req.ip
            );

            const isProduction = process.env.NODE_ENV === 'production';
            res.cookie('accessToken', tokens.accessToken, {
              httpOnly: true,
              secure: isProduction,
              sameSite: isProduction ? 'strict' : 'lax',
              maxAge: 24 * 60 * 60 * 1000,
            });

            res.cookie('refreshToken', tokens.refreshToken, {
              httpOnly: true,
              secure: isProduction,
              sameSite: isProduction ? 'strict' : 'lax',
              maxAge: 7 * 24 * 60 * 60 * 1000,
            });

            const redirectUrl = req.session.redirectTo || '/dashboard';
            delete req.session.redirectTo;
            
            res.redirect(redirectUrl);

          } catch (error) {
            console.error('Google OAuth callback error:', error);
            res.redirect('/login?error=oauth_callback_failed');
          }
        }
      );

      // OAuth account management
      router.get('/oauth/accounts',
        authenticateJWT(),
        async (req, res) => {
          try {
            const userId = (req as any).user?.id;
            if (!userId) {
              return res.status(401).json({
                success: false,
                message: 'Authentication required',
              });
            }

            const accounts = await oauthService.getUserOAuthAccounts(userId);
            
            res.json({
              success: true,
              accounts: accounts.map(account => ({
                id: account.id,
                provider: account.provider_name,
                providerDisplayName: account.provider_display_name,
                providerUsername: account.provider_username,
                providerEmail: account.provider_email,
                connectedAt: account.created_at,
                lastLoginAt: account.last_login_at,
              })),
            });

          } catch (error) {
            console.error('Get OAuth accounts error:', error);
            res.status(500).json({
              success: false,
              message: 'Failed to get OAuth accounts',
            });
          }
        }
      );

      router.delete('/oauth/accounts/:providerId',
        authenticateJWT(),
        async (req, res) => {
          try {
            const userId = (req as any).user?.id;
            const providerId = parseInt(req.params.providerId);

            if (!userId) {
              return res.status(401).json({
                success: false,
                message: 'Authentication required',
              });
            }

            const success = await oauthService.unlinkOAuthAccount(userId, providerId);
            
            if (success) {
              res.json({
                success: true,
                message: 'OAuth account unlinked successfully',
              });
            } else {
              res.status(404).json({
                success: false,
                message: 'OAuth account not found',
              });
            }

          } catch (error) {
            console.error('Unlink OAuth account error:', error);
            
            if (error instanceof Error && error.message.includes('Cannot unlink')) {
              res.status(400).json({
                success: false,
                message: error.message,
              });
            } else {
              res.status(500).json({
                success: false,
                message: 'Failed to unlink OAuth account',
              });
            }
          }
        }
      );

    } catch (error) {
      console.error('Failed to setup OAuth routes:', error);
    }
  };

  // Setup OAuth routes asynchronously
  setupOAuthRoutes();

  // Session management endpoints
  router.get('/sessions',
    authenticateJWT(),
    async (req, res) => {
      try {
        const userId = (req as any).user?.id;
        if (!userId) {
          return res.status(401).json({
            success: false,
            message: 'Authentication required',
          });
        }

        const sessions = await sessionService.getUserSessions(userId);
        
        res.json({
          success: true,
          sessions: sessions.map(session => ({
            sessionId: session.sessionId,
            ipAddress: session.ipAddress,
            userAgent: session.userAgent,
            createdAt: session.createdAt,
            lastAccessed: session.lastAccessed,
            expiresAt: session.expiresAt,
            isCurrent: session.sessionId === req.sessionID,
          })),
        });

      } catch (error) {
        console.error('Get sessions error:', error);
        res.status(500).json({
          success: false,
          message: 'Failed to get sessions',
        });
      }
    }
  );

  router.delete('/sessions/:sessionId',
    authenticateJWT(),
    async (req, res) => {
      try {
        const userId = (req as any).user?.id;
        const sessionId = req.params.sessionId;

        if (!userId) {
          return res.status(401).json({
            success: false,
            message: 'Authentication required',
          });
        }

        // Verify session belongs to user
        const sessions = await sessionService.getUserSessions(userId);
        const sessionExists = sessions.some(s => s.sessionId === sessionId);

        if (!sessionExists) {
          return res.status(404).json({
            success: false,
            message: 'Session not found',
          });
        }

        await sessionService.destroySession(sessionId);
        
        res.json({
          success: true,
          message: 'Session terminated successfully',
        });

      } catch (error) {
        console.error('Terminate session error:', error);
        res.status(500).json({
          success: false,
          message: 'Failed to terminate session',
        });
      }
    }
  );

  // Admin endpoints
  router.get('/admin/stats',
    authenticateJWT(),
    requirePermissions(['admin']),
    async (req, res) => {
      try {
        const jwtService = new (await import('../services/jwt.js')).JWTService(db);
        const tokenStats = await jwtService.getTokenStatistics();
        const sessionStats = await sessionService.getSessionStatistics();

        res.json({
          success: true,
          stats: {
            tokens: tokenStats,
            sessions: sessionStats,
          },
        });

      } catch (error) {
        console.error('Get admin stats error:', error);
        res.status(500).json({
          success: false,
          message: 'Failed to get authentication statistics',
        });
      }
    }
  );

  // Error handler
  router.use((err: Error, req: any, res: any, next: any) => {
    console.error('Auth router error:', err);
    
    res.status(500).json({
      success: false,
      message: 'Authentication service error',
      ...(process.env.NODE_ENV === 'development' && { error: err.message }),
    });
  });

  return router;
}