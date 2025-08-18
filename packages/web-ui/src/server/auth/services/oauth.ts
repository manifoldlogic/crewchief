import passport from 'passport';
import { Strategy as GitHubStrategy } from 'passport-github2';
import { Strategy as GoogleStrategy } from 'passport-google-oauth20';
import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';
import crypto from 'crypto';
import type { User } from '../models/user.js';
import { UserModel } from '../models/user.js';

// OAuth provider configuration
export interface OAuthProvider {
  id: number;
  name: string;
  display_name: string;
  client_id: string;
  client_secret: string;
  authorization_url: string;
  token_url: string;
  user_info_url: string;
  default_scopes: string[];
  config: Record<string, any>;
  is_enabled: boolean;
}

export interface OAuthAccount {
  id: number;
  user_id: number;
  provider_id: number;
  provider_user_id: string;
  provider_username?: string;
  provider_email?: string;
  provider_name?: string;
  provider_avatar_url?: string;
  access_token?: string;
  refresh_token?: string;
  token_expires_at?: Date;
  token_scope?: string;
  raw_profile: Record<string, any>;
  created_at: Date;
  updated_at: Date;
  last_login_at?: Date;
}

export interface OAuthProfile {
  id: string;
  username?: string;
  displayName?: string;
  emails?: Array<{ value: string; verified?: boolean }>;
  photos?: Array<{ value: string }>;
  provider: string;
  _raw: string;
  _json: any;
}

export class OAuthService {
  private userModel: UserModel;

  constructor(private db: Pool) {
    this.userModel = new UserModel(db);
    this.initializePassportStrategies();
  }

  // Initialize Passport OAuth strategies
  private async initializePassportStrategies(): Promise<void> {
    try {
      const providers = await this.getEnabledProviders();
      
      for (const provider of providers) {
        switch (provider.name) {
          case 'github':
            this.setupGitHubStrategy(provider);
            break;
          case 'google':
            this.setupGoogleStrategy(provider);
            break;
          default:
            console.warn(`Unknown OAuth provider: ${provider.name}`);
        }
      }
    } catch (error) {
      console.error('Failed to initialize OAuth strategies:', error);
    }
  }

  // Setup GitHub OAuth strategy
  private setupGitHubStrategy(provider: OAuthProvider): void {
    if (!provider.client_id || provider.client_id === 'GITHUB_CLIENT_ID_PLACEHOLDER') {
      console.warn('GitHub OAuth not configured - skipping strategy setup');
      return;
    }

    passport.use('github', new GitHubStrategy({
      clientID: provider.client_id,
      clientSecret: provider.client_secret,
      callbackURL: `/auth/oauth/github/callback`,
      scope: provider.default_scopes,
    }, async (accessToken: string, refreshToken: string, profile: any, done: any) => {
      try {
        const result = await this.handleOAuthCallback(
          provider,
          profile,
          accessToken,
          refreshToken
        );
        done(null, result);
      } catch (error) {
        done(error, null);
      }
    }));
  }

  // Setup Google OAuth strategy
  private setupGoogleStrategy(provider: OAuthProvider): void {
    if (!provider.client_id || provider.client_id === 'GOOGLE_CLIENT_ID_PLACEHOLDER') {
      console.warn('Google OAuth not configured - skipping strategy setup');
      return;
    }

    passport.use('google', new GoogleStrategy({
      clientID: provider.client_id,
      clientSecret: provider.client_secret,
      callbackURL: `/auth/oauth/google/callback`,
      scope: provider.default_scopes,
    }, async (accessToken: string, refreshToken: string, profile: any, done: any) => {
      try {
        const result = await this.handleOAuthCallback(
          provider,
          profile,
          accessToken,
          refreshToken
        );
        done(null, result);
      } catch (error) {
        done(error, null);
      }
    }));
  }

  // Handle OAuth callback
  private async handleOAuthCallback(
    provider: OAuthProvider,
    profile: OAuthProfile,
    accessToken: string,
    refreshToken: string
  ): Promise<{ user: User; isNewUser: boolean; oauthAccount: OAuthAccount }> {
    const client = await this.db.connect();
    
    try {
      await client.query('BEGIN');

      // Extract profile information
      const email = profile.emails?.[0]?.value;
      const displayName = profile.displayName || profile.username;
      const avatarUrl = profile.photos?.[0]?.value;

      if (!email) {
        throw new Error('Email address is required for OAuth authentication');
      }

      // Check if OAuth account already exists
      const existingOAuthAccount = await client.query(`
        SELECT oa.*, u.* FROM auth_oauth_accounts oa
        JOIN auth_users u ON oa.user_id = u.id
        WHERE oa.provider_id = $1 AND oa.provider_user_id = $2
      `, [provider.id, profile.id]);

      let user: User;
      let isNewUser = false;
      let oauthAccount: OAuthAccount;

      if (existingOAuthAccount.rows.length > 0) {
        // Existing OAuth account - update tokens and profile
        user = existingOAuthAccount.rows[0];
        
        const updateResult = await client.query(`
          UPDATE auth_oauth_accounts 
          SET 
            provider_username = $1,
            provider_email = $2,
            provider_name = $3,
            provider_avatar_url = $4,
            access_token = $5,
            refresh_token = $6,
            token_expires_at = $7,
            raw_profile = $8,
            updated_at = NOW(),
            last_login_at = NOW()
          WHERE provider_id = $9 AND provider_user_id = $10
          RETURNING *
        `, [
          profile.username,
          email,
          displayName,
          avatarUrl,
          accessToken,
          refreshToken || null,
          null, // GitHub/Google don't provide expiration in basic flow
          JSON.stringify(profile._json),
          provider.id,
          profile.id
        ]);

        oauthAccount = updateResult.rows[0];

        // Update user's last login
        await client.query(`
          UPDATE auth_users 
          SET last_login_at = NOW()
          WHERE id = $1
        `, [user.id]);

      } else {
        // Check if user exists with this email
        const existingUser = await client.query(`
          SELECT * FROM auth_users WHERE email = $1 AND is_active = true
        `, [email.toLowerCase()]);

        if (existingUser.rows.length > 0) {
          // Link OAuth account to existing user
          user = existingUser.rows[0];
          
          const insertResult = await client.query(`
            INSERT INTO auth_oauth_accounts (
              user_id, provider_id, provider_user_id, provider_username,
              provider_email, provider_name, provider_avatar_url,
              access_token, refresh_token, raw_profile, last_login_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
            RETURNING *
          `, [
            user.id,
            provider.id,
            profile.id,
            profile.username,
            email,
            displayName,
            avatarUrl,
            accessToken,
            refreshToken || null,
            JSON.stringify(profile._json)
          ]);

          oauthAccount = insertResult.rows[0];

        } else {
          // Create new user with OAuth account
          isNewUser = true;
          
          // Generate a secure random password (user won't know it)
          const randomPassword = crypto.randomBytes(32).toString('hex');
          
          const userResult = await client.query(`
            INSERT INTO auth_users (
              uuid, email, password_hash, first_name, avatar_url, is_verified
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
          `, [
            uuidv4(),
            email.toLowerCase(),
            await UserModel.hashPassword(randomPassword),
            displayName || null,
            avatarUrl || null,
            true // OAuth users are automatically verified
          ]);

          user = userResult.rows[0];

          // Create OAuth account
          const oauthResult = await client.query(`
            INSERT INTO auth_oauth_accounts (
              user_id, provider_id, provider_user_id, provider_username,
              provider_email, provider_name, provider_avatar_url,
              access_token, refresh_token, raw_profile, last_login_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
            RETURNING *
          `, [
            user.id,
            provider.id,
            profile.id,
            profile.username,
            email,
            displayName,
            avatarUrl,
            accessToken,
            refreshToken || null,
            JSON.stringify(profile._json)
          ]);

          oauthAccount = oauthResult.rows[0];

          // Assign default role to new user
          const defaultRole = await client.query(
            'SELECT id FROM auth_roles WHERE is_default = true LIMIT 1'
          );

          if (defaultRole.rows.length > 0) {
            await client.query(`
              INSERT INTO auth_user_roles (user_id, role_id)
              VALUES ($1, $2)
            `, [user.id, defaultRole.rows[0].id]);
          }
        }
      }

      await client.query('COMMIT');
      return { user, isNewUser, oauthAccount };

    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  // Get enabled OAuth providers
  async getEnabledProviders(): Promise<OAuthProvider[]> {
    const result = await this.db.query(`
      SELECT * FROM auth_oauth_providers 
      WHERE is_enabled = true
      ORDER BY name
    `);
    
    return result.rows;
  }

  // Get provider by name
  async getProvider(name: string): Promise<OAuthProvider | null> {
    const result = await this.db.query(`
      SELECT * FROM auth_oauth_providers 
      WHERE name = $1 AND is_enabled = true
    `, [name]);
    
    return result.rows[0] || null;
  }

  // Get user's OAuth accounts
  async getUserOAuthAccounts(userId: number): Promise<OAuthAccount[]> {
    const result = await this.db.query(`
      SELECT oa.*, op.name as provider_name, op.display_name as provider_display_name
      FROM auth_oauth_accounts oa
      JOIN auth_oauth_providers op ON oa.provider_id = op.id
      WHERE oa.user_id = $1
      ORDER BY oa.created_at DESC
    `, [userId]);
    
    return result.rows;
  }

  // Unlink OAuth account
  async unlinkOAuthAccount(userId: number, providerId: number): Promise<boolean> {
    // Check if user has a password (so they can still log in)
    const user = await this.userModel.getUserById(userId);
    if (!user) {
      throw new Error('User not found');
    }

    // Check if user has other OAuth accounts or a password
    const oauthAccounts = await this.getUserOAuthAccounts(userId);
    const hasOtherAccounts = oauthAccounts.length > 1;
    const hasPassword = user.password_hash && user.password_hash.length > 0;

    if (!hasOtherAccounts && !hasPassword) {
      throw new Error('Cannot unlink the last authentication method. Please set a password first.');
    }

    const result = await this.db.query(`
      DELETE FROM auth_oauth_accounts 
      WHERE user_id = $1 AND provider_id = $2
    `, [userId, providerId]);

    return (result.rowCount || 0) > 0;
  }

  // Update OAuth provider configuration
  async updateProvider(providerId: number, updates: {
    client_id?: string;
    client_secret?: string;
    is_enabled?: boolean;
    default_scopes?: string[];
    config?: Record<string, any>;
  }): Promise<OAuthProvider | null> {
    const updateFields: string[] = [];
    const values: any[] = [];
    let paramCount = 1;

    if (updates.client_id !== undefined) {
      updateFields.push(`client_id = $${paramCount++}`);
      values.push(updates.client_id);
    }

    if (updates.client_secret !== undefined) {
      updateFields.push(`client_secret = $${paramCount++}`);
      values.push(updates.client_secret);
    }

    if (updates.is_enabled !== undefined) {
      updateFields.push(`is_enabled = $${paramCount++}`);
      values.push(updates.is_enabled);
    }

    if (updates.default_scopes !== undefined) {
      updateFields.push(`default_scopes = $${paramCount++}`);
      values.push(updates.default_scopes);
    }

    if (updates.config !== undefined) {
      updateFields.push(`config = $${paramCount++}`);
      values.push(JSON.stringify(updates.config));
    }

    if (updateFields.length === 0) {
      const result = await this.db.query(
        'SELECT * FROM auth_oauth_providers WHERE id = $1',
        [providerId]
      );
      return result.rows[0] || null;
    }

    values.push(providerId);

    const result = await this.db.query(`
      UPDATE auth_oauth_providers 
      SET ${updateFields.join(', ')}, updated_at = NOW()
      WHERE id = $${paramCount}
      RETURNING *
    `, values);

    // Reinitialize strategies if provider was updated
    if (result.rows.length > 0) {
      await this.initializePassportStrategies();
    }

    return result.rows[0] || null;
  }

  // Generate OAuth authorization URL
  async getAuthorizationUrl(providerName: string, state?: string): Promise<string> {
    const provider = await this.getProvider(providerName);
    if (!provider) {
      throw new Error(`OAuth provider '${providerName}' not found or disabled`);
    }

    const params = new URLSearchParams({
      client_id: provider.client_id,
      redirect_uri: `${process.env.BASE_URL || 'http://localhost:3456'}/auth/oauth/${providerName}/callback`,
      scope: provider.default_scopes.join(' '),
      response_type: 'code',
    });

    if (state) {
      params.append('state', state);
    }

    return `${provider.authorization_url}?${params.toString()}`;
  }

  // OAuth statistics
  async getOAuthStatistics(): Promise<{
    totalOAuthUsers: number;
    usersByProvider: Record<string, number>;
    recentLogins: Array<{
      provider_name: string;
      count: number;
      last_login: Date;
    }>;
  }> {
    const [totalResult, byProviderResult, recentResult] = await Promise.all([
      this.db.query('SELECT COUNT(DISTINCT user_id) as count FROM auth_oauth_accounts'),
      this.db.query(`
        SELECT op.name, COUNT(*) as count
        FROM auth_oauth_accounts oa
        JOIN auth_oauth_providers op ON oa.provider_id = op.id
        GROUP BY op.name
      `),
      this.db.query(`
        SELECT op.name as provider_name, COUNT(*) as count, MAX(oa.last_login_at) as last_login
        FROM auth_oauth_accounts oa
        JOIN auth_oauth_providers op ON oa.provider_id = op.id
        WHERE oa.last_login_at > NOW() - INTERVAL '30 days'
        GROUP BY op.name
        ORDER BY count DESC
      `)
    ]);

    const usersByProvider: Record<string, number> = {};
    byProviderResult.rows.forEach(row => {
      usersByProvider[row.name] = parseInt(row.count);
    });

    return {
      totalOAuthUsers: parseInt(totalResult.rows[0].count),
      usersByProvider,
      recentLogins: recentResult.rows.map(row => ({
        provider_name: row.provider_name,
        count: parseInt(row.count),
        last_login: new Date(row.last_login),
      })),
    };
  }
}