import bcrypt from 'bcrypt';
import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';

// User model interfaces
export interface User {
  id: number;
  uuid: string;
  email: string;
  username?: string;
  password_hash: string;
  first_name?: string;
  last_name?: string;
  avatar_url?: string;
  is_active: boolean;
  is_verified: boolean;
  is_locked: boolean;
  failed_login_attempts: number;
  locked_until?: Date;
  password_changed_at: Date;
  password_reset_token?: string;
  password_reset_expires?: Date;
  email_verification_token?: string;
  email_verification_expires?: Date;
  two_factor_enabled: boolean;
  two_factor_secret?: string;
  backup_codes?: string[];
  created_at: Date;
  updated_at: Date;
  last_login_at?: Date;
  last_login_ip?: string;
  preferences: Record<string, any>;
}

export interface CreateUserData {
  email: string;
  password: string;
  username?: string;
  first_name?: string;
  last_name?: string;
  avatar_url?: string;
}

export interface UpdateUserData {
  username?: string;
  first_name?: string;
  last_name?: string;
  avatar_url?: string;
  preferences?: Record<string, any>;
}

export interface Role {
  id: number;
  name: string;
  display_name: string;
  description?: string;
  is_default: boolean;
  is_system: boolean;
  permissions: string[];
  created_at: Date;
  updated_at: Date;
}

// Password hashing configuration
const SALT_ROUNDS = 12;
const PASSWORD_MIN_LENGTH = 8;
const PASSWORD_REGEX = /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]/;

// Account lockout configuration
const MAX_FAILED_ATTEMPTS = 5;
const LOCKOUT_DURATION_MINUTES = 30;

export class UserModel {
  constructor(private db: Pool) {}

  // Password utilities
  static async hashPassword(password: string): Promise<string> {
    if (password.length < PASSWORD_MIN_LENGTH) {
      throw new Error(`Password must be at least ${PASSWORD_MIN_LENGTH} characters long`);
    }
    
    if (!PASSWORD_REGEX.test(password)) {
      throw new Error('Password must contain at least one uppercase letter, one lowercase letter, one number, and one special character');
    }
    
    return bcrypt.hash(password, SALT_ROUNDS);
  }

  static async verifyPassword(password: string, hash: string): Promise<boolean> {
    return bcrypt.compare(password, hash);
  }

  // User creation
  async createUser(userData: CreateUserData): Promise<User> {
    const client = await this.db.connect();
    
    try {
      await client.query('BEGIN');
      
      // Check if email already exists
      const existingEmail = await client.query(
        'SELECT id FROM auth_users WHERE email = $1',
        [userData.email.toLowerCase()]
      );
      
      if (existingEmail.rows.length > 0) {
        throw new Error('Email already registered');
      }
      
      // Check if username already exists (if provided)
      if (userData.username) {
        const existingUsername = await client.query(
          'SELECT id FROM auth_users WHERE username = $1',
          [userData.username]
        );
        
        if (existingUsername.rows.length > 0) {
          throw new Error('Username already taken');
        }
      }
      
      // Hash password
      const password_hash = await UserModel.hashPassword(userData.password);
      
      // Insert user
      const userResult = await client.query(`
        INSERT INTO auth_users (
          uuid, email, username, password_hash, first_name, last_name, avatar_url
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
      `, [
        uuidv4(),
        userData.email.toLowerCase(),
        userData.username || null,
        password_hash,
        userData.first_name || null,
        userData.last_name || null,
        userData.avatar_url || null
      ]);
      
      const user = userResult.rows[0];
      
      // Assign default role
      const defaultRole = await client.query(
        'SELECT id FROM auth_roles WHERE is_default = true LIMIT 1'
      );
      
      if (defaultRole.rows.length > 0) {
        await client.query(`
          INSERT INTO auth_user_roles (user_id, role_id)
          VALUES ($1, $2)
        `, [user.id, defaultRole.rows[0].id]);
      }
      
      await client.query('COMMIT');
      return user;
      
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  // User authentication
  async authenticateUser(email: string, password: string, ipAddress?: string): Promise<User | null> {
    const client = await this.db.connect();
    
    try {
      // Get user by email
      const userResult = await client.query(`
        SELECT * FROM auth_users 
        WHERE email = $1 AND is_active = true
      `, [email.toLowerCase()]);
      
      if (userResult.rows.length === 0) {
        return null;
      }
      
      const user = userResult.rows[0];
      
      // Check if account is locked
      if (user.is_locked) {
        if (user.locked_until && new Date(user.locked_until) > new Date()) {
          throw new Error('Account is temporarily locked due to too many failed login attempts');
        } else {
          // Unlock expired account
          await client.query(`
            UPDATE auth_users 
            SET is_locked = false, locked_until = NULL, failed_login_attempts = 0
            WHERE id = $1
          `, [user.id]);
          user.is_locked = false;
          user.failed_login_attempts = 0;
        }
      }
      
      // Verify password
      const isValidPassword = await UserModel.verifyPassword(password, user.password_hash);
      
      if (!isValidPassword) {
        // Increment failed attempts
        const newFailedAttempts = user.failed_login_attempts + 1;
        const shouldLock = newFailedAttempts >= MAX_FAILED_ATTEMPTS;
        
        const lockoutUntil = shouldLock 
          ? new Date(Date.now() + LOCKOUT_DURATION_MINUTES * 60 * 1000)
          : null;
        
        await client.query(`
          UPDATE auth_users 
          SET 
            failed_login_attempts = $1,
            is_locked = $2,
            locked_until = $3
          WHERE id = $4
        `, [newFailedAttempts, shouldLock, lockoutUntil, user.id]);
        
        if (shouldLock) {
          throw new Error(`Account locked due to ${MAX_FAILED_ATTEMPTS} failed login attempts. Try again in ${LOCKOUT_DURATION_MINUTES} minutes.`);
        }
        
        return null;
      }
      
      // Successful login - reset failed attempts and update last login
      await client.query(`
        UPDATE auth_users 
        SET 
          failed_login_attempts = 0,
          is_locked = false,
          locked_until = NULL,
          last_login_at = NOW(),
          last_login_ip = $1
        WHERE id = $2
      `, [ipAddress || null, user.id]);
      
      return { ...user, failed_login_attempts: 0, is_locked: false };
      
    } finally {
      client.release();
    }
  }

  // Get user by ID
  async getUserById(id: number): Promise<User | null> {
    const result = await this.db.query(`
      SELECT * FROM auth_users WHERE id = $1 AND is_active = true
    `, [id]);
    
    return result.rows[0] || null;
  }

  // Get user by UUID
  async getUserByUuid(uuid: string): Promise<User | null> {
    const result = await this.db.query(`
      SELECT * FROM auth_users WHERE uuid = $1 AND is_active = true
    `, [uuid]);
    
    return result.rows[0] || null;
  }

  // Get user by email
  async getUserByEmail(email: string): Promise<User | null> {
    const result = await this.db.query(`
      SELECT * FROM auth_users WHERE email = $1 AND is_active = true
    `, [email.toLowerCase()]);
    
    return result.rows[0] || null;
  }

  // Update user
  async updateUser(id: number, updates: UpdateUserData): Promise<User | null> {
    const updateFields: string[] = [];
    const values: any[] = [];
    let paramCount = 1;
    
    if (updates.username !== undefined) {
      // Check username uniqueness if provided
      if (updates.username) {
        const existing = await this.db.query(
          'SELECT id FROM auth_users WHERE username = $1 AND id != $2',
          [updates.username, id]
        );
        if (existing.rows.length > 0) {
          throw new Error('Username already taken');
        }
      }
      updateFields.push(`username = $${paramCount++}`);
      values.push(updates.username);
    }
    
    if (updates.first_name !== undefined) {
      updateFields.push(`first_name = $${paramCount++}`);
      values.push(updates.first_name);
    }
    
    if (updates.last_name !== undefined) {
      updateFields.push(`last_name = $${paramCount++}`);
      values.push(updates.last_name);
    }
    
    if (updates.avatar_url !== undefined) {
      updateFields.push(`avatar_url = $${paramCount++}`);
      values.push(updates.avatar_url);
    }
    
    if (updates.preferences !== undefined) {
      updateFields.push(`preferences = $${paramCount++}`);
      values.push(JSON.stringify(updates.preferences));
    }
    
    if (updateFields.length === 0) {
      return this.getUserById(id);
    }
    
    values.push(id);
    
    const result = await this.db.query(`
      UPDATE auth_users 
      SET ${updateFields.join(', ')}, updated_at = NOW()
      WHERE id = $${paramCount} AND is_active = true
      RETURNING *
    `, values);
    
    return result.rows[0] || null;
  }

  // Change password
  async changePassword(id: number, oldPassword: string, newPassword: string): Promise<boolean> {
    const user = await this.getUserById(id);
    if (!user) {
      throw new Error('User not found');
    }
    
    // Verify old password
    const isValidOldPassword = await UserModel.verifyPassword(oldPassword, user.password_hash);
    if (!isValidOldPassword) {
      throw new Error('Current password is incorrect');
    }
    
    // Hash new password
    const newPasswordHash = await UserModel.hashPassword(newPassword);
    
    // Update password
    await this.db.query(`
      UPDATE auth_users 
      SET password_hash = $1, password_changed_at = NOW(), updated_at = NOW()
      WHERE id = $2
    `, [newPasswordHash, id]);
    
    return true;
  }

  // Get user roles and permissions
  async getUserRolesAndPermissions(userId: number): Promise<{
    roles: Role[];
    permissions: string[];
  }> {
    const result = await this.db.query(`
      SELECT r.* 
      FROM auth_roles r
      JOIN auth_user_roles ur ON r.id = ur.role_id
      WHERE ur.user_id = $1 
        AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
    `, [userId]);
    
    const roles = result.rows;
    const permissionSet = new Set<string>();
    
    roles.forEach(role => {
      role.permissions.forEach((permission: string) => {
        permissionSet.add(permission);
      });
    });
    
    return {
      roles,
      permissions: Array.from(permissionSet)
    };
  }

  // Check if user has permission
  async userHasPermission(userId: number, permission: string): Promise<boolean> {
    const { permissions } = await this.getUserRolesAndPermissions(userId);
    return permissions.includes('*') || permissions.includes(permission);
  }

  // Assign role to user
  async assignRole(userId: number, roleId: number, assignedBy?: number, expiresAt?: Date): Promise<void> {
    await this.db.query(`
      INSERT INTO auth_user_roles (user_id, role_id, assigned_by, expires_at)
      VALUES ($1, $2, $3, $4)
      ON CONFLICT (user_id, role_id) DO NOTHING
    `, [userId, roleId, assignedBy || null, expiresAt || null]);
  }

  // Remove role from user
  async removeRole(userId: number, roleId: number): Promise<void> {
    await this.db.query(`
      DELETE FROM auth_user_roles
      WHERE user_id = $1 AND role_id = $2
    `, [userId, roleId]);
  }

  // Deactivate user
  async deactivateUser(id: number): Promise<void> {
    await this.db.query(`
      UPDATE auth_users 
      SET is_active = false, updated_at = NOW()
      WHERE id = $1
    `, [id]);
  }

  // Unlock user account
  async unlockUser(id: number): Promise<void> {
    await this.db.query(`
      UPDATE auth_users 
      SET 
        is_locked = false,
        locked_until = NULL,
        failed_login_attempts = 0,
        updated_at = NOW()
      WHERE id = $1
    `, [id]);
  }
}