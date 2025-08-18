import jwt from 'jsonwebtoken';
import crypto from 'crypto';
import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';
import type { User } from '../models/user.js';

// JWT Configuration
const JWT_SECRET = process.env.JWT_SECRET || 'dev-jwt-secret-change-in-production';
const JWT_REFRESH_SECRET = process.env.JWT_REFRESH_SECRET || 'dev-refresh-secret-change-in-production';
const JWT_ISSUER = 'crewchief-web-ui';
const JWT_AUDIENCE = 'crewchief-api';

// Token expiration times
const ACCESS_TOKEN_EXPIRES_IN = '24h'; // 24 hours as required
const REFRESH_TOKEN_EXPIRES_IN = '7d'; // 7 days
const REFRESH_TOKEN_ROTATION_THRESHOLD = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

// Token payload interfaces
export interface AccessTokenPayload {
  userId: number;
  userUuid: string;
  email: string;
  sessionId: string;
  permissions: string[];
  roles: string[];
  iat?: number;
  exp?: number;
  iss?: string;
  aud?: string;
}

export interface RefreshTokenPayload {
  userId: number;
  sessionId: string;
  familyId: string;
  tokenId: string;
  iat?: number;
  exp?: number;
  iss?: string;
  aud?: string;
}

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  accessTokenExpiresAt: Date;
  refreshTokenExpiresAt: Date;
}

export interface RefreshTokenInfo {
  id: number;
  token_hash: string;
  user_id: number;
  family_id: string;
  device_info: Record<string, any>;
  ip_address?: string;
  expires_at: Date;
  revoked_at?: Date;
  created_at: Date;
  last_used_at: Date;
}

export class JWTService {
  constructor(private db: Pool) {}

  // Generate access token
  generateAccessToken(user: User, permissions: string[], roles: string[], sessionId: string): string {
    const payload: AccessTokenPayload = {
      userId: user.id,
      userUuid: user.uuid,
      email: user.email,
      sessionId,
      permissions,
      roles,
    };

    return jwt.sign(payload, JWT_SECRET, {
      expiresIn: ACCESS_TOKEN_EXPIRES_IN,
      issuer: JWT_ISSUER,
      audience: JWT_AUDIENCE,
      subject: user.uuid,
    });
  }

  // Generate refresh token
  private generateRefreshToken(userId: number, sessionId: string, familyId: string): string {
    const tokenId = uuidv4();
    
    const payload: RefreshTokenPayload = {
      userId,
      sessionId,
      familyId,
      tokenId,
    };

    return jwt.sign(payload, JWT_REFRESH_SECRET, {
      expiresIn: REFRESH_TOKEN_EXPIRES_IN,
      issuer: JWT_ISSUER,
      audience: JWT_AUDIENCE,
    });
  }

  // Create token pair (access + refresh)
  async createTokenPair(
    user: User, 
    permissions: string[], 
    roles: string[], 
    deviceInfo: Record<string, any> = {},
    ipAddress?: string
  ): Promise<TokenPair> {
    const sessionId = uuidv4();
    const familyId = uuidv4();
    
    // Generate tokens
    const accessToken = this.generateAccessToken(user, permissions, roles, sessionId);
    const refreshToken = this.generateRefreshToken(user.id, sessionId, familyId);
    
    // Calculate expiration dates
    const accessTokenExpiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000); // 24 hours
    const refreshTokenExpiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000); // 7 days
    
    // Hash and store refresh token
    const tokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');
    
    await this.db.query(`
      INSERT INTO auth_refresh_tokens (
        token_hash, user_id, family_id, device_info, ip_address, expires_at
      ) VALUES ($1, $2, $3, $4, $5, $6)
    `, [
      tokenHash,
      user.id,
      familyId,
      JSON.stringify(deviceInfo),
      ipAddress || null,
      refreshTokenExpiresAt
    ]);
    
    return {
      accessToken,
      refreshToken,
      accessTokenExpiresAt,
      refreshTokenExpiresAt,
    };
  }

  // Verify access token
  verifyAccessToken(token: string): AccessTokenPayload | null {
    try {
      const decoded = jwt.verify(token, JWT_SECRET, {
        issuer: JWT_ISSUER,
        audience: JWT_AUDIENCE,
      }) as AccessTokenPayload;
      
      return decoded;
    } catch (error) {
      if (error instanceof jwt.TokenExpiredError) {
        throw new Error('Access token has expired');
      } else if (error instanceof jwt.JsonWebTokenError) {
        throw new Error('Invalid access token');
      } else {
        throw new Error('Token verification failed');
      }
    }
  }

  // Verify refresh token
  private verifyRefreshToken(token: string): RefreshTokenPayload | null {
    try {
      const decoded = jwt.verify(token, JWT_REFRESH_SECRET, {
        issuer: JWT_ISSUER,
        audience: JWT_AUDIENCE,
      }) as RefreshTokenPayload;
      
      return decoded;
    } catch (error) {
      if (error instanceof jwt.TokenExpiredError) {
        throw new Error('Refresh token has expired');
      } else if (error instanceof jwt.JsonWebTokenError) {
        throw new Error('Invalid refresh token');
      } else {
        throw new Error('Refresh token verification failed');
      }
    }
  }

  // Refresh token rotation
  async refreshTokens(
    refreshToken: string,
    deviceInfo: Record<string, any> = {},
    ipAddress?: string
  ): Promise<TokenPair> {
    const client = await this.db.connect();
    
    try {
      await client.query('BEGIN');
      
      // Verify the refresh token
      const tokenPayload = this.verifyRefreshToken(refreshToken);
      if (!tokenPayload) {
        throw new Error('Invalid refresh token');
      }
      
      // Hash the provided token
      const tokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');
      
      // Get the stored refresh token
      const storedTokenResult = await client.query(`
        SELECT * FROM auth_refresh_tokens 
        WHERE token_hash = $1 AND revoked_at IS NULL
      `, [tokenHash]);
      
      if (storedTokenResult.rows.length === 0) {
        // Token not found or already revoked - possible token reuse attack
        // Revoke entire token family
        await client.query(`
          UPDATE auth_refresh_tokens 
          SET revoked_at = NOW(), revoked_reason = 'token_reuse_detected'
          WHERE family_id = $1 AND revoked_at IS NULL
        `, [tokenPayload.familyId]);
        
        await client.query('COMMIT');
        throw new Error('Refresh token not found or already used. Token family revoked for security.');
      }
      
      const storedToken = storedTokenResult.rows[0] as RefreshTokenInfo;
      
      // Check if token is expired
      if (new Date(storedToken.expires_at) < new Date()) {
        await client.query(`
          UPDATE auth_refresh_tokens 
          SET revoked_at = NOW(), revoked_reason = 'expired'
          WHERE id = $1
        `, [storedToken.id]);
        
        await client.query('COMMIT');
        throw new Error('Refresh token has expired');
      }
      
      // Check if user exists and is active
      const userResult = await client.query(`
        SELECT u.*, 
               array_agg(DISTINCT r.name) as role_names,
               array_agg(DISTINCT p.permission) as permissions
        FROM auth_users u
        LEFT JOIN auth_user_roles ur ON u.id = ur.user_id 
          AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
        LEFT JOIN auth_roles r ON ur.role_id = r.id
        LEFT JOIN LATERAL unnest(r.permissions) as p(permission) ON true
        WHERE u.id = $1 AND u.is_active = true AND u.is_locked = false
        GROUP BY u.id
      `, [storedToken.user_id]);
      
      if (userResult.rows.length === 0) {
        throw new Error('User not found or account is disabled');
      }
      
      const user = userResult.rows[0];
      const permissions = user.permissions?.filter(Boolean) || [];
      const roles = user.role_names?.filter(Boolean) || [];
      
      // Update last used timestamp for current token
      await client.query(`
        UPDATE auth_refresh_tokens 
        SET last_used_at = NOW()
        WHERE id = $1
      `, [storedToken.id]);
      
      // Check if token should be rotated (older than threshold)
      const tokenAge = Date.now() - new Date(storedToken.created_at).getTime();
      const shouldRotate = tokenAge > REFRESH_TOKEN_ROTATION_THRESHOLD;
      
      let newTokenPair: TokenPair;
      
      if (shouldRotate) {
        // Revoke current token
        await client.query(`
          UPDATE auth_refresh_tokens 
          SET revoked_at = NOW(), revoked_reason = 'rotated'
          WHERE id = $1
        `, [storedToken.id]);
        
        // Create new token pair with same family ID
        const sessionId = uuidv4();
        const newAccessToken = this.generateAccessToken(user, permissions, roles, sessionId);
        const newRefreshToken = this.generateRefreshToken(user.id, sessionId, storedToken.family_id);
        
        const accessTokenExpiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000);
        const refreshTokenExpiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);
        
        // Store new refresh token
        const newTokenHash = crypto.createHash('sha256').update(newRefreshToken).digest('hex');
        await client.query(`
          INSERT INTO auth_refresh_tokens (
            token_hash, user_id, family_id, device_info, ip_address, expires_at
          ) VALUES ($1, $2, $3, $4, $5, $6)
        `, [
          newTokenHash,
          user.id,
          storedToken.family_id,
          JSON.stringify({ ...storedToken.device_info, ...deviceInfo }),
          ipAddress || storedToken.ip_address,
          refreshTokenExpiresAt
        ]);
        
        newTokenPair = {
          accessToken: newAccessToken,
          refreshToken: newRefreshToken,
          accessTokenExpiresAt,
          refreshTokenExpiresAt,
        };
      } else {
        // Just generate new access token, keep same refresh token
        const newAccessToken = this.generateAccessToken(user, permissions, roles, tokenPayload.sessionId);
        const accessTokenExpiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000);
        
        newTokenPair = {
          accessToken: newAccessToken,
          refreshToken, // Keep the same refresh token
          accessTokenExpiresAt,
          refreshTokenExpiresAt: new Date(storedToken.expires_at),
        };
      }
      
      await client.query('COMMIT');
      return newTokenPair;
      
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  // Revoke refresh token
  async revokeRefreshToken(refreshToken: string, reason: string = 'user_logout'): Promise<void> {
    const tokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');
    
    await this.db.query(`
      UPDATE auth_refresh_tokens 
      SET revoked_at = NOW(), revoked_reason = $1
      WHERE token_hash = $2 AND revoked_at IS NULL
    `, [reason, tokenHash]);
  }

  // Revoke all user tokens
  async revokeAllUserTokens(userId: number, reason: string = 'logout_all'): Promise<void> {
    await this.db.query(`
      UPDATE auth_refresh_tokens 
      SET revoked_at = NOW(), revoked_reason = $1
      WHERE user_id = $2 AND revoked_at IS NULL
    `, [reason, userId]);
  }

  // Revoke token family (security measure)
  async revokeTokenFamily(familyId: string, reason: string = 'security_breach'): Promise<void> {
    await this.db.query(`
      UPDATE auth_refresh_tokens 
      SET revoked_at = NOW(), revoked_reason = $1
      WHERE family_id = $2 AND revoked_at IS NULL
    `, [reason, familyId]);
  }

  // Get user's active refresh tokens
  async getUserActiveTokens(userId: number): Promise<RefreshTokenInfo[]> {
    const result = await this.db.query(`
      SELECT * FROM auth_refresh_tokens
      WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
      ORDER BY last_used_at DESC
    `, [userId]);
    
    return result.rows;
  }

  // Cleanup expired tokens
  async cleanupExpiredTokens(): Promise<number> {
    const result = await this.db.query(`
      UPDATE auth_refresh_tokens 
      SET revoked_at = NOW(), revoked_reason = 'expired'
      WHERE expires_at < NOW() AND revoked_at IS NULL
    `);
    
    return result.rowCount || 0;
  }

  // Get token statistics
  async getTokenStatistics(): Promise<{
    totalActive: number;
    totalExpired: number;
    totalRevoked: number;
    activeByUser: Record<number, number>;
  }> {
    const [activeResult, expiredResult, revokedResult, byUserResult] = await Promise.all([
      this.db.query('SELECT COUNT(*) as count FROM auth_refresh_tokens WHERE revoked_at IS NULL AND expires_at > NOW()'),
      this.db.query('SELECT COUNT(*) as count FROM auth_refresh_tokens WHERE expires_at <= NOW()'),
      this.db.query('SELECT COUNT(*) as count FROM auth_refresh_tokens WHERE revoked_at IS NOT NULL'),
      this.db.query(`
        SELECT user_id, COUNT(*) as count 
        FROM auth_refresh_tokens 
        WHERE revoked_at IS NULL AND expires_at > NOW()
        GROUP BY user_id
      `)
    ]);
    
    const activeByUser: Record<number, number> = {};
    byUserResult.rows.forEach(row => {
      activeByUser[row.user_id] = parseInt(row.count);
    });
    
    return {
      totalActive: parseInt(activeResult.rows[0].count),
      totalExpired: parseInt(expiredResult.rows[0].count),
      totalRevoked: parseInt(revokedResult.rows[0].count),
      activeByUser,
    };
  }
}