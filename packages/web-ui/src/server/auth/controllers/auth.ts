import { type Request, type Response, type NextFunction } from 'express';
import { validationResult } from 'express-validator';
import { Pool } from 'pg';
import { UserModel, type CreateUserData, type User } from '../models/user.js';
import { JWTService, type TokenPair } from '../services/jwt.js';
import { OAuthService } from '../services/oauth.js';
import { SessionService, type SessionData } from '../services/session.js';
import { RateLimitService } from '../middleware/rate-limit.js';

// Request interfaces
export interface AuthenticatedRequest extends Request {
  user?: {
    id: number;
    uuid: string;
    email: string;
    sessionId: string;
    permissions: string[];
    roles: string[];
  };
  session: any;
}

export interface LoginRequest extends Request {
  body: {
    email: string;
    password: string;
    rememberMe?: boolean;
  };
}

export interface RegisterRequest extends Request {
  body: {
    email: string;
    password: string;
    confirmPassword: string;
    username?: string;
    firstName?: string;
    lastName?: string;
  };
}

export interface RefreshRequest extends Request {
  body: {
    refreshToken: string;
  };
}

export interface ChangePasswordRequest extends AuthenticatedRequest {
  body: {
    currentPassword: string;
    newPassword: string;
    confirmPassword: string;
  };
}

// Response interfaces
interface AuthResponse {
  success: boolean;
  user?: {
    id: number;
    uuid: string;
    email: string;
    username?: string;
    firstName?: string;
    lastName?: string;
    avatarUrl?: string;
    roles: string[];
    permissions: string[];
    isVerified: boolean;
    lastLoginAt?: Date;
  };
  tokens?: {
    accessToken: string;
    refreshToken: string;
    expiresAt: Date;
  };
  message?: string;
}

export class AuthController {
  private userModel: UserModel;
  private jwtService: JWTService;
  private oauthService: OAuthService;
  private sessionService: SessionService;
  private rateLimitService: RateLimitService;

  constructor(private db: Pool) {
    this.userModel = new UserModel(db);
    this.jwtService = new JWTService(db);
    this.oauthService = new OAuthService(db);
    this.sessionService = new SessionService();
    this.rateLimitService = new RateLimitService(db);
  }

  // Initialize services
  async initialize(): Promise<void> {
    await this.sessionService.connect();
  }

  // Register new user
  register = async (req: RegisterRequest, res: Response): Promise<void> => {
    try {
      // Validate input
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        res.status(400).json({
          success: false,
          message: 'Validation failed',
          errors: errors.array(),
        });
        return;
      }

      const { email, password, confirmPassword, username, firstName, lastName } = req.body;

      // Check password confirmation
      if (password !== confirmPassword) {
        res.status(400).json({
          success: false,
          message: 'Passwords do not match',
        });
        return;
      }

      // Check rate limiting
      const isLimited = await this.rateLimitService.checkRateLimit(
        'email',
        email,
        'register',
        5, // max 5 registration attempts per hour
        60 * 60 * 1000 // 1 hour window
      );

      if (isLimited) {
        res.status(429).json({
          success: false,
          message: 'Too many registration attempts. Please try again later.',
        });
        return;
      }

      // Create user
      const userData: CreateUserData = {
        email,
        password,
        username,
        first_name: firstName,
        last_name: lastName,
      };

      const user = await this.userModel.createUser(userData);
      
      // Get user roles and permissions
      const { roles, permissions } = await this.userModel.getUserRolesAndPermissions(user.id);
      const roleNames = roles.map(role => role.name);

      // Create tokens
      const deviceInfo = {
        userAgent: req.headers['user-agent'],
        ip: req.ip,
      };

      const tokens = await this.jwtService.createTokenPair(
        user,
        permissions,
        roleNames,
        deviceInfo,
        req.ip
      );

      // Create session
      const sessionData: SessionData = {
        userId: user.id,
        userUuid: user.uuid,
        email: user.email,
        roles: roleNames,
        permissions,
        ipAddress: req.ip,
        userAgent: req.headers['user-agent'],
      };

      // Set session in Express session store
      req.session.user = sessionData;

      // Set secure HTTP-only cookies
      const isProduction = process.env.NODE_ENV === 'production';
      res.cookie('accessToken', tokens.accessToken, {
        httpOnly: true,
        secure: isProduction,
        sameSite: isProduction ? 'strict' : 'lax',
        maxAge: 24 * 60 * 60 * 1000, // 24 hours
      });

      res.cookie('refreshToken', tokens.refreshToken, {
        httpOnly: true,
        secure: isProduction,
        sameSite: isProduction ? 'strict' : 'lax',
        maxAge: 7 * 24 * 60 * 60 * 1000, // 7 days
      });

      const response: AuthResponse = {
        success: true,
        user: {
          id: user.id,
          uuid: user.uuid,
          email: user.email,
          username: user.username,
          firstName: user.first_name,
          lastName: user.last_name,
          avatarUrl: user.avatar_url,
          roles: roleNames,
          permissions,
          isVerified: user.is_verified,
        },
        tokens: {
          accessToken: tokens.accessToken,
          refreshToken: tokens.refreshToken,
          expiresAt: tokens.accessTokenExpiresAt,
        },
        message: 'Registration successful',
      };

      res.status(201).json(response);

    } catch (error) {
      console.error('Registration error:', error);
      
      if (error instanceof Error) {
        if (error.message.includes('Email already registered') || error.message.includes('Username already taken')) {
          res.status(409).json({
            success: false,
            message: error.message,
          });
          return;
        }
        
        if (error.message.includes('Password must')) {
          res.status(400).json({
            success: false,
            message: error.message,
          });
          return;
        }
      }

      res.status(500).json({
        success: false,
        message: 'Registration failed. Please try again.',
      });
    }
  };

  // Login user
  login = async (req: LoginRequest, res: Response): Promise<void> => {
    try {
      // Validate input
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        res.status(400).json({
          success: false,
          message: 'Validation failed',
          errors: errors.array(),
        });
        return;
      }

      const { email, password, rememberMe } = req.body;

      // Check rate limiting
      const isLimited = await this.rateLimitService.checkRateLimit(
        'email',
        email,
        'login',
        10, // max 10 login attempts per hour
        60 * 60 * 1000 // 1 hour window
      );

      if (isLimited) {
        res.status(429).json({
          success: false,
          message: 'Too many login attempts. Please try again later.',
        });
        return;
      }

      // Authenticate user
      const user = await this.userModel.authenticateUser(email, password, req.ip);
      
      if (!user) {
        // Log failed attempt
        await this.rateLimitService.recordLoginAttempt(
          email,
          null,
          req.ip || '0.0.0.0',
          req.headers['user-agent'] || '',
          'password',
          false,
          'invalid_credentials'
        );

        res.status(401).json({
          success: false,
          message: 'Invalid email or password',
        });
        return;
      }

      // Get user roles and permissions
      const { roles, permissions } = await this.userModel.getUserRolesAndPermissions(user.id);
      const roleNames = roles.map(role => role.name);

      // Create tokens
      const deviceInfo = {
        userAgent: req.headers['user-agent'],
        ip: req.ip,
      };

      const tokens = await this.jwtService.createTokenPair(
        user,
        permissions,
        roleNames,
        deviceInfo,
        req.ip
      );

      // Log successful attempt
      await this.rateLimitService.recordLoginAttempt(
        email,
        user.id,
        req.ip || '0.0.0.0',
        req.headers['user-agent'] || '',
        'password',
        true
      );

      // Create session
      const sessionData: SessionData = {
        userId: user.id,
        userUuid: user.uuid,
        email: user.email,
        roles: roleNames,
        permissions,
        ipAddress: req.ip,
        userAgent: req.headers['user-agent'],
      };

      req.session.user = sessionData;

      // Set secure cookies
      const isProduction = process.env.NODE_ENV === 'production';
      const cookieMaxAge = rememberMe ? 7 * 24 * 60 * 60 * 1000 : 24 * 60 * 60 * 1000;

      res.cookie('accessToken', tokens.accessToken, {
        httpOnly: true,
        secure: isProduction,
        sameSite: isProduction ? 'strict' : 'lax',
        maxAge: 24 * 60 * 60 * 1000, // 24 hours
      });

      res.cookie('refreshToken', tokens.refreshToken, {
        httpOnly: true,
        secure: isProduction,
        sameSite: isProduction ? 'strict' : 'lax',
        maxAge: cookieMaxAge,
      });

      const response: AuthResponse = {
        success: true,
        user: {
          id: user.id,
          uuid: user.uuid,
          email: user.email,
          username: user.username,
          firstName: user.first_name,
          lastName: user.last_name,
          avatarUrl: user.avatar_url,
          roles: roleNames,
          permissions,
          isVerified: user.is_verified,
          lastLoginAt: user.last_login_at,
        },
        tokens: {
          accessToken: tokens.accessToken,
          refreshToken: tokens.refreshToken,
          expiresAt: tokens.accessTokenExpiresAt,
        },
        message: 'Login successful',
      };

      res.json(response);

    } catch (error) {
      console.error('Login error:', error);
      
      if (error instanceof Error && error.message.includes('locked')) {
        res.status(423).json({
          success: false,
          message: error.message,
        });
        return;
      }

      res.status(500).json({
        success: false,
        message: 'Login failed. Please try again.',
      });
    }
  };

  // Refresh tokens
  refresh = async (req: RefreshRequest, res: Response): Promise<void> => {
    try {
      const { refreshToken } = req.body;
      
      if (!refreshToken) {
        res.status(400).json({
          success: false,
          message: 'Refresh token is required',
        });
        return;
      }

      // Refresh tokens
      const deviceInfo = {
        userAgent: req.headers['user-agent'],
        ip: req.ip,
      };

      const tokens = await this.jwtService.refreshTokens(
        refreshToken,
        deviceInfo,
        req.ip
      );

      // Update cookies
      const isProduction = process.env.NODE_ENV === 'production';
      
      res.cookie('accessToken', tokens.accessToken, {
        httpOnly: true,
        secure: isProduction,
        sameSite: isProduction ? 'strict' : 'lax',
        maxAge: 24 * 60 * 60 * 1000,
      });

      if (tokens.refreshToken !== refreshToken) {
        // New refresh token was issued
        res.cookie('refreshToken', tokens.refreshToken, {
          httpOnly: true,
          secure: isProduction,
          sameSite: isProduction ? 'strict' : 'lax',
          maxAge: 7 * 24 * 60 * 60 * 1000,
        });
      }

      res.json({
        success: true,
        tokens: {
          accessToken: tokens.accessToken,
          refreshToken: tokens.refreshToken,
          expiresAt: tokens.accessTokenExpiresAt,
        },
        message: 'Tokens refreshed successfully',
      });

    } catch (error) {
      console.error('Token refresh error:', error);
      
      res.status(401).json({
        success: false,
        message: error instanceof Error ? error.message : 'Token refresh failed',
      });
    }
  };

  // Logout user
  logout = async (req: AuthenticatedRequest, res: Response): Promise<void> => {
    try {
      const refreshToken = req.cookies.refreshToken;
      
      if (refreshToken) {
        // Revoke refresh token
        await this.jwtService.revokeRefreshToken(refreshToken, 'user_logout');
      }

      // Destroy session
      if (req.session) {
        req.session.destroy((err) => {
          if (err) {
            console.error('Session destruction error:', err);
          }
        });
      }

      // Clear cookies
      res.clearCookie('accessToken');
      res.clearCookie('refreshToken');

      res.json({
        success: true,
        message: 'Logout successful',
      });

    } catch (error) {
      console.error('Logout error:', error);
      
      res.status(500).json({
        success: false,
        message: 'Logout failed',
      });
    }
  };

  // Logout from all devices
  logoutAll = async (req: AuthenticatedRequest, res: Response): Promise<void> => {
    try {
      const userId = req.user?.id;
      
      if (!userId) {
        res.status(401).json({
          success: false,
          message: 'Authentication required',
        });
        return;
      }

      // Revoke all refresh tokens
      await this.jwtService.revokeAllUserTokens(userId, 'logout_all');

      // Destroy all sessions
      await this.sessionService.destroyAllUserSessions(userId);

      // Clear current cookies
      res.clearCookie('accessToken');
      res.clearCookie('refreshToken');

      res.json({
        success: true,
        message: 'Logged out from all devices successfully',
      });

    } catch (error) {
      console.error('Logout all error:', error);
      
      res.status(500).json({
        success: false,
        message: 'Logout from all devices failed',
      });
    }
  };

  // Get current user
  me = async (req: AuthenticatedRequest, res: Response): Promise<void> => {
    try {
      const userId = req.user?.id;
      
      if (!userId) {
        res.status(401).json({
          success: false,
          message: 'Authentication required',
        });
        return;
      }

      const user = await this.userModel.getUserById(userId);
      
      if (!user) {
        res.status(404).json({
          success: false,
          message: 'User not found',
        });
        return;
      }

      const { roles, permissions } = await this.userModel.getUserRolesAndPermissions(user.id);
      const roleNames = roles.map(role => role.name);

      res.json({
        success: true,
        user: {
          id: user.id,
          uuid: user.uuid,
          email: user.email,
          username: user.username,
          firstName: user.first_name,
          lastName: user.last_name,
          avatarUrl: user.avatar_url,
          roles: roleNames,
          permissions,
          isVerified: user.is_verified,
          lastLoginAt: user.last_login_at,
          createdAt: user.created_at,
        },
      });

    } catch (error) {
      console.error('Get user error:', error);
      
      res.status(500).json({
        success: false,
        message: 'Failed to get user information',
      });
    }
  };

  // Change password
  changePassword = async (req: ChangePasswordRequest, res: Response): Promise<void> => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        res.status(400).json({
          success: false,
          message: 'Validation failed',
          errors: errors.array(),
        });
        return;
      }

      const userId = req.user?.id;
      const { currentPassword, newPassword, confirmPassword } = req.body;

      if (!userId) {
        res.status(401).json({
          success: false,
          message: 'Authentication required',
        });
        return;
      }

      if (newPassword !== confirmPassword) {
        res.status(400).json({
          success: false,
          message: 'New passwords do not match',
        });
        return;
      }

      // Change password
      await this.userModel.changePassword(userId, currentPassword, newPassword);

      // Revoke all other sessions for security
      await this.jwtService.revokeAllUserTokens(userId, 'password_changed');
      await this.sessionService.destroyAllUserSessions(userId);

      res.json({
        success: true,
        message: 'Password changed successfully. Please log in again.',
      });

    } catch (error) {
      console.error('Change password error:', error);
      
      if (error instanceof Error && error.message.includes('incorrect')) {
        res.status(400).json({
          success: false,
          message: error.message,
        });
        return;
      }

      res.status(500).json({
        success: false,
        message: 'Failed to change password',
      });
    }
  };
}